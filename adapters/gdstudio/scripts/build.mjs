import { readFile, writeFile, mkdir, stat, readdir } from 'node:fs/promises'
import { resolve, dirname } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { builtinModules } from 'node:module'
const project = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const root = resolve(project, '../..')
const { build } = await import(
  pathToFileURL(resolve(root, 'adapters/qqmusic/node_modules/esbuild/lib/main.js')).href
)
const { componentize } = await import(
  pathToFileURL(
    resolve(
      root,
      'packages/wasm-crypto/node_modules/@bytecodealliance/componentize-js/src/componentize.js'
    )
  ).href
)
const output = resolve(project, 'dist')
await mkdir(output, { recursive: true })
const result = await build({
  absWorkingDir: project,
  entryPoints: ['src/index.js'],
  outfile: resolve(output, 'guest.js'),
  bundle: true,
  platform: 'browser',
  format: 'esm',
  target: 'es2022',
  metafile: true,
  external: ['nons:host/transport@1.0.0', 'nons:plugin/host@1.0.0'],
  define: { 'process.env.DEBUG': '""' }
})
const builtins = new Set(builtinModules.flatMap((s) => [s, `node:${s}`]))
for (const input of Object.values(result.metafile.inputs))
  for (const item of input.imports)
    if (builtins.has(item.path)) throw new Error(`Node builtin in guest: ${item.path}`)
const pluginWit = await readFile(resolve(root, 'plugins/wit/plugin.wit'), 'utf8')
const hostWit = await readFile(resolve(root, 'packages/adapter-host/wit/host.wit'), 'utf8')
const hostInterface = hostWit.slice(
  hostWit.indexOf('interface transport'),
  hostWit.indexOf('\nworld host')
)
const witWorld =
  pluginWit.replace('  import host;', '  import host;\n  import nons:host/transport@1.0.0;') +
  `\npackage nons:host@1.0.0 {\n${hostInterface}\n}`
const js = await componentize({
  sourcePath: resolve(output, 'guest.js'),
  witWorld,
  worldName: 'plugin',
  disableFeatures: ['stdio', 'http', 'fetch-event'],
  env: {}
})
const destination = resolve(root, 'app/src-tauri/bundled-adapters/gdstudio')
await mkdir(destination, { recursive: true })
const component = resolve(output, 'adapter.component.wasm')
await writeFile(component, js.component)
if ((await stat(component)).size > 32 * 1024 * 1024)
  throw new Error('Audio provider exceeds component size limit')
await writeFile(resolve(destination, 'adapter.component.wasm'), js.component)
await writeFile(
  resolve(destination, 'manifest.json'),
  await readFile(resolve(project, 'manifest.json'))
)
const licenses = [
  'API service: GD音乐台 (music.gdstudio.xyz). Documentation: https://music-api.gdstudio.xyz/api.php . Service policy: CC BY-NC 4.0; study/reference only, no commercial use. No GD Studio server code is bundled.'
]
const modules = resolve(project, 'node_modules')
const dependencies = []
for (const item of await readdir(modules, { withFileTypes: true })) {
  if (!item.isDirectory() || item.name.startsWith('.')) continue
  if (item.name.startsWith('@'))
    for (const child of await readdir(resolve(modules, item.name)))
      dependencies.push(`${item.name}/${child}`)
  else dependencies.push(item.name)
}
for (const dependency of dependencies.sort()) {
  const directory = resolve(modules, dependency)
  const metadata = JSON.parse(await readFile(resolve(directory, 'package.json'), 'utf8'))
  const notices = (await readdir(directory)).filter((name) =>
    /^(?:licen[sc]e|copying|notice)(?:\.|$)/i.test(name)
  )
  licenses.push(`${dependency}@${metadata.version} (${metadata.license ?? 'see notice'})`)
  for (const name of notices) {
    if ((await stat(resolve(directory, name))).isFile())
      licenses.push(await readFile(resolve(directory, name), 'utf8'))
  }
}
await writeFile(resolve(destination, 'LICENSE.dependencies'), licenses.join('\n\n'))
console.log(`Built audio provider: ${component}`)
