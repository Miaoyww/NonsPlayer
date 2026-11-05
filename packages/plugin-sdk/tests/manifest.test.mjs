import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { test } from 'node:test'
import {
  PLUGIN_API_VERSION,
  SDK_VERSION,
  UI_API_VERSION,
  validateManifest
} from '../lib/manifest.mjs'

const template = JSON.parse(
  await readFile(new URL('../template/manifest.json', import.meta.url), 'utf8')
)

function check(change) {
  const manifest = structuredClone(template)
  change(manifest)
  return validateManifest(manifest)
}

test('SDK, plugin API and UI API versions stay aligned', () => {
  assert.equal(SDK_VERSION, '3.0.0')
  assert.equal(PLUGIN_API_VERSION, SDK_VERSION)
  assert.equal(UI_API_VERSION, SDK_VERSION)
})

test('template manifest passes validation', async () => {
  assert.deepEqual(await validateManifest(template), [])
})

test('rejects invalid ids, versions and engine ranges', async () => {
  assert.match((await check((m) => (m.id = 'Con'))).join(), /id/)
  assert.match((await check((m) => (m.id = '1plugin'))).join(), /id/)
  assert.match((await check((m) => (m.version = '1.0'))).join(), /SemVer/)
  assert.match((await check((m) => (m.engines.pluginApi = 'two'))).join(), /engines\.pluginApi/)
})

test('rejects incompatible plugin and UI API ranges', async () => {
  const errors = await check((m) => (m.engines.pluginApi = '^1.0.0'))
  assert.match(errors.join(), /不兼容/)
})

test('network capabilities need no legacy host or HTTP permission fields', async () => {
  assert.ok((await check((m) => (m.httpHosts = ['example.com']))).length)
  assert.ok((await check((m) => (m.permissions = ['ui', 'http:request']))).length)
  assert.deepEqual(await check((m) => (m.permissions = ['ui'])), [])
})

test('checks file roots, UI permissions and contribution references', async () => {
  assert.match((await check((m) => (m.fileRoots = ['*']))).join(), /files:selected/)
  assert.match((await check((m) => (m.contributes.pages = []))).join(), /navigation/)
  assert.match(
    (
      await check((m) => {
        m.contributes.pages.push({ id: 'home', path: '/duplicate', export: 'Other' })
      })
    ).join(),
    /重复/
  )
  assert.match(
    (
      await check((m) => {
        m.permissions = []
      })
    ).join(),
    /ui 权限/
  )
})

test('validates fullscreen player panel pages and action slots', async () => {
  assert.deepEqual(
    await check((m) => {
      m.contributes.views = [
        { id: 'details', slot: 'fullscreen.player.panel', label: '详情', export: 'Details' },
        { id: 'details-action', slot: 'fullscreen.player.actions', export: 'DetailsAction' }
      ]
    }),
    []
  )
  assert.match(
    (
      await check((m) => {
        m.contributes.views = [
          { id: 'details', slot: 'fullscreen.player.panel', export: 'Details' }
        ]
      })
    ).join(),
    /label/
  )
  assert.match(
    (
      await check((m) => {
        m.contributes.views = [{ id: 'details', slot: 'unknown', label: '详情', export: 'Details' }]
      })
    ).join(),
    /allowed values|未知 slot/
  )
})

test('rejects unsafe entry paths and accepts credential repositories', async () => {
  assert.match((await check((m) => (m.frontend = '../ui.mjs'))).join(), /frontend/)
  assert.match((await check((m) => (m.frontend = 'ui.js'))).join(), /frontend/)
  assert.deepEqual(await check((m) => (m.repository = 'https://user:pass@example.com/repo')), [])
  assert.deepEqual(await check((m) => (m.repository = 'https://example.com/repo')), [])
})
