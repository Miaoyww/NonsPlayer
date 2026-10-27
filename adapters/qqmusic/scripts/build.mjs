import { spawnSync } from 'node:child_process'
import { copyFile, mkdir, readFile, readdir, stat, writeFile } from 'node:fs/promises'
import { resolve, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const project = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const root = resolve(project, '../..')
const target = resolve(project, 'target')
const env = { ...process.env, CARGO_TARGET_DIR: target }
const result = spawnSync(
  'cargo',
  [
    'component',
    'build',
    '--manifest-path',
    resolve(project, 'Cargo.toml'),
    '--target',
    'wasm32-unknown-unknown',
    '--release',
    '--locked'
  ],
  { cwd: root, stdio: 'inherit', env }
)
if (result.error) throw result.error
if (result.status !== 0) process.exit(result.status ?? 1)

const wasm = resolve(target, 'wasm32-unknown-unknown/release/nons_adapter_qqmusic.wasm')
if ((await stat(wasm)).size > 32 * 1024 * 1024)
  throw new Error('Adapter exceeds component size limit')

// Resolve production WASM dependencies so distributed notices follow the lockfile.
const metadata = spawnSync(
  'cargo',
  [
    'metadata',
    '--manifest-path',
    resolve(project, 'Cargo.toml'),
    '--locked',
    '--format-version',
    '1',
    '--filter-platform',
    'wasm32-unknown-unknown'
  ],
  { cwd: root, encoding: 'utf8', env }
)
if (metadata.error) throw metadata.error
if (metadata.status !== 0) throw new Error(metadata.stderr)
const notices = []
for (const dependency of JSON.parse(metadata.stdout)
  .packages.filter((entry) => entry.source)
  .sort((a, b) => a.name.localeCompare(b.name))) {
  notices.push(
    `${dependency.name}@${dependency.version} (${dependency.license ?? 'see upstream'})\n${dependency.repository ?? ''}`
  )
  const directory = dirname(dependency.manifest_path)
  for (const name of (await readdir(directory)).filter((name) =>
    /^(license|copying|notice)([.-]|$)/i.test(name)
  )) {
    const path = resolve(directory, name)
    if ((await stat(path)).isFile()) notices.push(await readFile(path, 'utf8'))
  }
}

const output = resolve(project, 'dist')
const bundled = resolve(root, 'app/src-tauri/bundled-adapters/qqmusic')
await mkdir(output, { recursive: true })
await mkdir(bundled, { recursive: true })
await writeFile(resolve(output, 'LICENSE.dependencies'), notices.join('\n\n'))
await copyFile(wasm, resolve(output, 'adapter.component.wasm'))
for (const destination of [output, bundled]) {
  await copyFile(resolve(project, 'manifest.json'), resolve(destination, 'manifest.json'))
  await copyFile(
    resolve(project, 'LICENSE.qq-music-api'),
    resolve(destination, 'LICENSE.qq-music-api')
  )
}
await copyFile(resolve(output, 'LICENSE.dependencies'), resolve(bundled, 'LICENSE.dependencies'))
await copyFile(
  resolve(output, 'adapter.component.wasm'),
  resolve(bundled, 'adapter.component.wasm')
)
console.log(`Built Rust QQ adapter: ${resolve(output, 'adapter.component.wasm')}`)
