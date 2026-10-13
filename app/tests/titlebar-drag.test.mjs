import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
import ts from 'typescript'

function harness(playerMode = false) {
  const actions = []
  const jsx = (type, props) => ({ type, props })
  const modules = {
    react: { useState: (value) => [value, () => {}], useEffect() {} },
    'react/jsx-runtime': { jsx, jsxs: jsx },
    '@tauri-apps/api/core': { isTauri: () => true },
    '@/lib/platform': { isAndroid: () => false },
    '@tauri-apps/api/window': {
      getCurrentWindow: () => ({
        startDragging: async () => actions.push('drag'),
        toggleMaximize: async () => actions.push('maximize'),
        isMaximized: async () => false
      })
    },
    'lucide-react': {},
    '@/components/ui/button': { Button: 'button' },
    '@/features/settings/settings-dialog': { SettingsDialog: 'settings' },
    '@/features/account/account-menu': { AccountMenu: 'account' },
    '@/features/workspace/music-navigation': {
      useMusicNavigation: () => ({ page: { view: 'library' }, navigate() {} })
    },
    '@/features/search/music-search': { MusicSearch: 'search' },
    '@/features/library/playlist-sidebar': { PlaylistSidebar: 'sidebar' },
    '@/plugins/host': { usePlugins: () => ({ plugins: [], loaded: new Set() }) },
    '@/plugins/types': {}
  }
  const exports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(new URL('../src/features/workspace/titlebar.tsx', import.meta.url), 'utf8'),
      { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX } }
    ).outputText,
    { exports, require: (name) => modules[name] }
  )
  const header = exports.Titlebar({ playerMode }).props.children[0]
  const right = header.props.children.at(-1)
  return { actions, right }
}

test('blank space to the right of music navigation drags and double-clicks maximize', () => {
  const { actions, right } = harness()
  const children = [right.props.children].flat()
  const spacer = children.find((child) => child?.props?.className?.includes('flex-1'))
  assert.ok(spacer, 'right titlebar column needs a growing drag region before its controls')
  spacer.props.onMouseDown({ button: 0, detail: 1 })
  spacer.props.onMouseDown({ button: 0, detail: 2 })
  spacer.props.onMouseDown({ button: 2, detail: 1 })
  assert.deepEqual(actions, ['drag', 'maximize'])
  assert.equal(right.props.onMouseDown, undefined, 'control events must not bubble into dragging')
  assert.equal(children.at(-1).props.onMouseDown, undefined)
})

test('fullscreen window controls keep their existing compact layout', () => {
  const { right } = harness(true)
  assert.equal([right.props.children].flat().filter(Boolean).length, 1)
})
