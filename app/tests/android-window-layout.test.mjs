import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
import ts from 'typescript'

function harness(android = true) {
  const calls = [],
    effects = [],
    window = new EventTarget()
  let dark = false,
    observer,
    release
  const exports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(
        new URL('../src/features/workspace/use-android-window-layout.ts', import.meta.url),
        'utf8'
      ),
      {
        compilerOptions: { module: ts.ModuleKind.CommonJS }
      }
    ).outputText,
    {
      exports,
      window,
      console,
      Promise,
      document: { documentElement: { classList: { contains: () => dark } } },
      MutationObserver: class {
        constructor(callback) {
          this.callback = callback
          observer = this
        }
        observe() {}
        disconnect() {
          this.disconnected = true
        }
      },
      require: (name) =>
        ({
          react: { useEffect: (effect) => effects.push(effect) },
          '@/lib/platform': { isAndroid: () => android },
          '@tauri-apps/api/core': {
            isTauri: () => true,
            invoke: (command, args) => {
              calls.push({ command, ...args })
              return new Promise((resolve) => {
                release = resolve
              })
            }
          }
        })[name]
    }
  )
  return {
    calls,
    mount(immersive) {
      exports.useAndroidWindowLayout(immersive)
      return effects.pop()()
    },
    theme(value) {
      dark = value
      observer.callback()
    },
    focus() {
      window.dispatchEvent(new Event('focus'))
    },
    finish() {
      release?.()
    },
    disconnected: () => observer?.disconnected
  }
}
const settle = async () => {
  for (let i = 0; i < 12; i++) await Promise.resolve()
}

test('Android serializes entering, exiting and reopening while restoring bars on cleanup', async () => {
  const app = harness()
  const close = app.mount(true)
  await settle()
  close()
  const reopen = app.mount(true)
  await settle()
  assert.equal(app.calls.length, 1, 'one native request at a time')
  app.finish()
  await settle()
  assert.equal(app.calls[1].immersive, false)
  app.finish()
  await settle()
  assert.equal(app.calls[2].immersive, true)
  app.finish()
  await settle()
  app.theme(true)
  await settle()
  assert.equal(app.calls[3].dark, true)
  assert.equal(app.calls[3].immersive, true)
  app.finish()
  await settle()
  reopen()
  await settle()
  assert.equal(app.disconnected(), true)
  assert.equal(app.calls.at(-1).immersive, false)
  app.finish()
  await settle()
  const count = app.calls.length
  app.focus()
  await settle()
  assert.equal(app.calls.length, count)
})

test('desktop does not configure Android system bars', async () => {
  const app = harness(false)
  assert.equal(app.mount(true), undefined)
  await settle()
  assert.equal(app.calls.length, 0)
})
