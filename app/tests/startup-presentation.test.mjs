import { errorNoticeModule } from './helpers/error-notice.mjs'
import { sourceCapabilities } from './helpers/source-capabilities.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from 'node:vm'
import { test } from 'node:test'
import ts from 'typescript'
const jsx = (type, props) => ({ type, props })
function load(path, modules) {
  const exports = {}
  runInNewContext(
    ts.transpileModule(readFileSync(new URL(path, import.meta.url), 'utf8'), {
      compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
    }).outputText,
    {
      exports,
      require: (name) => (name === '@/components/error-notice' ? errorNoticeModule : modules[name])
    }
  )
  return exports
}
const react = {
  useState: (value) => [typeof value === 'function' ? value() : value, () => {}],
  useEffect() {},
  useCallback: (fn) => fn,
  useRef: (value) => ({ current: value }),
  lazy: () => 'LazyPage',
  Suspense: 'Suspense'
}
const base = {
  react,
  'react/jsx-runtime': { jsx, jsxs: jsx },
  '@tauri-apps/api/core': { isTauri: () => true },
  'lucide-react': {},
  '@/lib/platform': { isAndroid: () => false }
}
function flatten(node) {
  return Array.isArray(node)
    ? node.flatMap(flatten)
    : node && typeof node === 'object'
      ? [node, ...flatten(node.props?.children)]
      : []
}
test('unresolved startup source renders the actual library shell instead of the source selection empty page', () => {
  const modules = {
    '@/features/music/source-capabilities': sourceCapabilities,
    ...base,
    '@/features/workspace/music-navigation': {
      useMusicNavigation: () => ({ page: { view: 'library', query: '' } })
    },
    './use-primary-music-page': { usePrimaryMusicPage: () => true },
    './use-android-back': { useAndroidBack() {} },
    '@/features/settings/settings-sections': { settingsSections: [] },
    '@/features/music/use-music-sources': {
      useMusicSources: () => ({ statuses: [], loading: true, revision: 0, error: '' })
    },
    '@/features/music/source-label': { sourceLabel: String },
    '@/lib/player': { usePlayer: () => ({}) },
    '@/features/settings/use-interface-density': { useInterfaceDensity: () => ['standard'] },
    '@/features/playback/use-playback-bar-mode': { usePlaybackBarMode: () => ['off'] },
    '@/features/playback/shortcuts/use-playback-shortcuts': { usePlaybackShortcuts() {} },
    'motion/react': { AnimatePresence: 'AnimatePresence' },
    '@/features/library/collection-actions': { CollectionActionsProvider: 'Collections' },
    '@/components/music/song-actions': { SongActionsProvider: 'Songs' },
    '@/components/music/list-find': { ListFindProvider: 'Find' },
    '@/components/music/track-search-context': { TrackSearchContext: { Provider: 'Search' } },
    '@/components/music/loading': {
      LibraryPageSkeleton: 'LibraryPageSkeleton',
      MusicPageSkeleton: 'MusicPageSkeleton'
    },
    '@/components/music/action-button': { ActionButton: 'ActionButton' },
    '@/components/music/music-page': { MusicPage: 'MusicPage' },
    '@/features/lyrics/lyrics-view': { default: 'LyricsView' },
    '@/features/playback/music-options': {},
    '@/plugins/host': {},
    '@/features/playback/playback-bar': {},
    '@/features/playback/persistent-playback-bar': {},
    '@/features/queue/queue-page': {},
    '@/features/tools/tools-page': {}
  }
  const { MusicWorkspace } = load('../src/features/workspace/music-workspace.tsx', modules)
  const tree = MusicWorkspace({
    nowPlaying: false,
    playerVisible: false,
    onNowPlayingChange() {},
    onPlayerExitComplete() {}
  })
  assert.ok(flatten(tree).some((node) => node.type === 'LibraryPageSkeleton'))
  assert.equal(JSON.stringify(tree).includes('正在读取音乐来源'), false)
})
test('account menu shows recovery instead of declaring logout before account state arrives', () => {
  const { AccountMenu } = load('../src/features/account/account-menu.tsx', {
    ...base,
    '@base-ui/react/popover': {
      Popover: Object.fromEntries(
        ['Root', 'Trigger', 'Portal', 'Positioner', 'Popup', 'Title', 'Description'].map((key) => [
          key,
          key
        ])
      )
    },
    '@/features/workspace/music-navigation': { useMusicNavigation: () => ({ navigate() {} }) },
    '@/features/music/use-music-sources': { useMusicSources: () => ({ loading: true }) },
    './use-account-adapters': {
      useAccountAdapters: () => ({ adapters: [], loading: true, error: '', refresh() {} })
    },
    '@/components/music/action-button': { ActionButton: 'Button' },
    '@/features/music/client': {},
    '@/features/music/source-label': { sourceLabel: String },
    '@/lib/player': {},
    './account-avatar': { AccountAvatar: 'Avatar' },
    './saved-accounts': { SavedAccounts: 'SavedAccounts' }
  })
  const trigger = flatten(AccountMenu()).find((node) => node.type === 'Trigger')
  assert.equal(JSON.stringify(trigger).includes('未登录'), false)
  assert.ok(JSON.stringify(trigger).includes('正在恢复'))
})
