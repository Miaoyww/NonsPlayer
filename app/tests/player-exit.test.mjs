import { errorNoticeModule } from './helpers/error-notice.mjs'
import { sourceCapabilities } from './helpers/source-capabilities.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import ts from 'typescript'

const jsx = (type, props) => ({ type, props })
function load(path, modules, storage) {
  const source = readFileSync(new URL(path, import.meta.url), 'utf8')
  const { outputText } = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
  })
  const exports = {}
  runInNewContext(outputText, {
    exports,
    localStorage: storage,
    require(name) {
      if (name === '@/components/error-notice') return errorNoticeModule
      if (name === 'react/jsx-runtime') return { jsx, jsxs: jsx }
      assert.ok(name in modules, `unexpected dependency: ${name}`)
      return modules[name]
    }
  })
  return exports
}
function find(node, type) {
  if (!node || typeof node !== 'object') return
  if (node.type === type) return node
  for (const child of [node.props?.children].flat()) {
    const match = find(child, type)
    if (match) return match
  }
}
function harness(initialMode = 'collapsible', storage) {
  let phase
  let barMode = initialMode
  const App = load(
    '../src/App.tsx',
    {
      '@/features/updates/update-provider': { UpdateProvider: 'UpdateProvider' },
      '@/features/workspace/use-android-window-layout': { useAndroidWindowLayout() {} },
      '@/features/lyrics/android-media-lyrics': { AndroidMediaLyrics: 'AndroidMediaLyrics' },
      '@/lib/platform': { isAndroid: () => false },
      react: {
        useState(initial) {
          phase ??= typeof initial === 'function' ? initial() : initial
          return [
            phase,
            (next) => {
              phase = typeof next === 'function' ? next(phase) : next
            }
          ]
        },
        useEffect() {}
      },
      'motion/react': { MotionConfig: 'MotionConfig' },
      '@/features/workspace/titlebar': { Titlebar: 'Titlebar' },
      '@/features/workspace/music-workspace': { MusicWorkspace: 'MusicWorkspace' },
      '@/features/playback/music-options': { MusicOptionsProvider: 'MusicOptionsProvider' },
      '@/features/workspace/music-navigation': {
        MusicNavigationProvider: 'MusicNavigationProvider'
      },
      '@/features/account/account': { AccountProvider: 'AccountProvider' },
      '@/plugins/host': { PluginProvider: 'PluginProvider', PluginSlot: 'PluginSlot' }
    },
    storage
  ).default
  const MusicWorkspace = load('../src/features/workspace/music-workspace.tsx', {
    '@/features/music/source-capabilities': sourceCapabilities,
    '@/components/music/list-find': { ListFindProvider: 'ListFindProvider' },
    '@/features/music/use-music-sources': {
      useMusicSources: () => ({ preferences: { current: 'netease' }, statuses: [], revision: 0 })
    },
    '@/features/music/source-label': { sourceLabel: String },
    './use-android-back': { useAndroidBack() {} },
    './use-primary-music-page': { usePrimaryMusicPage: () => true },
    './mobile-page-header': { MobilePageHeader: 'MobilePageHeader' },
    '@/components/music/track-search-context': {
      TrackSearchContext: { Provider: 'TrackSearchContext' }
    },
    '@/features/settings/settings-sections': { settingsSections: [] },
    '@/lib/platform': { isAndroid: () => false },
    '@/components/music/loading': {
      MusicPageSkeleton: 'MusicPageSkeleton',
      LibraryPageSkeleton: 'LibraryPageSkeleton'
    },
    react: {
      useState: (value) => [value, () => {}],
      useRef: (current) => ({ current }),
      useCallback: (fn) => fn,
      useEffect() {},
      lazy: () => 'LazyView',
      Suspense: 'Suspense'
    },
    '@tauri-apps/api/core': { isTauri: () => false },
    '@tauri-apps/plugin-dialog': {},
    'lucide-react': {},
    '@tauri-apps/api/event': {},
    '@/features/local/folder-manager': { FolderManager: 'FolderManager' },
    '@/lib/runtime-cache': {},
    'motion/react': { AnimatePresence: 'AnimatePresence' },
    '@/features/playback/use-playback-bar-mode': { usePlaybackBarMode: () => [barMode, () => {}] },
    '@/features/settings/use-interface-density': {
      useInterfaceDensity: () => ['standard', () => {}]
    },
    '@/features/playback/persistent-playback-bar': {
      PersistentPlaybackBar: 'PersistentPlaybackBar'
    },
    '@/features/playback/shortcuts/use-playback-shortcuts': { usePlaybackShortcuts() {} },
    '@/features/lyrics/lyrics-view': { default: 'LyricsView' },
    '@/lib/player': { usePlayer: () => ({ index: null, queue: [] }) },
    '@/components/music/action-button': { ActionButton: 'ActionButton' },
    '@/components/music/music-page': { MusicPage: 'MusicPage' },
    '@/features/playback/playback-bar': { PlaybackBar: 'PlaybackBar' },
    '@/features/tools/tools-page': { ToolsPage: 'ToolsPage' },
    '@/plugins/host': { PluginPageHost: 'PluginPageHost' },
    '@/features/workspace/music-navigation': {
      useMusicNavigation: () => ({ page: { view: 'local', query: '' } })
    },
    '@/components/music/infinite-load': {},
    '@/lib/use-paged-list': { usePagedList: () => ({ items: [], more: false, busy: false }) },
    '@/features/playback/music-options': { QualitySelect: 'QualitySelect' },
    '@/components/music/track-list': {},
    '@/features/queue/queue-page': { QueuePage: 'QueuePage' },
    '@/features/library/collection-actions': { CollectionActionsProvider: 'collection-actions' },
    '@/components/music/song-actions': { SongActionsProvider: 'SongActionsProvider' }
  }).MusicWorkspace
  function render() {
    const props = find(App(), 'MusicWorkspace').props
    const workspace = MusicWorkspace(props)
    return {
      ...props,
      workspace,
      bar: find(workspace, 'PlaybackBar') ?? find(workspace, 'PersistentPlaybackBar'),
      exitComplete: find(workspace, 'AnimatePresence').props.onExitComplete
    }
  }
  return {
    render,
    setMode: (mode) => {
      barMode = mode
    }
  }
}

test('playback bar waits for the actual fullscreen exit completion', () => {
  const { render } = harness()
  assert.ok(render().bar)
  render().onNowPlayingChange(true)
  assert.equal(render().bar, undefined)
  render().onNowPlayingChange(false)
  const closing = render()
  assert.equal(closing.nowPlaying, false)
  assert.equal(closing.bar, undefined, 'exit animation still owns the screen')
  closing.exitComplete()
  assert.ok(render().bar, 'bar returns when Motion reports exit complete')
})

test('opening fullscreen cannot render the default search page while its module loads', () => {
  const { render } = harness()
  render().onNowPlayingChange(true)
  const opening = render()
  const boundary = find(opening.workspace, 'Suspense')
  assert.ok(
    boundary.props.fallback.type !== 'MusicPageSkeleton' ||
      boundary.props.fallback.props.view !== undefined,
    'the unspecified MusicPageSkeleton displays 搜索音乐 during fullscreen loading'
  )
})

test('a delayed exit callback cannot show the bar after reopening fullscreen', () => {
  const { render } = harness()
  render().onNowPlayingChange(true)
  render().onNowPlayingChange(false)
  const closing = render()
  closing.onNowPlayingChange(true)
  closing.exitComplete()
  assert.equal(render().nowPlaying, true)
  assert.equal(render().bar, undefined)
})

test('persistent and hidden modes respect fullscreen exit ownership and live settings', () => {
  const app = harness('persistent')
  assert.equal(app.render().bar.type, 'PersistentPlaybackBar')
  app.render().onNowPlayingChange(true)
  assert.equal(app.render().bar, undefined)
  app.render().onNowPlayingChange(false)
  const closing = app.render()
  assert.equal(closing.bar, undefined)
  app.setMode('off')
  closing.exitComplete()
  assert.equal(app.render().bar, undefined)
  app.setMode('collapsible')
  assert.equal(app.render().bar.type, 'PlaybackBar')
  app.setMode('persistent')
  assert.equal(app.render().bar.type, 'PersistentPlaybackBar')
})

test('startup keeps fullscreen closed even if an older version saved it as open', () => {
  let reads = 0
  const app = harness('collapsible', {
    getItem() {
      reads++
      return 'true'
    },
    setItem() {
      assert.fail('fullscreen location must not be persisted')
    }
  })
  assert.equal(app.render().nowPlaying, false)
  assert.equal(app.render().playerVisible, false)
  assert.equal(reads, 0)
  app.render().onNowPlayingChange(true)
  assert.equal(app.render().nowPlaying, true)
})
