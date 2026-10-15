import { readFile, writeFile, mkdir, stat } from 'node:fs/promises'
import { spawnSync } from 'node:child_process'
import { resolve, dirname } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { bundle } from './bundle.mjs'
const project = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const root = resolve(project, '../..')
const output = resolve(project, 'dist')
const { componentize } = await import(
  pathToFileURL(
    resolve(
      root,
      'packages/wasm-crypto/node_modules/@bytecodealliance/componentize-js/src/componentize.js'
    )
  ).href
)
function run(executable, args) {
  const result = spawnSync(executable, args, { cwd: root, stdio: 'inherit' })
  if (result.error) throw result.error
  if (result.status !== 0) throw new Error(`Build failed: ${executable}`)
}
const guest = await bundle(output)
run('cargo', [
  'build',
  '--manifest-path',
  resolve(root, 'packages/wasm-crypto/Cargo.toml'),
  '--locked',
  '--release',
  '--target',
  'wasm32-unknown-unknown'
])
const cryptoWit = await readFile(resolve(root, 'packages/wasm-crypto/wit/crypto.wit'), 'utf8')
const hostWit = await readFile(resolve(root, 'packages/adapter-host/wit/host.wit'), 'utf8')
const pluginWit = await readFile(resolve(root, 'plugins/wit/plugin.wit'), 'utf8')
const hostInterface = hostWit.slice(
  hostWit.indexOf('interface transport'),
  hostWit.indexOf('\nworld host')
)
const witWorld =
  pluginWit.replace(
    '  import host;',
    '  import host;\n  import nons:crypto/primitives@1.0.0;\n  import nons:host/transport@1.0.0;'
  ) +
  '\npackage nons:crypto@1.0.0 {\n' +
  cryptoWit.split('\n').slice(1).join('\n') +
  '\n}\npackage nons:host@1.0.0 {\n' +
  hostInterface +
  '\n}'
const js = await componentize({
  sourcePath: guest,
  witWorld,
  worldName: 'plugin',
  disableFeatures: ['stdio', 'http', 'fetch-event'],
  env: {}
})
await writeFile(resolve(output, 'js.component.wasm'), js.component)
const jco = resolve(root, 'packages/wasm-crypto/node_modules/@bytecodealliance/jco/dist/jco.js')
const cryptoComponent = resolve(output, 'crypto.component.wasm')
run(process.execPath, [
  jco,
  'new',
  resolve(root, 'packages/wasm-crypto/target/wasm32-unknown-unknown/release/nons_wasm_crypto.wasm'),
  '-o',
  cryptoComponent
])
const component = resolve(output, 'adapter.component.wasm')
run(
  process.env.NONS_WAC_BIN ??
    (process.platform === 'win32' ? resolve(root, '.local/tools/bin/wac.exe') : 'wac'),
  ['plug', resolve(output, 'js.component.wasm'), '--plug', cryptoComponent, '-o', component]
)
if ((await stat(component)).size > 32 * 1024 * 1024)
  throw new Error('Adapter exceeds approved temporary package limit')
const bundled = resolve(root, 'app/src-tauri/bundled-adapters/qqmusic')
await mkdir(bundled, { recursive: true })
for (const name of ['adapter.component.wasm', 'LICENSE.qq-music-api'])
  await writeFile(resolve(bundled, name), await readFile(resolve(output, name)))
await writeFile(
  resolve(bundled, 'manifest.json'),
  await readFile(resolve(project, 'manifest.json'))
)
console.log(`Built adapter: ${component}`)
