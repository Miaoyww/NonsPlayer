import { spawnSync } from 'node:child_process'
import { copyFile, mkdir, readFile, writeFile, stat } from 'node:fs/promises'
import { resolve, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
const project = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const root = resolve(project, '../..')
const target = resolve(project, 'target')
const build = spawnSync(
  'cargo',
  [
    'build',
    '--manifest-path',
    resolve(project, 'Cargo.toml'),
    '--target',
    'wasm32-wasip2',
    '--release',
    '--locked'
  ],
  { cwd: root, stdio: 'inherit', env: { ...process.env, CARGO_TARGET_DIR: target } }
)
if (build.error) throw build.error
if (build.status !== 0) process.exit(build.status ?? 1)
const wasm = resolve(target, 'wasm32-wasip2/release/nons_adapter_jbsou.wasm')
if ((await stat(wasm)).size > 32 * 1024 * 1024)
  throw new Error('Audio provider exceeds component size limit')
const dist = resolve(project, 'dist')
const destination = resolve(root, 'app/src-tauri/bundled-adapters/jbsou')
await mkdir(dist, { recursive: true })
await mkdir(destination, { recursive: true })
await copyFile(wasm, resolve(dist, 'adapter.component.wasm'))
await copyFile(wasm, resolve(destination, 'adapter.component.wasm'))
await copyFile(resolve(project, 'manifest.json'), resolve(destination, 'manifest.json'))
// Cargo metadata identifies only production dependencies of the WASM target.
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
    'wasm32-wasip2'
  ],
  { encoding: 'utf8', env: { ...process.env, CARGO_TARGET_DIR: target } }
)
if (metadata.error) throw metadata.error
if (metadata.status !== 0) throw new Error(metadata.stderr)
const notices = [await readFile(resolve(project, 'LICENSE.musicdl'), 'utf8')]
for (const pkg of JSON.parse(metadata.stdout)
  .packages.filter((pkg) => pkg.source)
  .sort((a, b) => a.name.localeCompare(b.name))) {
  notices.push(
    `${pkg.name}@${pkg.version} (${pkg.license ?? 'see upstream'})\n${pkg.repository ?? ''}`
  )
  const directory = dirname(pkg.manifest_path)
  const { readdir } = await import('node:fs/promises')
  for (const name of (await readdir(directory)).filter((name) =>
    /^(license|copying|notice)([.-]|$)/i.test(name)
  )) {
    if ((await stat(resolve(directory, name))).isFile())
      notices.push(await readFile(resolve(directory, name), 'utf8'))
  }
}
await writeFile(resolve(destination, 'LICENSE.dependencies'), notices.join('\n\n'))
console.log(`Built Rust audio provider: ${resolve(dist, 'adapter.component.wasm')}`)
