import assert from 'node:assert/strict'
import { mkdtemp, mkdir, rm, writeFile } from 'node:fs/promises'
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
