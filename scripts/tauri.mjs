import { createRequire } from 'node:module'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { spawn } from 'node:child_process'
import { stat } from 'node:fs/promises'

export function windowsBundlePlan(args, platform = process.platform) {
  const separator = args.indexOf('--')
  const options = separator < 0 ? args : args.slice(0, separator)
  if (
    platform !== 'win32' ||
    !['build', 'bundle'].includes(args[0]) ||
    options.some((value) =>
      ['-d', '--debug', '--no-bundle', '-h', '--help', '-V', '--version'].includes(value)
    )
  )
    return undefined
  const bundle = ['bundle']
  for (let index = 1; index < options.length; index++) {
    const value = options[index]
    if (['--ignore-version-mismatches'].includes(value)) continue
    if (['--runner', '-r'].includes(value)) {
      index++
      continue
    }
    if (value.startsWith('--runner=')) continue
    bundle.push(value)
  }
  const build =
    args[0] === 'build'
      ? [...options, '--no-bundle', ...(separator < 0 ? [] : args.slice(separator))]
      : undefined
  const targetIndex = options.findIndex((value) => ['--target', '-t'].includes(value))
  const target =
    targetIndex < 0
      ? options.find((value) => value.startsWith('--target='))?.slice('--target='.length)
      : options[targetIndex + 1]
  return { build, bundle, target }
}

export async function symbolConfig(targetDirectory, target, binaryName) {
  const source = join(targetDirectory, ...(target ? [target] : []), 'release', `${binaryName}.pdb`)
  const metadata = await stat(source).catch(() => undefined)
  if (!metadata?.isFile() || !metadata.size)
    throw new Error(`Missing Windows debug symbols: ${source}`)
  // This configuration is passed only after linking, never to a development build script.
  return { bundle: { resources: { [source]: `${binaryName}.pdb` } } }
}

export function cargoBinaryName(metadata, native) {
  const manifest = resolve(native, 'Cargo.toml')
  const current = metadata.packages.find((entry) => resolve(entry.manifest_path) === manifest)
  const binaries = current?.targets.filter((target) => target.kind.includes('bin')) ?? []
  const binary = current?.default_run
    ? binaries.find((target) => target.name === current.default_run)
    : binaries.length === 1
      ? binaries[0]
      : undefined
  if (!binary)
    throw new Error(`Cannot determine Cargo main binary for Windows symbols: ${manifest}`)
  // Tauri can rename the executable through mainBinaryName, but the PDB and its
  // embedded CodeView name are produced by Cargo's original binary target.
  return binary.name
}

async function run(command, args, options, capture = false) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      ...options,
      stdio: capture ? ['ignore', 'pipe', 'inherit'] : 'inherit'
    })
    let output = ''
    if (capture)
      child.stdout.setEncoding('utf8').on('data', (chunk) => {
        output += chunk
      })
    child.on('error', reject)
    child.on('exit', (code, signal) => {
      if (code === 0) resolve(output)
      else reject(new Error(`${command} exited with ${code ?? signal}`))
    })
  })
}

async function main() {
  const root = dirname(dirname(fileURLToPath(import.meta.url)))
  const app = join(root, 'app')
  const native = join(app, 'src-tauri')
  const env = { ...process.env }
  const args = process.argv.slice(2)
  const require = createRequire(join(app, 'package.json'))
  const cli = require.resolve('@tauri-apps/cli/tauri.js')
  const options = { cwd: app, env }
  const plan = windowsBundlePlan(args)
  if (!plan) return run(process.execPath, [cli, ...args], options)
  if (plan.build) await run(process.execPath, [cli, ...plan.build], options)
  const metadata = JSON.parse(
    await run(
      'cargo',
      ['metadata', '--no-deps', '--format-version', '1'],
      { cwd: native, env },
      true
    )
  )
  const symbols = await symbolConfig(
    metadata.target_directory,
    plan.target ?? env.CARGO_BUILD_TARGET,
    cargoBinaryName(metadata, native)
  )
  await run(process.execPath, [cli, ...plan.bundle, '--config', JSON.stringify(symbols)], options)
}

if (process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  main().catch((error) => {
    console.error(error.message)
    process.exitCode = 1
  })
}
