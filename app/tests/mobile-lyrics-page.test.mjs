import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from 'node:vm'
import { test } from 'node:test'
import ts from 'typescript'

function harness(android = true) {
  const slots = [],
    effects = [],
    exports = {},
    window = new EventTarget()
  let cursor = 0
  runInNewContext(
    ts.transpileModule(
      readFileSync(
        new URL('../src/features/lyrics/use-mobile-lyrics-page.ts', import.meta.url),
        'utf8'
      ),
      { compilerOptions: { module: ts.ModuleKind.CommonJS } }
    ).outputText,
    {
      exports,
      window,
      require: (name) =>
        name === '@/lib/platform'
          ? { isAndroid: () => android }
          : {
              useState(initial) {
                const index = cursor++
                if (!(index in slots)) slots[index] = initial
                return [
                  slots[index],
                  (value) => {
                    slots[index] = value
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
    }
  )
  return {
    render(present = true) {
      cursor = 0
      const state = exports.useMobileLyricsPage(present)
      effects.splice(0).forEach((fn) => fn())
      return state
    },
    back() {
      const event = new Event('nons-android-back', { cancelable: true })
      window.dispatchEvent(event)
      return event.defaultPrevented
    },
    close() {
      slots.forEach((slot) => slot?.cleanup?.())
    }
  }
}

test('Android starts at the cover, returns from lyrics before closing, and resets after player exit', () => {
  const app = harness()
  assert.equal(app.render().lyricsPage, false)
  assert.equal(app.back(), false)
  app.render().showLyricsPage()
  assert.equal(app.render().lyricsPage, true)
  assert.equal(app.back(), true)
  assert.equal(app.render().lyricsPage, false)
  app.render().showLyricsPage()
  app.render()
  app.render().showOverview()
  assert.equal(app.render().lyricsPage, false)
  app.render().showLyricsPage()
  app.render()
  app.render(false)
  assert.equal(app.render().lyricsPage, false)
  assert.equal(app.back(), false)
  app.close()
})

test('desktop leaves the existing combined player and back behavior in control', () => {
  const app = harness(false)
  assert.equal(app.render().mobile, false)
  app.render().showLyricsPage()
  app.render()
  assert.equal(app.back(), false)
  app.close()
})
