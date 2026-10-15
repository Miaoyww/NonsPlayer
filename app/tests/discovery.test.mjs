import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
import ts from 'typescript'

function harness(responses) {
  const requests = []
  const { outputText } = ts.transpileModule(
    readFileSync(new URL('../src/features/discovery/discovery-api.ts', import.meta.url), 'utf8'),
    { compilerOptions: { module: ts.ModuleKind.CommonJS } }
  )
  const exports = {}
  runInNewContext(outputText, {
    exports,
    require: (name) =>
      name === '@/features/music/entities'
        ? { musicCollection: (item) => ({ ...item, id: item.reference.id }) }
        : {
            queryPage: async (_query, offset) => {
              requests.push({ offset })
              const response = responses.shift()
              if (response instanceof Error) throw response
              return {
                type: 'entities',
                data: {
                  items: response.items.map((item) => ({
                    ...item,
                    reference: { source: 'qqmusic', kind: 'playlist', id: item.id }
                  })),
                  nextCursor: response.more ? 'next' : null
                }
              }
            }
          }
  })
  const create = () =>
    exports.createDiscoveryPlaylistLoader({
      section: 'recommended',
      category: '全部',
      order: 'hot',
      refresh: false
    })
  return { create, requests }
}
const page = (ids, more = true) => ({ items: ids.map((id) => ({ id })), more })
const ids = (value) => Array.from(value.items, (item) => item.id)

test('continuation deduplicates against short recommendations without mutating cached pages', async () => {
  const first = page([1, 2]),
    next = page([2, 3, 3, 4], false)
  const app = harness([first, next]),
    load = app.create()
  assert.deepEqual(ids(await load(0)), [1, 2])
  assert.deepEqual(ids(await load(30)), [3, 4])
  assert.deepEqual(
    app.requests.map((request) => request.offset),
    [0, 30]
  )
  assert.deepEqual(ids(next), [2, 3, 3, 4])
})

test('duplicate-only pages preserve continuation and failed pages can retry', async () => {
  const app = harness([page([1]), page([1]), new Error('offline'), page([1, 2], false)]),
    load = app.create()
  await load(0)
  const duplicate = await load(30)
  assert.deepEqual(ids(duplicate), [])
  assert.equal(duplicate.more, true)
  await assert.rejects(load(60), /offline/)
  const retry = await load(60)
  assert.deepEqual(ids(retry), [2])
  assert.equal(retry.more, false)
})

test('refresh and new queries reset displayed IDs', async () => {
  const app = harness([page([1]), page([1]), page([1])]),
    load = app.create()
  await load(0)
  assert.deepEqual(ids(await load(0)), [1])
  assert.deepEqual(ids(await app.create()(0)), [1])
})
