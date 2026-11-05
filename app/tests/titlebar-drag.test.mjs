import { errorNoticeModule } from './helpers/error-notice.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import ts from 'typescript'

function findDragRegion(node) {
  if (!node || typeof node !== 'object') return undefined
  if (node.props?.onMouseDown) return node
  for (const child of [node.props?.children].flat()) {
    const found = findDragRegion(child)
    if (found) return found
  }
  return undefined
}

function harness(playerMode = false, initialFullscreen = false) {
  const actions = []
  const listeners = new Map()
  const window = {
    addEventListener(type, listener) {
      listeners.set(type, [...(listeners.get(type) ?? []), listener])
    },
    removeEventListener(type, listener) {
      listeners.set(
        type,
        (listeners.get(type) ?? []).filter((candidate) => candidate !== listener)
      )
    }
  }
  const jsx = (type, props) => ({ type, props })
  let stateIndex = 0
  const modules = {
    react: {
      useState: (value) => [stateIndex++ === 1 ? initialFullscreen : value, () => {}],
      useEffect(callback) {
        callback()
      }
    },
    'react/jsx-runtime': { jsx, jsxs: jsx },
    '@tauri-apps/api/core': { isTauri: () => true },
    '@/lib/platform': { isAndroid: () => false },
    './use-primary-music-page': { usePrimaryMusicPage: () => true },
    '@tauri-apps/api/window': {
      getCurrentWindow: () => ({
        startDragging: async () => actions.push('drag'),
        toggleMaximize: async () => actions.push('maximize'),
        isMaximized: async () => false,
        isFullscreen: async () => false,
        setFullscreen: async (value) => actions.push(`fullscreen:${value}`),
        isFocused: async () => true,
        onResized: async () => () => {},
        onFocusChanged: async () => () => {}
      })
    },
    'lucide-react': {},
    '@/components/ui/button': { Button: 'button' },
    '@/features/settings/shell/settings-dialog': { SettingsDialog: 'settings' },
    '@/features/tools/tools-dialog': { ToolsDialog: 'tools' },
    '@/features/downloads/download-progress-button': { DownloadProgressButton: () => null },
    '@/features/account/account-menu': { AccountMenu: 'account' },
    '@/features/workspace/music-navigation': {
      useMusicNavigation: () => ({ page: { view: 'library' }, navigate() {} })
    },
    '@/features/search/music-search': { MusicSearch: 'search' },
    './music-nav-buttons': { MusicNavButtons: 'nav-buttons' },
    '@/plugins/host': { usePlugins: () => ({ plugins: [], loaded: new Set() }) },
    '@/plugins/types': {}
  }
  const exports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(new URL('../src/features/workspace/titlebar.tsx', import.meta.url), 'utf8'),
      { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX } }
    ).outputText,
    {
      exports,
      window,
      require: (name) => (name === '@/components/error-notice' ? errorNoticeModule : modules[name])
    }
  )
  const header = exports.Titlebar({ playerMode }).props.children[0]
  const right = header.props.children.at(-1)
  return { actions, header, listeners, right }
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

test('fullscreen window controls keep their existing compact layout', async () => {
  const { actions, right } = harness(true)
  const children = [right.props.children].flat().filter(Boolean)
  assert.equal(children.length, 1)
  const controls = [children[0].props.children].flat().filter(Boolean)
  const labels = controls.map((control) => control.props['aria-label'])
  assert.equal(labels.at(1), '进入全屏')
  assert.equal(labels.at(2), '最小化')
  controls.at(1).props.onClick()
  await new Promise((resolve) => setTimeout(resolve, 0))
  assert.deepEqual(actions, ['fullscreen:true'])
})

test('escape exits fullscreen before closing the player', async () => {
  const { actions, listeners } = harness(true, true)
  const event = {
    key: 'Escape',
    defaultPrevented: false,
    preventDefault() {
      this.defaultPrevented = true
    },
    stopPropagation() {}
  }
  listeners.get('keydown')[0](event)
  await new Promise((resolve) => setTimeout(resolve, 0))
  assert.equal(event.defaultPrevented, true)
  assert.deepEqual(actions, ['fullscreen:false'])
})

test('F toggles fullscreen in player mode unless a field or modifier handles it', async () => {
  const { actions, listeners } = harness(true)
  const keydown = listeners.get('keydown')[0]
  const press = (overrides = {}) => {
    const event = {
      code: 'KeyF',
      key: 'f',
      defaultPrevented: false,
      isComposing: false,
      preventDefault() {
        this.defaultPrevented = true
      },
      stopPropagation() {},
      ...overrides
    }
    keydown(event)
  }
  press({ ctrlKey: true })
  press({ target: { closest: () => ({}) } })
  assert.deepEqual(actions, [])
  press()
  await new Promise((resolve) => setTimeout(resolve, 0))
  assert.deepEqual(actions, ['fullscreen:true'])
})

test('dragging the player titlebar exits fullscreen first', async () => {
  const { actions, header } = harness(true, true)
  const dragRegion = findDragRegion(header)
  dragRegion.props.onMouseDown({ button: 0, detail: 1 })
  await new Promise((resolve) => setTimeout(resolve, 0))
  assert.deepEqual(actions, ['fullscreen:false', 'drag'])
})
