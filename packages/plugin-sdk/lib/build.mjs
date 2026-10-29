import { spawnSync } from 'node:child_process'
import { access, copyFile, cp, mkdir, readFile, rm } from 'node:fs/promises'
import { dirname, join, resolve, sep } from 'node:path'
import { buildFrontend } from './frontend.mjs'
import { formatErrors, validateManifest } from './manifest.mjs'
import { typecheck } from './typecheck.mjs'
import { safeRelative } from './util.mjs'
import { zipDirectory } from './zip.mjs'

async function exists(path) {
  return access(path).then(
    () => true,
    () => false
  )
}

export async function buildPlugin(options = {}) {
  const project = resolve(options.project ?? process.cwd())
  const manifestPath = join(project, 'manifest.json')
  const manifest = JSON.parse(await readFile(manifestPath, 'utf8'))
  const structural = await validateManifest(manifest)
  if (structural.length) throw new Error(`manifest.json 未通过校验：\n${formatErrors(structural)}`)

  const out = resolve(options.out ?? join(project, 'dist'))
  if (out === project || project.startsWith(out + sep))
    throw new Error('构建输出目录不能是插件根目录或其上级目录')
  await rm(out, { recursive: true, force: true })
  await mkdir(out, { recursive: true })

  const backend = manifest.backend ? await buildBackend(project, manifest.id) : null
  if (manifest.frontend) {
    if (options.typecheck !== false) {
      const result = typecheck(project)
      if (!result.ok) throw new Error(`TypeScript 检查失败：\n${result.text}`)
    }
    await buildFrontend(project, out, manifest.frontend)
  }
  if (backend) {
    const destination = join(out, manifest.backend)
    await mkdir(dirname(destination), { recursive: true })
    await copyFile(backend, destination)
  }
  await copyFile(manifestPath, join(out, 'manifest.json'))
  if (manifest.configuration) {
    if (!safeRelative(manifest.configuration)) throw new Error('配置声明路径无效')
    const destination = join(out, manifest.configuration)
    await mkdir(dirname(destination), { recursive: true })
    await copyFile(join(project, manifest.configuration), destination)
  }
  if (await exists(join(project, 'assets')))
    await cp(join(project, 'assets'), join(out, 'assets'), { recursive: true })

  const packaged = await validateManifest(manifest, { root: out })
  if (packaged.length) throw new Error(`构建产物未通过校验：\n${formatErrors(packaged)}`)

  const zip =
    options.zip === false
      ? null
      : resolve(options.zipPath ?? join(project, `${manifest.id}-${manifest.version}.zip`))
  if (zip) await zipDirectory(out, zip)
  return { out, zip }
}

async function buildBackend(project, id) {
  const backend = join(project, 'backend')
  const args = ['component', 'build', '--release', '--target', 'wasm32-unknown-unknown']
  if (await exists(join(backend, 'Cargo.lock'))) args.push('--locked')
  const result = spawnSync('cargo', args, {
    cwd: backend,
    env: { ...process.env, CARGO_TARGET_DIR: join(backend, 'target') },
    stdio: 'inherit'
  })
  if (result.error) throw result.error
  if (result.status !== 0) throw new Error(`WASM 后端构建失败，退出码 ${result.status}`)
  const wasm = join(
    backend,
    'target/wasm32-unknown-unknown/release',
    `${id.replaceAll('-', '_')}.wasm`
  )
  if (!(await exists(wasm)))
    throw new Error(`未找到 WASM 产物，请确认 Cargo 包名为 ${id.replaceAll('-', '_')}`)
  return wasm
}
