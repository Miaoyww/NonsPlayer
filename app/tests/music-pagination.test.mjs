import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from 'node:vm'
import { test } from 'node:test'
import ts from 'typescript'
function harness() {
  let scope = 'session-a',
    next = 'cursor-a'
  const calls = [],
    exports = {}
  const client = {
    query: async (source, request) => {
      calls.push({ source, request })
      return { type: 'entities', data: { items: [], nextCursor: next } }
    }
  }
  client.refresh = client.query
  const modules = {
    '@/lib/player': { nativeCall: async () => scope },
    './client': { musicClient: client },
    './source-preferences': { sourcePreferences: { get: async () => ({ current: 'qqmusic' }) } },
    './legacy-reference': {}
  }
  runInNewContext(
    ts.transpileModule(
      readFileSync(new URL('../src/features/music/queries.ts', import.meta.url), 'utf8'),
      { compilerOptions: { module: ts.ModuleKind.CommonJS } }
    ).outputText,
    { exports, require: (name) => modules[name] }
  )
  return {
    ...exports,
    calls,
    setScope: (value) => (scope = value),
    setNext: (value) => (next = value)
  }
}
const request = {
  operation: 'libraryCollections',
  kind: 'playlist',
  filter: 'all',
  page: { cursor: null, limit: 30 }
}
test('opaque cursors are isolated by source, request, size and session', async () => {
  const h = harness()
  await h.queryPage(request, 0, 30, 'qqmusic')
  h.setNext(null)
  await h.queryPage(request, 30, 30, 'qqmusic')
  assert.equal(h.calls[1].request.page.cursor, 'cursor-a')
  for (const args of [
    [request, 30, 25, 'qqmusic'],
    [request, 30, 30, 'netease'],
    [{ ...request, filter: 'created' }, 30, 30, 'qqmusic']
  ])
    await assert.rejects(h.queryPage(...args), /分页上下文/)
  h.setScope('session-b')
  await assert.rejects(h.queryPage(request, 30, 30, 'qqmusic'), /分页上下文/)
})
test('refresh retires old offsets and repeated cursors fail instead of looping', async () => {
  const h = harness()
  await h.queryPage(request, 0, 30)
  h.setNext('cursor-b')
  await h.queryPage(request, 30, 30)
  h.setNext('fresh')
  await h.queryPage(request, 0, 30, undefined, true)
  await assert.rejects(h.queryPage(request, 60, 30), /分页上下文/)
  await assert.rejects(h.queryPage(request, 30, 30), /重复游标/)
  for (const size of [0, -1, NaN, 1.5])
    await assert.rejects(h.queryPage(request, 0, size), /分页请求/)
})
