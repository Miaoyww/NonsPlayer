import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
import { JSDOM } from 'jsdom'
import ts from 'typescript'

function harness() {
  const dom = new JSDOM('<button id="card">歌单卡片</button>')
  const { window } = dom
  const slots = [],
    effects = []
  let cursor = 0
  const react = {
    createContext: () => ({ Provider: 'provider' }),
    useState(initial) {
      const index = cursor++
      if (!(index in slots)) slots[index] = typeof initial === 'function' ? initial() : initial
      return [
        slots[index],
        (value) => {
          slots[index] = typeof value === 'function' ? value(slots[index]) : value
        }
      ]
    },
    useEffect(fn, deps) {
      const index = cursor++,
        old = slots[index]
      if (!old || deps.some((value, i) => value !== old.deps[i])) {
        old?.cleanup?.()
        slots[index] = { deps }
        effects.push(() => {
          slots[index].cleanup = fn()
        })
      }
    }
  }
  const jsx = (type, props) => ({ type, props })
  const exports = {}
  const { outputText } = ts.transpileModule(
    readFileSync(
      new URL('../src/features/workspace/music-navigation.tsx', import.meta.url),
      'utf8'
    ),
    { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX } }
  )
  runInNewContext(outputText, {
    exports,
    window,
    document: window.document,
    require: (name) => (name === 'react' ? react : { jsx, jsxs: jsx })
  })
  function render() {
    cursor = 0
    const state = exports.MusicNavigationProvider({ children: null }).props.value
    effects.splice(0).forEach((fn) => fn())
    return state
  }
  function mouse(type, button) {
    const event = new window.MouseEvent(type, { button, bubbles: true, cancelable: true })
    window.document.getElementById('card').dispatchEvent(event)
    return event
  }
  function unmount() {
    slots.forEach((slot) => slot?.cleanup?.())
  }
  function close() {
    unmount()
    dom.window.close()
  }
  return { render, mouse, unmount, close }
}
test('opaque entity references distinguish cross-source history and remain intact when going back', () => {
  const app = harness()
  const first = {
    id: 0,
    kind: 'album',
    name: 'Same album',
    cover: '',
    subtitle: '',
    trackCount: 0,
    reference: { source: 'qqmusic', kind: 'album', id: 'album/雪:1' }
  }
  const second = { ...first, reference: { ...first.reference, source: 'other' } }
  app.render().navigate('album', '', first)
  app.render().navigate('album', '', second)
  assert.equal(app.render().page.collection.reference.source, 'other')
  app.render().back()
  assert.equal(app.render().page.collection.reference.source, 'qqmusic')
  assert.equal(app.render().page.collection.reference.id, 'album/雪:1')
  app.render().forward()
  assert.equal(app.render().page.collection.reference.source, 'other')
  app.close()
})
test('mouse back and forward navigate application history exactly once per click', () => {
  const app = harness()
  app.render().navigate('discover')
  app.render().navigate('local')
  app.render()
  assert.equal(app.mouse('mousedown', 3).defaultPrevented, true)
  app.mouse('mouseup', 3)
  app.mouse('auxclick', 3)
  assert.equal(app.render().page.view, 'discover')
  app.mouse('mousedown', 4)
  app.mouse('mouseup', 4)
  app.mouse('auxclick', 4)
  assert.equal(app.render().page.view, 'local')
  app.close()
})
test('side buttons stay within history bounds and do not change primary or context clicks', () => {
  const app = harness()
  app.render()
  for (const button of [3, 4]) {
    assert.equal(app.mouse('mousedown', button).defaultPrevented, true)
    assert.equal(app.render().page.view, 'library')
  }
  app.render().navigate('discover')
  app.render()
  for (const button of [0, 1, 2]) {
    assert.equal(app.mouse('mousedown', button).defaultPrevented, false)
    assert.equal(app.render().page.view, 'discover')
  }
  app.mouse('mousedown', 3)
  app.render().navigate('local')
  app.render()
  app.mouse('mousedown', 4)
  assert.equal(app.render().page.view, 'local')
  app.close()
})
test('navigation listeners are removed on provider unmount', () => {
  const app = harness()
  app.render().navigate('discover')
  app.render()
  app.unmount()
  assert.equal(app.mouse('mousedown', 3).defaultPrevented, false)
  assert.equal(app.render().page.view, 'discover')
  app.close()
})
