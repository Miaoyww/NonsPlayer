import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import ts from 'typescript'
import { JSDOM } from 'jsdom'

test('shared Slider forwards accessible values, keyboard commits and vertical volume semantics', async () => {
  const dom = new JSDOM('<div id="root"></div>', { pretendToBeVisual: true })
  const previous = new Map()
  for (const [name, value] of Object.entries({
    window: dom.window,
    document: dom.window.document,
    HTMLElement: dom.window.HTMLElement,
    Element: dom.window.Element,
    Node: dom.window.Node,
    IS_REACT_ACT_ENVIRONMENT: true
  })) {
    previous.set(name, Object.getOwnPropertyDescriptor(globalThis, name))
    Object.defineProperty(globalThis, name, { configurable: true, writable: true, value })
  }
  const React = await import('react')
  const { createRoot } = await import('react-dom/client')
  const modules = {
    react: React,
    'react/jsx-runtime': await import('react/jsx-runtime'),
    '@base-ui/react/slider': await import('@base-ui/react/slider'),
    cn: await import('cn')
  }
  const exports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(new URL('../src/components/ui/slider.tsx', import.meta.url), 'utf8'),
      {
        compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
      }
    ).outputText,
    { exports, require: (name) => modules[name] }
  )
  const root = createRoot(document.getElementById('root'))
  const commits = []
  function Controlled(props) {
    const [value, setValue] = React.useState(25)
    return React.createElement(exports.Slider, {
      value,
      onValueChange: setValue,
      onValueCommitted: (value) => commits.push(value),
      'aria-label': '播放进度',
      'aria-valuetext': '0:25 / 1:40',
      ...props
    })
  }
  try {
    await React.act(async () => root.render(React.createElement(Controlled)))
    let input = document.querySelector('input')
    assert.equal(input.getAttribute('aria-label'), '播放进度')
    assert.equal(input.getAttribute('aria-valuetext'), '0:25 / 1:40')
    assert.equal(input.value, '25')
    await React.act(async () =>
      input.dispatchEvent(
        new window.KeyboardEvent('keydown', {
          key: 'ArrowRight',
          bubbles: true,
          cancelable: true
        })
      )
    )
    assert.equal(input.value, '26')
    assert.deepEqual(commits, [26])
    await React.act(async () =>
      root.render(
        React.createElement(Controlled, {
          orientation: 'vertical',
          min: 0,
          max: 1,
          step: 0.01,
          disabled: true,
          'aria-label': '音量'
        })
      )
    )
    input = document.querySelector('input')
    assert.equal(input.getAttribute('aria-orientation'), 'vertical')
    assert.equal(input.getAttribute('aria-label'), '音量')
    assert.equal(input.disabled, true)
  } finally {
    await React.act(async () => root.unmount())
    dom.window.close()
    for (const [name, descriptor] of previous) {
      if (descriptor) Object.defineProperty(globalThis, name, descriptor)
      else delete globalThis[name]
    }
  }
})
