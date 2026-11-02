import { sourceCapabilities } from './helpers/source-capabilities.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import { test } from 'node:test'
import ts from 'typescript'

function harness({ statuses = [], current, page = { view: 'library' } } = {}) {
  const navigated = []
  const jsx = (type, props) => ({ type, props })
  const modules = {
    'react/jsx-runtime': { jsx, jsxs: jsx },
    '@/components/ui/button': { Button: 'button' },
    '@/features/music/source-capabilities': sourceCapabilities,
    '@/features/music/use-music-sources': {
      useMusicSources: () => ({ statuses, preferences: { current } })
    },
    './music-navigation': {
      useMusicNavigation: () => ({ page, navigate: (view) => navigated.push(view) })
    }
  }
  const exports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(
        new URL('../src/features/workspace/music-nav-buttons.tsx', import.meta.url),
        'utf8'
      ),
      { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX } }
    ).outputText,
    { exports, require: (name) => modules[name] }
  )
  return { buttons: exports.MusicNavButtons(), navigated }
}

const source = (capabilities) => ({
  descriptor: { source: 'netease', capabilities },
  enabled: true
})

test('disables pages the active adapter does not support and always keeps local available', () => {
  const { buttons } = harness({ statuses: [source(['userLibrary'])], current: 'netease' })
  const [library, discover, local] = buttons
  assert.equal(library.props.disabled, false)
  assert.equal(discover.props.disabled, true)
  assert.equal(discover.props.title, '此适配器未提供发现')
  assert.equal(local.props.disabled, false)
  assert.equal(local.props.title, undefined)
})

test('highlights the navigation entry that owns the current page', () => {
  const selected = (view) => {
    const { buttons } = harness({ statuses: [source([])], current: 'netease', page: { view } })
    return buttons
      .filter((button) => button.props['aria-current'] === 'page')
      .map((button) => button.props.children)
      .join(',')
  }
  assert.equal(selected('library'), '音乐库')
  assert.equal(selected('collection'), '音乐库')
  assert.equal(selected('accounts'), '音乐库')
  assert.equal(selected('search'), '发现')
  assert.equal(selected('local-album'), '本地')
  assert.equal(selected('settings'), '')
})

test('marks the selected entry with accent text instead of a filled background', () => {
  const { buttons } = harness({ statuses: [source([])], current: 'netease' })
  const [library, discover] = buttons
  assert.equal(library.props.variant, 'ghost')
  assert.match(library.props.className, /text-\[var\(--music-accent\)\]/)
  assert.equal(discover.props.variant, 'ghost')
  assert.equal(discover.props.className, undefined)
})

test('navigates to the matching music view', () => {
  const { buttons, navigated } = harness()
  buttons.forEach((button) => button.props.onClick())
  assert.deepEqual(navigated, ['library', 'discover', 'local'])
})
