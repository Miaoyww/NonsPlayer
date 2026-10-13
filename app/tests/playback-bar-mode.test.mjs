import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
import ts from 'typescript'

const source = readFileSync(
  new URL('../src/features/playback/use-playback-bar-mode.ts', import.meta.url),
  'utf8'
)
function session(saved, unavailable = false, android = false) {
  const exports = {},
    values = new Map(saved === undefined ? [] : [['nons-playback-bar-mode', saved]])
  const notifications = []
  runInNewContext(
    ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS } }).outputText,
    {
      exports,
      require: (name) =>
        name === '@/lib/platform'
          ? { isAndroid: () => android }
          : {
              useSyncExternalStore: (subscribe, get) => {
                subscribe(() => notifications.push(get()))
                return get()
              }
            },
      localStorage: {
        getItem: (key) => {
          if (unavailable) throw Error('unavailable')
          return values.get(key)
        },
        setItem: (key, value) => {
          if (unavailable) throw Error('unavailable')
          values.set(key, value)
        }
      }
    }
  )
  return { read: exports.usePlaybackBarMode, values, notifications }
}
test('playback bar defaults to collapsible and shares persisted mode immediately', () => {
  const app = session()
  const [initial, change] = app.read()
  assert.equal(initial, 'collapsible')
  change('persistent')
  assert.equal(app.read()[0], 'persistent')
  assert.equal(session(app.values.get('nons-playback-bar-mode')).read()[0], 'persistent')
  change('off')
  assert.equal(app.read()[0], 'off')
  assert.equal(session(app.values.get('nons-playback-bar-mode')).read()[0], 'off')
  assert.ok(app.notifications.includes('persistent'))
  assert.ok(app.notifications.includes('off'))
})

test('Android always exposes the persistent bar even with a saved off or collapsible mode', () => {
  for (const saved of [undefined, 'off', 'collapsible', 'persistent']) {
    const app = session(saved, false, true)
    assert.equal(app.read()[0], 'persistent')
    app.read()[1]('off')
    assert.equal(app.read()[0], 'persistent')
    assert.equal(app.values.get('nons-playback-bar-mode'), saved)
  }
})
test('invalid or unavailable storage preserves defaults and session changes', () => {
  for (const value of ['', 'unknown', 'null', 42])
    assert.equal(session(value).read()[0], 'collapsible')
  const app = session(undefined, true)
  const [, change] = app.read()
  change('persistent')
  assert.equal(app.read()[0], 'persistent')
  change('invalid')
  assert.equal(app.read()[0], 'persistent')
})
