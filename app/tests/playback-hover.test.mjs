import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import { test } from 'node:test'
import React, { act } from 'react'
import { createRoot } from 'react-dom/client'
import * as jsx from 'react/jsx-runtime'
import { JSDOM } from 'jsdom'
import ts from 'typescript'

test('left hover opens the player and moving to right controls holds it until leaving the whole card', async () => {
  const dom = new JSDOM('<div id="root"></div><button id="outside">Outside</button>')
  const previous = {
    window: globalThis.window,
    document: globalThis.document,
    act: globalThis.IS_REACT_ACT_ENVIRONMENT
  }
  Object.assign(globalThis, {
    window: dom.window,
    document: dom.window.document,
    IS_REACT_ACT_ENVIRONMENT: true
  })
  const modules = {
    react: React,
    'react/jsx-runtime': jsx,
    'lucide-react': Object.fromEntries(
      ['ChevronLeft', 'ChevronRight', 'Pause', 'Play', 'Repeat', 'Repeat1', 'Shuffle'].map(
        (name) => [name, () => null]
      )
    ),
    '@tauri-apps/api/core': { isTauri: () => true },
    '@/lib/player': {
      usePlayer: () => ({ queue: [], index: null, status: 'stopped', repeatMode: 'off' }),
      adjacentIndex: () => null,
      statusLabels: {}
    },
    '@/components/music/action-button': {
      ActionButton: ({ size, variant, ...props }) => React.createElement('button', props)
    },
    '@/components/music/track-title': { trackDisplayTitle: (track) => track.title },
    '@/features/playback/playback-notice': { PlaybackNotice: () => null },
    '@/features/playback/playback-timeline': { PlaybackTimeline: () => null },
    '@/features/playback/current-track-like': {
      CurrentTrackLike: () => React.createElement('button', { id: 'like' }, 'Like')
    },
    '@/features/queue/queue-popover': {
      QueuePopover: () => React.createElement('button', { id: 'queue' }, 'Queue')
    },
    '@/features/playback/volume-control': {
      VolumeControl: () =>
        React.createElement(
          'div',
          { className: 'volume-control' },
          React.createElement('button', { id: 'volume' }, 'Volume'),
          React.createElement('div', { className: 'volume-card' }, 'Volume slider')
        )
    }
  }
  const exports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(new URL('../src/features/playback/playback-bar.tsx', import.meta.url), 'utf8'),
      { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX } }
    ).outputText,
    {
      exports,
      require: (name) => {
        assert.ok(name in modules, name)
        return modules[name]
      }
    }
  )
  const css = readFileSync(new URL('../src/styles/playback.css', import.meta.url), 'utf8')
  const selector = css
    .match(/(\.playback-capsule:is\([\s\S]*?\))\s*\{\s*width: 648px/)[1]
    .replaceAll(':hover', '[data-test-hover]')
  const root = createRoot(document.getElementById('root'))
  try {
    await act(async () =>
      root.render(
        React.createElement(exports.PlaybackBar, {
          onLyrics() {},
          onQueue() {},
          onError() {}
        })
      )
    )
    const capsule = document.querySelector('.playback-capsule')
    const main = document.querySelector('.capsule-main')
    const outside = document.getElementById('outside')
    const expanded = () => capsule.matches(selector)
    async function move(from, to) {
      from?.removeAttribute('data-test-hover')
      to?.setAttribute('data-test-hover', '')
      await act(async () => {
        if (from)
          from.dispatchEvent(
            new dom.window.MouseEvent('mouseout', { bubbles: true, relatedTarget: to })
          )
        else
          to.dispatchEvent(
            new dom.window.MouseEvent('mouseover', { bubbles: true, relatedTarget: null })
          )
      })
    }
    assert.equal(expanded(), false)
    const like = document.getElementById('like')
    await move(null, like)
    assert.equal(expanded(), false, 'right controls must not initiate expansion')
    await move(like, main)
    assert.equal(expanded(), true, 'left region opens the card')
    await move(main, like)
    assert.equal(expanded(), true, 'moving from left to right must not retract the controls')
    let current = like
    for (const next of [
      document.getElementById('queue'),
      document.getElementById('volume'),
      document.querySelector('.volume-card'),
      capsule
    ]) {
      await move(current, next)
      assert.equal(expanded(), true, 'all card controls and padding retain expansion')
      current = next
    }
    await move(current, outside)
    assert.equal(expanded(), false, 'leaving the entire card collapses it')
    await move(outside, like)
    assert.equal(expanded(), false, 'reentering from the right remains collapsed')
  } finally {
    await act(async () => root.unmount())
    dom.window.close()
    Object.assign(globalThis, {
      window: previous.window,
      document: previous.document,
      IS_REACT_ACT_ENVIRONMENT: previous.act
    })
  }
})
