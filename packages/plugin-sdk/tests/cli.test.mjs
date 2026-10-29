import assert from 'node:assert/strict'
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { test } from 'node:test'
import { unzipSync } from 'fflate'
import { buildPlugin } from '../lib/build.mjs'
import { createPlugin } from '../lib/create.mjs'
import { validateManifest } from '../lib/manifest.mjs'

async function directory(t) {
  const root = await mkdtemp(join(tmpdir(), 'nons-plugin-sdk-'))
  t.after(() => rm(root, { recursive: true, force: true }))
  return root
}

test('create scaffolds a buildable frontend plugin', async (t) => {
  const root = await directory(t)
  const { target, id } = await createPlugin(join(root, 'demo-plugin'), { name: '演示插件' })
  assert.equal(id, 'demo-plugin')
  const manifest = JSON.parse(await readFile(join(target, 'manifest.json'), 'utf8'))
  assert.equal(manifest.id, 'demo-plugin')
  assert.equal(manifest.name, '演示插件')
  const pkg = JSON.parse(await readFile(join(target, 'package.json'), 'utf8'))
  assert.equal(pkg.name, 'nons-plugin-demo-plugin')

  const result = await buildPlugin({ project: target })
  assert.equal(result.out, join(target, 'dist'))
  const artifact = await readFile(join(result.out, 'ui.mjs'), 'utf8')
  assert.match(artifact, /\.\/_host\/sdk\.mjs/)
  assert.match(artifact, /\.\/_host\/react\.mjs/)
  assert.doesNotMatch(artifact, /from\s*["']@nonsplayer\/plugin-sdk["']/)
  assert.deepEqual(await validateManifest(manifest, { root: result.out }), [])

  const archive = unzipSync(new Uint8Array(await readFile(result.zip)))
  assert.ok(archive['manifest.json'])
  assert.ok(archive['ui.mjs'])
})

test('build rejects invalid manifests before writing artifacts', async (t) => {
  const root = await directory(t)
  const { target } = await createPlugin(join(root, 'demo-plugin'))
  const manifestPath = join(target, 'manifest.json')
  const manifest = JSON.parse(await readFile(manifestPath, 'utf8'))
  manifest.frontend = '../ui.mjs'
  await writeFile(manifestPath, JSON.stringify(manifest))
  await assert.rejects(buildPlugin({ project: target }), /frontend/)
})

test('validate reports missing packaged entries', async (t) => {
  const root = await directory(t)
  const { target } = await createPlugin(join(root, 'demo-plugin'))
  const manifest = JSON.parse(await readFile(join(target, 'manifest.json'), 'utf8'))
  const errors = await validateManifest(manifest, { root: target })
  assert.match(errors.join(), /文件不存在/)
})
