import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import { test } from 'node:test'
import ts from 'typescript'
const exports = {}
runInNewContext(
  ts.transpileModule(
    readFileSync(new URL('../src/features/music/source-store.ts', import.meta.url), 'utf8'),
    {
      compilerOptions: { module: ts.ModuleKind.CommonJS }
    }
  ).outputText,
  { exports }
)
const tick = () => new Promise((resolve) => setImmediate(resolve))
function harness() {
  const registrations = [],
    reads = [],
    stopped = []
  const store = exports.createSourceStore(
    () => new Promise((resolve, reject) => reads.push({ resolve, reject })),
    (event, changed) =>
      new Promise((resolve) =>
        registrations.push({ event, changed, resolve: () => resolve(() => stopped.push(event)) })
      ),
    String
  )
  return {
    store,
    registrations,
    reads,
    stopped,
    finishWatch: () => registrations.forEach((r) => r.resolve())
  }
}
test('several startup consumers share five parallel listeners and a single bootstrap read', async () => {
  const h = harness(),
    first = h.store.subscribe(() => {}),
    second = h.store.subscribe(() => {})
  assert.equal(h.registrations.length, 5)
  assert.equal(h.reads.length, 0)
  h.finishWatch()
  await tick()
  assert.equal(h.reads.length, 1)
  h.reads[0].resolve([{ current: 'test', supplementary: [], revision: 1 }, []])
  await tick()
  assert.equal(h.store.getSnapshot().loading, false)
  const snapshot = h.store.getSnapshot()
  assert.equal(h.store.getSnapshot(), snapshot)
  first()
  assert.equal(h.stopped.length, 0)
  second()
  assert.equal(h.stopped.length, 5)
})
test('changes during a bootstrap reject old publication and merge into one replacement', async () => {
  const h = harness(),
    stop = h.store.subscribe(() => {})
  h.finishWatch()
  await tick()
  h.registrations[0].changed()
  h.registrations[1].changed()
  h.reads[0].resolve([{ current: 'old', supplementary: [], revision: 1 }, []])
  await tick()
  assert.equal(h.store.getSnapshot().preferences, undefined)
  assert.equal(h.reads.length, 2)
  h.reads[1].resolve([{ current: 'new', supplementary: [], revision: 2 }, []])
  await tick()
  assert.equal(h.store.getSnapshot().preferences.current, 'new')
  stop()
})
test('unmounting cancels late reads and releases listeners that resolve later', async () => {
  const h = harness(),
    stop = h.store.subscribe(() => {})
  stop()
  h.finishWatch()
  await tick()
  assert.equal(h.stopped.length, 5)
  assert.equal(h.reads.length, 0)
  const again = h.store.subscribe(() => {})
  h.finishWatch()
  await tick()
  again()
  h.reads[0].resolve([{ current: 'late', supplementary: [], revision: 1 }, []])
  await tick()
  assert.equal(h.store.getSnapshot().preferences, undefined)
})

test('adapter loading failures are visible without a fabricated source entry', async () => {
  const h = harness()
  const stop = h.store.subscribe(() => {})
  h.finishWatch()
  await tick()
  h.reads[0].resolve([{ current: 'qqmusic', supplementary: [], revision: 1 }, []])
  await tick()
  h.registrations.find((r) => r.event === 'adapter-load-failed').changed('Component load failed')
  assert.equal(h.store.getSnapshot().error, 'Component load failed')
  assert.equal(h.store.getSnapshot().statuses.length, 0)
  stop()
})
