import { componentize } from '@bytecodealliance/componentize-js'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { spawnSync } from 'node:child_process'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const project = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const root = resolve(project, '../..')
const output = resolve(root, '.local/wasm-crypto-fixture')
await mkdir(output, { recursive: true })
function run(executable, args) {
  const result = spawnSync(executable, args, { cwd: root, stdio: 'inherit' })
  if (result.error) throw result.error
  if (result.status !== 0) throw new Error(`Build failed: ${executable}`)
}
run('cargo', [
  'build',
  '--manifest-path',
  resolve(project, 'Cargo.toml'),
  '--locked',
  '--release',
  '--target',
  'wasm32-unknown-unknown'
])
const cryptoWit = await readFile(resolve(project, 'wit/crypto.wit'), 'utf8')
const pluginWit = await readFile(resolve(root, 'plugins/wit/plugin.wit'), 'utf8')
const witWorld =
  pluginWit.replace('  import host;', '  import host;\n  import nons:crypto/primitives@1.0.0;') +
  '\npackage nons:crypto@1.0.0 {\n' +
  cryptoWit.split('\n').slice(1).join('\n') +
  '\n}'
const js = await componentize({
  sourcePath: resolve(project, 'tests/component-guest.js'),
  witWorld,
  worldName: 'plugin',
  disableFeatures: ['stdio', 'random', 'clocks', 'http', 'fetch-event'],
  env: {}
})
const socket = resolve(output, 'js.component.wasm')
const plug = resolve(output, 'crypto.component.wasm')
const component = resolve(output, 'composed.component.wasm')
await writeFile(socket, js.component)
const jco = resolve(project, 'node_modules/@bytecodealliance/jco/dist/jco.js')
run(process.execPath, [
  jco,
  'new',
  resolve(project, 'target/wasm32-unknown-unknown/release/nons_wasm_crypto.wasm'),
  '-o',
  plug
])
// WAC is a build tool, never a child service in the application runtime.
run(process.env.NONS_WAC_BIN ?? 'wac', ['plug', socket, '--plug', plug, '-o', component])
run(process.execPath, [jco, 'wit', component])
console.log(`Composed fixture: ${component} (${(await readFile(component)).length} bytes)`)
