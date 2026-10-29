import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { test } from 'node:test'
import { validateManifest } from '../lib/manifest.mjs'

const template = JSON.parse(
  await readFile(new URL('../template/manifest.json', import.meta.url), 'utf8')
)

function check(change) {
  const manifest = structuredClone(template)
  change(manifest)
  return validateManifest(manifest)
}

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

test('requires matching HTTP permissions and hosts', async () => {
  assert.match((await check((m) => (m.httpHosts = ['example.com']))).join(), /httpHosts/)
  assert.match(
    (
      await check((m) => {
        m.permissions.push('http:request')
        m.httpHosts = ['*.example.com']
      })
    ).join(),
    /无效域名/
  )
  assert.deepEqual(
    await check((m) => {
      m.permissions.push('http:request')
      m.httpHosts = ['example.com', '*']
    }),
    []
  )
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

test('rejects unsafe entry paths and repositories with credentials', async () => {
  assert.match((await check((m) => (m.frontend = '../ui.mjs'))).join(), /frontend/)
  assert.match((await check((m) => (m.frontend = 'ui.js'))).join(), /frontend/)
  assert.match(
    (await check((m) => (m.repository = 'https://user:pass@example.com/repo'))).join(),
    /repository/
  )
  assert.deepEqual(await check((m) => (m.repository = 'https://example.com/repo')), [])
})
