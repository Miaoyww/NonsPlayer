import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import { test } from 'node:test'
import ts from 'typescript'

function harness() {
  const state = { status: 'playing', visible: true, reduced: false }
  const jsx = (type, props) => ({ type, props })
  const modules = {
    react: { useRef: () => ({ current: null }) },
    'react/jsx-runtime': { jsx, jsxs: jsx },
    'motion/react': {
      useInView: () => state.visible,
      useReducedMotion: () => state.reduced
    },
    '@/lib/player': { usePlayer: () => state },
    '@/components/animate-ui/icons/audio-lines': { AudioLines: 'AudioLines' },
    '@/components/ui/spinner': { Spinner: 'Spinner' }
  }
  const exports = {}
  const { outputText } = ts.transpileModule(
    readFileSync(new URL('../src/components/music/playing-indicator.tsx', import.meta.url), 'utf8'),
    { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX } }
  )
  runInNewContext(outputText, { exports, require: (name) => modules[name] })
  return { state, render: () => exports.PlayingIndicator() }
}

test('the playback indicator animates only during actual playback and resumes after pause', () => {
  const h = harness()
  assert.equal(h.render().props.children.props.animate, true)
  assert.equal(h.render().props['aria-label'], '正在播放')
  for (const status of ['paused', 'error', 'stopped']) {
    h.state.status = status
    assert.equal(h.render().props.children.props.animate, false, status)
  }
  h.state.status = 'playing'
  assert.equal(h.render().props.children.props.animate, true)
})

test('loading and buffering expose immediate loading feedback instead of playback animation', () => {
  const h = harness()
  for (const status of ['loading', 'buffering']) {
    h.state.status = status
    assert.equal(h.render().props['aria-label'], '正在加载')
    assert.equal(h.render().props.children.type, 'Spinner')
    assert.equal(h.render().props.children.props.style.animationPlayState, 'running')
  }
})

test('loading icons stop animating offscreen and for reduced motion', () => {
  const h = harness()
  h.state.status = 'loading'
  h.state.visible = false
  assert.equal(h.render().props.children.props.style.animationPlayState, 'paused')
  h.state.visible = true
  h.state.reduced = true
  assert.equal(h.render().props.children.props.style.animationPlayState, 'paused')
})

test('offscreen and reduced-motion playback indicators stay static', () => {
  const h = harness()
  h.state.visible = false
  assert.equal(h.render().props.children.props.animate, false)
  h.state.visible = true
  h.state.reduced = true
  assert.equal(h.render().props.children.props.animate, false)
  h.state.reduced = false
  assert.equal(h.render().props.children.props.animate, true)
})
