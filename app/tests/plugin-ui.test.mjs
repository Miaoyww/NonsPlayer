import { coverImage } from './helpers/cover-image.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { mkdtemp, mkdir, writeFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { runInNewContext } from 'node:vm'
import { test } from 'node:test'
import * as React from 'react'
import * as jsx from 'react/jsx-runtime'
import { createRoot } from 'react-dom/client'
import { JSDOM } from 'jsdom'
import ts from 'typescript'
import { buildFrontend } from '../../scripts/plugin-build-ui.mjs'

function load(path, modules) {
  modules = { '@/features/music/cover-image': coverImage, ...modules }
  const exports = {}
  const { outputText } = ts.transpileModule(readFileSync(new URL(path, import.meta.url), 'utf8'), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
  })
  runInNewContext(outputText, {
    exports,
    require(name) {
      assert.ok(name in modules, name)
      return modules[name]
    }
  })
  return exports
}
const routing = load('../src/plugins/types.ts', {})
test('plugin bridge accepts a real ESM namespace and repeated initialization', () => {
  const { installBridge } = load('../src/plugins/bridge.ts', {})
  // React is a native ESM namespace here, with the same export restrictions as SDK.
  assert.throws(() => Object.freeze(React), TypeError)
  installBridge(React, jsx, React)
  assert.doesNotThrow(() => installBridge(React, jsx, React))
})
test('plugin page routing retains namespaces, selects longest prefixes and rejects escapes', () => {
  assert.equal(
    routing.pluginPath('sample', '/settings?tab=one'),
    '/plugins/sample/settings?tab=one'
  )
  for (const path of ['/../other', '//evil.test', '/%2e%2e/other', '\\other', 'https://evil.test'])
    assert.throws(() => routing.pluginPath('sample', path))
  assert.equal(
    routing.resolvePluginPath('/plugins/sample/settings/child?x=1').pathname,
    '/settings/child'
  )
  const manifest = {
    contributes: {
      pages: [
        { id: 'root', path: '/' },
        { id: 'settings', path: '/settings' }
      ]
    }
  }
  assert.equal(routing.resolvePluginPage(manifest, '/settings/child').id, 'settings')
  assert.equal(routing.resolvePluginPage(manifest, '/settings-other').id, 'root')
})

test('lazy plugin chunks resolve the shared host modules from their asset directory', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'nons-plugin-chunks-'))
  const project = join(directory, 'project'),
    out = join(directory, 'out')
  try {
    await mkdir(join(project, 'frontend'), { recursive: true })
    await writeFile(
      join(project, 'frontend/index.tsx'),
      'export const loadPage=()=>import("./page");'
    )
    await writeFile(
      join(project, 'frontend/page.tsx'),
      'import {createElement} from "react"; import {Button} from "@app/plugin-sdk"; export const Page=()=>createElement(Button);'
    )
    await buildFrontend(project, out, 'ui.mjs')
    await mkdir(join(out, '_host'))
    await writeFile(join(out, '_host/react.mjs'), 'export const createElement=(type)=>({type});')
    await writeFile(join(out, '_host/sdk.mjs'), 'export const Button="shared-host-button";')
    const entry = await import(pathToFileURL(join(out, 'ui.mjs')))
    assert.equal((await entry.loadPage()).Page().type, 'shared-host-button')
  } finally {
    await rm(directory, { recursive: true, force: true })
  }
})

test('real dynamic ui.mjs uses host React and scoped events, renders and cleans its timers', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'nons-plugin-ui-'))
  const dom = new JSDOM('<div id="root"></div>', {
    url: 'http://localhost/',
    pretendToBeVisual: true
  })
  const previous = {
    window: globalThis.window,
    document: globalThis.document,
    navigator: globalThis.navigator,
    act: globalThis.IS_REACT_ACT_ENVIRONMENT
  }
  globalThis.window = dom.window
  globalThis.document = dom.window.document
  Object.defineProperty(globalThis, 'navigator', {
    value: dom.window.navigator,
    configurable: true
  })
  globalThis.IS_REACT_ACT_ENVIRONMENT = true
  const pendingTimers = new Set()
  const originalSet = window.setTimeout.bind(window),
    originalClear = window.clearTimeout.bind(window)
  window.setTimeout = (...args) => {
    const timer = originalSet(...args)
    pendingTimers.add(timer)
    return timer
  }
  window.clearTimeout = (timer) => {
    pendingTimers.delete(timer)
    originalClear(timer)
  }
  const root = createRoot(document.getElementById('root'))
  try {
    const scopeModule = load('../src/plugins/scope.tsx', { react: React })
    const calls = []
    const navigations = []
    const likes = []
    const Button = ({ children, variant: _variant, size: _size, ...props }) =>
      React.createElement('button', props, children)
    const artistLinks = load('../src/components/music/music-links.tsx', {
      react: React,
      'react/jsx-runtime': jsx,
      '@/features/local/use-local-preferences': {},
      '@/lib/player': {},
      '@/features/workspace/music-navigation': {
        useMusicNavigation: () => ({ navigate: (...args) => navigations.push(args) })
      }
    })
    const likeButton = load('../src/features/playback/current-track-like.tsx', {
      'react/jsx-runtime': jsx,
      'lucide-react': { Heart: () => React.createElement('svg') },
      '@tauri-apps/api/core': { isTauri: () => true },
      '@/components/music/music-page': { MusicPageHeader: 'header' },
      '@/components/ui/button': { Button },
      '@/features/account/account': {
        useAccount: () => ({
          profile: {},
          likedIds: new Set(),
          likesReady: true,
          pendingLikes: new Set(),
          toggleLike: async (id) => {
            likes.push(id)
          }
        })
      }
    })
    const songComponents = load('../src/plugins/song-components.tsx', {
      'react/jsx-runtime': jsx,
      './scope': scopeModule,
      '@/components/music/music-links': artistLinks,
      '@/components/music/track-identity': {},
      '@/features/playback/current-track-like': likeButton
    })
    let finish
    const sdk = load('../src/plugins/sdk.ts', {
      '@tauri-apps/api/event': { listen: async () => () => {} },
      react: React,
      './song-components': songComponents,
      './collection-tabs': {},
      './scope': scopeModule,
      './types': routing,
      '@/lib/player': {
        nativeCall: (...args) => {
          if (args[1]?.operation === 'config.get')
            return Promise.resolve(
              JSON.stringify({
                values: { previewDuration: 8, pauseOnHover: true },
                revision: 0,
                diagnostics: {},
                pendingReload: false
              })
            )
          calls.push(args)
          return new Promise((resolve) => {
            finish = resolve
          })
        }
      },
      '@/features/settings/use-theme': {},
      '@/components/music/use-cover-source': { useCoverSource: () => undefined },
      '@/features/workspace/music-navigation': {},
      '@/components/music/music-page': { MusicPageHeader: 'header' },
      '@/components/ui/button': { Button },
      '@/components/ui/progress': {},
      '@/components/ui/input': {},
      '@/components/animate-ui/components/base/tabs': {},
      './downloaded-song-list': {},
      './icon': {}
    })
    globalThis.__NONS_PLUGIN_HOST__ = { react: React, jsx, sdk }
    await buildFrontend(
      fileURLToPath(new URL('../../plugins/netease-island', import.meta.url)),
      directory,
      'ui.mjs'
    )
    await mkdir(join(directory, '_host'))
    for (const [name, object, exports] of [
      ['react', 'react', 'useEffect,useRef,useState'],
      ['jsx-runtime', 'jsx', 'jsx,jsxs,Fragment'],
      [
        'sdk',
        'sdk',
        'Button,SongArtists,SongLikeButton,useSongPlayback,useCoverSource,usePluginEvent,usePluginConfig'
      ]
    ]) {
      await writeFile(
        join(directory, '_host', `${name}.mjs`),
        `export const {${exports}}=globalThis.__NONS_PLUGIN_HOST__.${object};`
      )
    }
    const { DynamicIsland } = await import(pathToFileURL(join(directory, 'ui.mjs')))
    const scope = {
      descriptor: {
        generation: 7,
        manifest: { id: 'sample', permissions: ['ui', 'storage', 'player:control', 'config'] }
      },
      active: true,
      events: new Map(),
      listeners: new Set()
    }
    let storage
    function StorageProbe() {
      storage = sdk.usePluginStorage()
      return null
    }
    await React.act(async () => {
      root.render(
        React.createElement(
          scopeModule.PluginScope.Provider,
          { value: scope },
          React.createElement(DynamicIsland),
          React.createElement(StorageProbe)
        )
      )
    })
    assert.equal(document.querySelector('[role=status]'), null)
    assert.equal(scope.listeners.size, 1)
    await React.act(async () => {
      scope.events.set('song-detected', {
        id: 1,
        title: '歌曲一',
        artist: '歌手甲',
        artists: [{ id: 123, name: '歌手甲' }],
        album: '专辑',
        cover: ''
      })
      scope.listeners.forEach((notify) => notify())
    })
    assert.match(document.body.textContent, /歌曲一/)
    assert.match(document.body.textContent, /立即播放/)
    assert.match(document.body.textContent, /下一首播放/)
    assert.match(document.body.textContent, /收藏/)
    await React.act(async () => {
      document.querySelector('.music-entity-link').click()
      document.querySelector('[aria-label="收藏歌曲"]').click()
    })
    assert.equal(navigations[0][0], 'artist')
    assert.equal(navigations[0][2].id, 123)
    assert.deepEqual(likes, [1])
    assert.equal(pendingTimers.size, 1)
    await React.act(async () => {
      document.querySelector('[aria-label="关闭歌曲预览"]').focus()
    })
    assert.equal(pendingTimers.size, 0, 'keyboard focus pauses dismissal')
    await React.act(async () => {
      document.querySelector('[aria-label="关闭歌曲预览"]').click()
    })
    assert.equal(document.querySelector('[role=status]'), null)
    await React.act(async () => {
      scope.events.set('song-detected', { id: 2, title: '歌曲二', artist: '歌手乙', cover: '' })
      scope.listeners.forEach((notify) => notify())
    })
    const next = [...document.querySelectorAll('button')].find(
      (b) => b.textContent === '下一首播放'
    )
    await React.act(async () => {
      next.click()
      next.click()
    })
    assert.equal(calls.length, 1, 'repeated clicks submit one queue operation')
    assert.equal(calls[0][1].operation, 'netease.play-song')
    assert.deepEqual(JSON.parse(calls[0][1].args), { id: 2, mode: 'next' })
    await React.act(async () => {
      finish('null')
    })
    assert.match(document.body.textContent, /已加入下一首播放/)
    await React.act(async () => {
      ;[...document.querySelectorAll('button')].find((b) => b.textContent === '立即播放').click()
    })
    assert.deepEqual(JSON.parse(calls[1][1].args), { id: 2, mode: 'now' })
    await React.act(async () => {
      finish('null')
    })
    assert.match(document.body.textContent, /已开始播放/)
    await React.act(async () => {
      document.querySelector('[aria-label="关闭歌曲预览"]').click()
    })
    const read = storage.get('key')
    assert.equal(calls[0][1].id, 'sample')
    assert.equal(calls[0][1].generation, 7)
    scope.active = false
    finish('"stale"')
    await assert.rejects(read, /禁用/)
    await assert.rejects(storage.get('key'), /禁用/)
    await React.act(async () => {
      root.unmount()
    })
    assert.equal(scope.listeners.size, 0)
    assert.equal(pendingTimers.size, 0)
  } finally {
    dom.window.close()
    globalThis.window = previous.window
    globalThis.document = previous.document
    Object.defineProperty(globalThis, 'navigator', {
      value: previous.navigator,
      configurable: true
    })
    globalThis.IS_REACT_ACT_ENVIRONMENT = previous.act
    delete globalThis.__NONS_PLUGIN_HOST__
    await rm(directory, { recursive: true, force: true })
  }
})
