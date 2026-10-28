import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import React from 'react'
import { renderToStaticMarkup } from 'react-dom/server'
import * as jsx from 'react/jsx-runtime'
import ts from 'typescript'

function harness(source = 'online') {
  const cache = new Map()
  const modules = {
    react: React,
    'react/jsx-runtime': jsx,
    '@tauri-apps/api/core': { isTauri: () => false },
    '@/lib/player': {
      usePlayer: () => ({
        actualQuality: 'lossless',
        index: 0,
        queue: [{ source: { kind: source } }]
      })
    },
    '@/components/error-notice': { ErrorNotice: () => null },
    '@/components/ui/select': Object.fromEntries(
      ['Select', 'SelectContent', 'SelectGroup', 'SelectItem', 'SelectTrigger', 'SelectValue'].map(
        (name) => [name, ({ children }) => React.createElement('div', null, children)]
      )
    )
  }
  function load(file, reload = false) {
    const url = new URL(file, import.meta.url)
    if (!reload && cache.has(url.href)) return cache.get(url.href)
    const exports = {}
    runInNewContext(
      ts.transpileModule(readFileSync(url, 'utf8'), {
        compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
      }).outputText,
      {
        exports,
        require(name) {
          if (name in modules) return modules[name]
          assert.ok(name.startsWith('.'), `unexpected dependency: ${name}`)
          return load(new URL(`${name}.ts`, url).href)
        }
      }
    )
    cache.set(url.href, exports)
    return exports
  }
  return () => load('../src/features/playback/music-options.tsx', true)
}

test('a reloaded quality control still reads the mounted provider context', () => {
  const reload = harness()
  const mounted = reload()
  const render = (consumer) =>
    renderToStaticMarkup(
      React.createElement(
        mounted.MusicOptionsProvider,
        null,
        React.createElement(consumer.QualitySelect)
      )
    )
  assert.match(render(mounted), /Hi-Res/)
  // Fast Refresh can keep the mounted provider while replacing a consumer's module.
  assert.match(render(reload()), /Hi-Res/)
})

test('a quality control outside the provider still reports the missing provider', () => {
  const options = harness()()
  assert.throws(
    () => renderToStaticMarkup(React.createElement(options.QualitySelect)),
    /MusicOptionsProvider is required/
  )
})

test('local playback replaces the quality selector with its source label', () => {
  const options = harness('local')()
  const markup = renderToStaticMarkup(
    React.createElement(
      options.MusicOptionsProvider,
      null,
      React.createElement(options.QualitySelect)
    )
  )
  assert.match(markup, /本地音乐/)
  assert.doesNotMatch(markup, /Hi-Res/)
})
