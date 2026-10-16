import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import ts from 'typescript'

function panel(props, reduced = false) {
  const jsx = (type, props) => ({ type, props })
  const exports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(
        new URL('../src/components/animate-ui/primitives/base/tabs.tsx', import.meta.url),
        'utf8'
      ),
      { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX } }
    ).outputText,
    {
      exports,
      require: (name) =>
        ({
          'react/jsx-runtime': { jsx, jsxs: jsx },
          '@base-ui/react/tabs': { Tabs: { Panel: 'panel' } },
          'motion/react': {
            motion: { div: 'motion.div' },
            AnimatePresence: 'presence',
            useReducedMotion: () => reduced
          },
          '@/lib/get-strict-context': { getStrictContext: () => [] }
        })[name] ?? {}
    }
  )
  return exports.TabsPanel(props).props.children.props.render.props
}

test('tab content fades without projecting old content bounds onto new filters and loading states', () => {
  for (const value of ['recommended', 'square', 'recommended']) {
    const rendered = panel({
      value,
      children: value === 'square' ? 'filters and skeletons' : 'playlists'
    })
    assert.equal(rendered.layout, false)
    assert.equal(rendered.animate.opacity, 1)
  }
})

test('tab panels allow explicit layout animation and respect reduced motion', () => {
  assert.equal(panel({ value: 'a', layout: 'size' }).layout, 'size')
  assert.equal(panel({ value: 'a' }, true).transition.duration, 0)
})
