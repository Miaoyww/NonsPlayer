import { buildFrontend } from './plugin-build-ui.mjs'
import { cp, mkdir, readFile, copyFile } from 'node:fs/promises'
import { resolve, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
import { spawnSync } from 'node:child_process'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const name = process.argv[2]
if (!name) throw new Error('Usage: node scripts/build-plugin.mjs <fixture-id> [output]')
if (!/^[a-z][a-z0-9-]{0,63}$/.test(name)) throw new Error('Invalid plugin ID')
const project = resolve(root, 'plugins', name)
const out = process.argv[3]
  ? resolve(root, process.argv[3])
  : resolve(root, 'app/src-tauri/bundled-plugins', name)
const manifest = JSON.parse(await readFile(resolve(project, 'manifest.json'), 'utf8'))
if (manifest.backend) {
  const result = spawnSync(
    'cargo',
    ['component', 'build', '--release', '--target', 'wasm32-unknown-unknown', '--locked'],
    {
      cwd: resolve(project, 'backend'),
      env: { ...process.env, CARGO_TARGET_DIR: resolve(project, 'backend/target') },
      stdio: 'inherit'
    }
  )
  if (result.status !== 0) process.exit(result.status ?? 1)
}
await mkdir(out, { recursive: true })
if (manifest.frontend) {
  const typecheck = spawnSync(
    process.execPath,
    [resolve(root, 'app/node_modules/typescript/bin/tsc'), '-p', resolve(project, 'tsconfig.json')],
    { stdio: 'inherit' }
  )
  if (typecheck.status !== 0) process.exit(typecheck.status ?? 1)
  await buildFrontend(project, out, manifest.frontend)
}
if (manifest.backend)
  await copyFile(
    resolve(
      project,
      'backend/target/wasm32-unknown-unknown/release',
      `${name.replaceAll('-', '_')}.wasm`
    ),
    resolve(out, manifest.backend)
  )
await copyFile(resolve(project, 'manifest.json'), resolve(out, 'manifest.json'))
if (manifest.configuration) {
  const relative = manifest.configuration
  if (
    !relative.endsWith('.json') ||
    relative.includes('\\') ||
    relative.split('/').some((part) => !part || part === '.' || part === '..') ||
    relative.includes(':')
  )
    throw new Error('Invalid configuration path')
  await mkdir(dirname(resolve(out, relative)), { recursive: true })
  await copyFile(resolve(project, relative), resolve(out, relative))
}
try {
  await cp(resolve(project, 'assets'), resolve(out, 'assets'), { recursive: true })
} catch (error) {
  if (error.code !== 'ENOENT') throw error
}
const packaged = spawnSync(
  'python',
  [resolve(root, 'scripts/package-plugin.py'), out, `${out}.zip`],
  { stdio: 'inherit' }
)
if (packaged.status !== 0) process.exit(packaged.status ?? 1)
console.log(`Plugin: ${out}`)
