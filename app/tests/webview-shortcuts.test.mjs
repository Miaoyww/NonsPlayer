import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
import { JSDOM } from 'jsdom'

function session(development = false) {
  const { window } = new JSDOM('<input>')
  const source = readFileSync(new URL('../src/main.tsx', import.meta.url), 'utf8')
    .split('ReactDOM.createRoot')[0]
    .replace(/^import .*$/gm, '')
    .replaceAll('import.meta.env.DEV', String(development))
  runInNewContext(source, {
    window,
    isAndroid: () => false,
    initializeFontSettings() {},
    installDiagnostics() {},
    installPluginBridge() {}
  })
  return window
}

test('release prevents browser shortcuts even inside inputs', () => {
  const window = session()
  for (const options of [
    { key: 'f', ctrlKey: true },
    { key: 'p', ctrlKey: true },
    { key: 'F12' },
    { key: 'F5' },
    { key: 'r', ctrlKey: true },
    { key: 'i', ctrlKey: true, shiftKey: true },
    { key: 'p', metaKey: true },
    { key: 'i', metaKey: true, altKey: true }
  ]) {
    const event = new window.KeyboardEvent('keydown', {
      ...options,
      bubbles: true,
      cancelable: true
    })
    window.document.querySelector('input').dispatchEvent(event)
    assert.equal(event.defaultPrevented, true, JSON.stringify(options))
  }
})

test('editing and playback shortcuts still reach application handlers', () => {
  const window = session()
  let received = 0
  window.addEventListener('keydown', () => received++)
  for (const key of ['c', 'v', 'x', 'a', 'z', ' ', 'ArrowRight']) {
    const event = new window.KeyboardEvent('keydown', {
      key,
      ctrlKey: key.length === 1 && key !== ' ',
      cancelable: true
    })
    window.dispatchEvent(event)
    assert.equal(event.defaultPrevented, false, key)
  }
  assert.equal(received, 7)
})

test('development retains browser debugging shortcuts', () => {
  const window = session(true)
  const event = new window.KeyboardEvent('keydown', { key: 'F12', cancelable: true })
  window.dispatchEvent(event)
  assert.equal(event.defaultPrevented, false)
})

test('release Ctrl+F reaches the application finder after suppressing browser find', () => {
  const window = session()
  let received = false
  window.document.addEventListener('keydown', () => {
    received = true
  })
  const event = new window.KeyboardEvent('keydown', {
    key: 'f',
    ctrlKey: true,
    bubbles: true,
    cancelable: true
  })
  window.document.querySelector('input').dispatchEvent(event)
  assert.equal(event.defaultPrevented, true)
  assert.equal(received, true)
})
