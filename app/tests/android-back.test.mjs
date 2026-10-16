import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import ts from 'typescript'

function harness(android = true, intercept = false) {
  const effects = [],
    refs = []
  let cursor = 0,
    callback,
    removed = 0,
    cleanup
  const exports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(
        new URL('../src/features/workspace/use-android-back.ts', import.meta.url),
        'utf8'
      ),
      {
        compilerOptions: { module: ts.ModuleKind.CommonJS }
      }
    ).outputText,
    {
      exports,
      console,
      Event,
      window: {
        dispatchEvent: (event) => {
          if (intercept) event.preventDefault()
          return !event.defaultPrevented
        }
      },
      require: (name) =>
        ({
          react: {
            useRef(value) {
              const i = cursor++
              return (refs[i] ??= { current: value })
            },
            useEffect(fn) {
              effects.push(fn)
            }
          },
          '@tauri-apps/api/core': { isTauri: () => true },
          '@tauri-apps/api/app': {
            onBackButtonPress: async (fn) => {
              callback = fn
              return {
                unregister: async () => {
                  removed++
                }
              }
            }
          },
          '@/lib/platform': { isAndroid: () => android }
        })[name]
    }
  )
  return {
    render(options) {
      cursor = 0
      exports.useAndroidBack(options)
    },
    async mount() {
      cleanup = effects.shift()()
      await Promise.resolve()
    },
    press() {
      callback?.({ canGoBack: false })
    },
    async close() {
      cleanup?.()
      await Promise.resolve()
    },
    get removed() {
      return removed
    }
  }
}

test('Android back uses workspace history and closes the player before navigating', async () => {
  const app = harness(),
    calls = []
  const options = {
    nowPlaying: false,
    back: () => calls.push('back'),
    closePlayer: () => calls.push('close')
  }
  app.render(options)
  await app.mount()
  app.press()
  app.render({ ...options, nowPlaying: true })
  app.press()
  app.render({ ...options, back: () => calls.push('latest') })
  app.press()
  assert.deepEqual(calls, ['back', 'close', 'latest'])
  await app.close()
  assert.equal(app.removed, 1)
})

test('desktop does not intercept the system back button', async () => {
  const app = harness(false),
    calls = []
  app.render({ nowPlaying: false, back: () => calls.push('back'), closePlayer() {} })
  await app.mount()
  app.press()
  assert.deepEqual(calls, [])
  await app.close()
  assert.equal(app.removed, 0)
})

test('an open menu can consume Android back before workspace navigation or player exit', async () => {
  const app = harness(true, true),
    calls = []
  app.render({
    nowPlaying: true,
    back: () => calls.push('back'),
    closePlayer: () => calls.push('close')
  })
  await app.mount()
  app.press()
  assert.deepEqual(calls, [])
  await app.close()
})

test('late Android registration is removed after unmount', async () => {
  const app = harness()
  app.render({ nowPlaying: false, back() {}, closePlayer() {} })
  const mounting = app.mount()
  await app.close()
  await mounting
  assert.equal(app.removed, 1)
})
