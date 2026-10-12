import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
import ts from 'typescript'

function harness(limit = 1000) {
  const exports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(new URL('../src/features/search/aggregate-query.ts', import.meta.url), 'utf8'),
      { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 } }
    ).outputText,
    { exports }
  )
  const requests = []
  const query = new exports.AggregateQuery(
    [
      { source: 'n', name: '网易云' },
      { source: 'q', name: 'QQ' }
    ],
    (source, cursor) =>
      new Promise((resolve, reject) => requests.push({ source, cursor, resolve, reject })),
    (item) => item.id,
    String,
    limit
  )
  return {
    query,
    requests,
    items: () => query.getSnapshot().items.map((value) => `${value.source}:${value.item.id}`),
    source: (value) => query.getSnapshot().sources.find((item) => item.source === value)
  }
}
const settle = async () => {
  await Promise.resolve()
  await Promise.resolve()
}
test('first source delivers immediately; later batches append and same IDs in different platforms stay distinct', async () => {
  const h = harness()
  h.query.loadVisible('all')
  h.query.loadVisible('all')
  assert.equal(h.requests.length, 2)
  h.requests[1].resolve({ items: [{ id: '1' }], nextCursor: 'q2' })
  await settle()
  assert.deepEqual(Array.from(h.items()), ['q:1'])
  h.requests[0].resolve({ items: [{ id: '1' }], nextCursor: 'n2' })
  await settle()
  assert.deepEqual(Array.from(h.items()), ['q:1', 'n:1'])
  h.query.loadVisible('q')
  assert.equal(h.requests[2].source, 'q')
  assert.equal(h.requests[2].cursor, 'q2')
  h.requests[2].resolve({ items: [{ id: '1' }, { id: '2' }], nextCursor: null })
  await settle()
  assert.deepEqual(Array.from(h.items()), ['q:1', 'n:1', 'q:2'])
  assert.equal(h.source('n').cursor, 'n2')
})
test('page failures retain results and cursor, require explicit retry and do not stop the other source', async () => {
  const h = harness()
  h.query.loadVisible('all')
  h.requests[0].resolve({ items: [{ id: '1' }], nextCursor: 'n2' })
  h.requests[1].reject(new Error('offline'))
  await settle()
  h.query.loadVisible('all')
  assert.equal(h.requests.length, 3)
  assert.equal(h.requests[2].source, 'n')
  h.requests[2].reject(new Error('page failed'))
  await settle()
  assert.equal(h.source('n').cursor, 'n2')
  assert.deepEqual(Array.from(h.items()), ['n:1'])
  void h.query.load('n', true)
  assert.equal(h.requests[3].cursor, 'n2')
  h.requests[3].resolve({ items: [{ id: '2' }], nextCursor: null })
  await settle()
  assert.deepEqual(Array.from(h.items()), ['n:1', 'n:2'])
  assert.match(h.source('q').error, /offline/)
})
test('disposed or restarted queries reject late completions and restore in-flight admission', async () => {
  const h = harness()
  h.query.loadVisible('all')
  h.query.dispose()
  h.query.start()
  h.query.loadVisible('all')
  h.requests[0].resolve({ items: [{ id: 'old' }], nextCursor: null })
  h.requests[1].reject(new Error('old error'))
  await settle()
  assert.equal(h.items().length, 0)
  assert.equal(h.source('q').busy, true)
  h.requests[2].resolve({ items: [{ id: 'new' }], nextCursor: null })
  await settle()
  assert.deepEqual(Array.from(h.items()), ['n:new'])
})
test('per-source capacity prevents unbounded pagination without erasing other sources', async () => {
  const h = harness(2)
  h.query.loadVisible('all')
  h.requests[0].resolve({ items: [{ id: '1' }, { id: '2' }, { id: '3' }], nextCursor: 'next' })
  await settle()
  assert.equal(h.items().length, 2)
  assert.equal(h.source('n').capped, true)
  assert.equal(h.source('n').more, false)
  h.query.loadVisible('n')
  assert.equal(h.requests.length, 2)
})
test('repeated cursors fail without publishing a partial page', async () => {
  const h = harness()
  void h.query.load('n')
  h.requests[0].resolve({ items: [{ id: '1' }], nextCursor: 'n2' })
  await settle()
  void h.query.load('n')
  h.requests[1].resolve({ items: [{ id: '2' }], nextCursor: 'n2' })
  await settle()
  assert.deepEqual(Array.from(h.items()), ['n:1'])
  assert.match(h.source('n').error, /重复游标/)
})
