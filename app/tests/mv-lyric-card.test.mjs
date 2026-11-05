import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import ts from 'typescript'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'

function harness() {
  const state = {
    position: 1_500,
    line: {
      words: [{ word: 'Hello' }],
      translatedLyric: ' 你好 ',
      romanLyric: ' ',
      startTime: 1_000,
      endTime: 2_000,
      isBG: false,
      isDuet: false
    },
    pronunciation: 'ni hao'
  }
  const jsx = (type, props) => ({ type, props })
  const exports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(new URL('../src/features/mv/mv-lyric-card.tsx', import.meta.url), 'utf8'),
      { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX } }
    ).outputText,
    {
      exports,
      require(name) {
        if (name === 'react') return { useMemo: (factory) => factory() }
        if (name === 'react/jsx-runtime') return { jsx, jsxs: jsx }
        if (name === '@animate-ui/react/primitives/texts/morphing')
          return { MorphingText: 'MorphingText' }
        if (name === 'motion/react')
          return { motion: { div: 'motion.div' }, useReducedMotion: () => false }
        if (name === '@/lib/player')
          return {
            useProgress: () => ({ status: 'paused', positionMs: state.position }),
            currentPosition: () => state.position
          }
        if (name === '@/features/desktop-lyrics/lyric-display')
          return {
            lyricText: (line) => line.words.map((word) => word.word).join(''),
            lyricPronunciation: () => state.pronunciation
          }
        assert.equal(name, './mv-lyric-line')
        return { findLyricLine: () => state.line }
      }
    }
  )
  const render = (showTranslation, showPronunciation) =>
    exports.MvLyricCard({ lines: [], showTranslation, showPronunciation })
  return { state, render }
}

function rows(card) {
  const children = card.props.children
  return (Array.isArray(children) ? Array.from(children) : [children]).filter(
    (row) => row && typeof row === 'object' && row.type === 'MorphingText'
  )
}

test('the MV card keeps the original line above enabled auxiliary rows', () => {
  const h = harness()
  const card = h.render(true, true).props.children
  assert.equal(card.props.children[0].props.layout, 'size')
  const rendered = rows(card)
  assert.deepEqual(
    rendered.map((row) => row.props.text),
    ['Hello', '你好', 'ni hao']
  )
  assert.deepEqual(
    rendered.map((row) => row.props.className),
    ['mv-lyric-original', 'mv-lyric-auxiliary', 'mv-lyric-auxiliary']
  )
})

test('the MV card follows each auxiliary visibility option', () => {
  const h = harness()
  assert.deepEqual(
    rows(h.render(true, false).props.children).map((row) => row.props.text),
    ['Hello', '你好']
  )
  assert.deepEqual(
    rows(h.render(false, true).props.children).map((row) => row.props.text),
    ['Hello', 'ni hao']
  )
  assert.deepEqual(
    rows(h.render(false, false).props.children).map((row) => row.props.text),
    ['Hello']
  )
})

test('the MV card renders nothing outside a lyric line', () => {
  const h = harness()
  h.state.line = null
  assert.equal(h.render(true, true), null)
})
