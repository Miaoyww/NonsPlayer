import assert from 'node:assert/strict'
import { mkdtemp, mkdir, readFile, readdir, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'
import { cargoBinaryName, symbolConfig, windowsBundlePlan } from '../../scripts/tauri.mjs'

test('Windows release links before adding symbols to installer resources', () => {
  const plan = windowsBundlePlan(
    ['build', '--ci', '--bundles', 'msi', 'nsis', '--', '--locked'],
    'win32'
  )
  assert.deepEqual(plan.build, [
    'build',
    '--ci',
    '--bundles',
    'msi',
    'nsis',
    '--no-bundle',
    '--',
    '--locked'
  ])
  assert.deepEqual(plan.bundle, ['bundle', '--ci', '--bundles', 'msi', 'nsis'])
  for (const args of [['dev'], ['build', '--debug'], ['build', '--no-bundle']]) {
    assert.equal(windowsBundlePlan(args, 'win32'), undefined)
  }
  assert.equal(windowsBundlePlan(['build'], 'linux'), undefined)
})

test('Windows bundle preserves CLI target and omits build-only arguments', () => {
  const plan = windowsBundlePlan(
    [
      'build',
      '--runner',
      'cargo',
      '--target',
      'aarch64-pc-windows-msvc',
      '--ignore-version-mismatches'
    ],
    'win32'
  )
  assert.equal(plan.target, 'aarch64-pc-windows-msvc')
  assert.deepEqual(plan.bundle, ['bundle', '--target', 'aarch64-pc-windows-msvc'])
  assert.deepEqual(windowsBundlePlan(['bundle', '--ci'], 'win32').bundle, ['bundle', '--ci'])
})

test('renamed Tauri executables keep Cargo PDB names without parsing CLI configuration', () => {
  const native = join(tmpdir(), 'nons-config-symbols', 'src-tauri')
  const metadata = {
    packages: [
      {
        manifest_path: join(native, 'Cargo.toml'),
        default_run: 'Nons',
        targets: [
          { name: 'nons_lib', kind: ['lib'] },
          { name: 'Nons', kind: ['bin'] }
        ]
      }
    ]
  }
  const args = [
    'build',
    '--config',
    '{"mainBinaryName":"Other"}',
    '--config',
    'flavor.json5',
    '--config',
    'flavor.toml'
  ]
  assert.deepEqual(windowsBundlePlan(args, 'win32').bundle, ['bundle', ...args.slice(1)])
  assert.equal(cargoBinaryName(metadata, native), 'Nons')
  delete metadata.packages[0].default_run
  assert.equal(cargoBinaryName(metadata, native), 'Nons')
  metadata.packages[0].targets.push({ name: 'Other', kind: ['bin'] })
  assert.throws(() => cargoBinaryName(metadata, native), /Cannot determine Cargo main binary/)
})

test('installer symbols use Cargo target directory and reject missing or empty PDBs', async (context) => {
  const root = await mkdtemp(join(tmpdir(), 'nons-symbols-'))
  context.after(() => rm(root, { recursive: true, force: true }))
  const triple = 'aarch64-pc-windows-msvc'
  const folder = join(root, triple, 'release')
  const symbols = join(folder, 'Nons.pdb')
  await assert.rejects(symbolConfig(root, triple, 'Nons'), /Missing Windows debug symbols/)
  await mkdir(folder, { recursive: true })
  await writeFile(symbols, '')
  await assert.rejects(symbolConfig(root, triple, 'Nons'), /Missing Windows debug symbols/)
  await writeFile(symbols, 'matching-link-fixture')
  assert.deepEqual(await symbolConfig(root, triple, 'Nons'), {
    bundle: { resources: { [symbols]: 'Nons.pdb' } }
  })
})

test('Windows installer resources retain both taskbar palettes when symbols are added', async (context) => {
  const root = await mkdtemp(join(tmpdir(), 'nons-taskbar-resources-'))
  context.after(() => rm(root, { recursive: true, force: true }))
  await mkdir(join(root, 'release'))
  await writeFile(join(root, 'release', 'Nons.pdb'), 'matching-link-fixture')
  const configs = await Promise.all(
    ['tauri.conf.json', 'tauri.windows.conf.json'].map(async (name) =>
      JSON.parse(await readFile(new URL(`../src-tauri/${name}`, import.meta.url), 'utf8'))
    )
  )
  configs.push(await symbolConfig(root, undefined, 'Nons'))
  let resources
  for (const config of configs) {
    const patch = config.bundle.resources
    resources = Array.isArray(patch)
      ? patch
      : { ...(Array.isArray(resources) ? {} : resources), ...patch }
  }
  const icons = await readdir(new URL('../src-tauri/icons/taskbar/', import.meta.url))
  assert.equal(icons.filter((name) => name.endsWith('.ico')).length, 8)
  for (const control of ['previous', 'next', 'play', 'pause']) {
    for (const color of ['white', 'black']) {
      const icon = `${control}-${color}.ico`
      assert.ok(icons.includes(icon), `Missing taskbar icon: ${icon}`)
      const bundled = Object.entries(resources).some(
        ([source, destination]) =>
          (source === 'icons/taskbar/' && destination === 'icons/taskbar/') ||
          (source === `icons/taskbar/${icon}` && destination === `icons/taskbar/${icon}`)
      )
      assert.ok(bundled, `icons/taskbar/${icon} was not found as a bundled resource`)
    }
  }
  assert.equal(resources[join(root, 'release', 'Nons.pdb')], 'Nons.pdb')
})
