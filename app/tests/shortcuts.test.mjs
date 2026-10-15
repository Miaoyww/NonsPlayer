import { musicReference } from './helpers/music-reference.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
import { JSDOM } from 'jsdom'
import ts from 'typescript'

function session(saved, android = false) {
  const dom = new JSDOM(
    '<main><input><button>Play</button><div contenteditable="true"><span></span></div></main>'
  )
  const registered = new Map(),
    values = new Map(saved ? [['nons-shortcut-settings', saved]] : [])
  const keys = loadKeys()
  const exports = {}
  const source = readFileSync(
    new URL('../src/features/playback/shortcuts/shortcuts.ts', import.meta.url),
    'utf8'
  )
  runInNewContext(
    ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS } }).outputText,
    {
      exports,
      Element: dom.window.Element,
      localStorage: {
        getItem: (key) => values.get(key) ?? null,
        setItem: (key, value) => values.set(key, value)
      },
      require: (name) =>
        name === '@/features/playback/shortcuts/shortcut-keys'
          ? keys
          : name === '@tauri-apps/api/core'
            ? { isTauri: () => true }
            : name === '@/lib/platform'
              ? { isAndroid: () => android }
              : name === '@/lib/player'
                ? { errorText: String, nativeCall: async () => {} }
                : {
                    async register(key, fn) {
                      if (key === 'Ctrl+Alt+N') throw Error('occupied')
                      if (registered.has(key)) throw Error('duplicate')
                      registered.set(key, fn)
                    },
                    async unregister(keys) {
                      for (const key of keys) registered.delete(key)
                    }
                  }
    }
  )
  return { ...exports, registered, values, document: dom.window.document, window: dom.window }
}
function config(app, bindings, enabled = true) {
  return { enabled, bindings: { ...app.getShortcutStatus().settings.bindings, ...bindings } }
}

test('Android does not restore desktop global shortcuts', async () => {
  const saved = JSON.stringify({ enabled: true, bindings: { toggle: 'Ctrl+Alt+P' } })
  const app = session(saved, true)
  await app.initializeShortcuts()
  assert.equal(app.registered.size, 0)
  await app.saveShortcuts(config(app, { toggle: 'Ctrl+Alt+P' }))
  assert.equal(app.registered.size, 0)
})

test('global shortcuts default to disabled and blank, restore on startup and unregister when disabled', async () => {
  const app = session()
  assert.equal(app.getShortcutStatus().settings.enabled, false)
  assert.ok(Object.values(app.getShortcutStatus().settings.bindings).every((value) => value === ''))
  await app.initializeShortcuts()
  assert.equal(app.registered.size, 0)
  await app.saveShortcuts(config(app, { toggle: 'Ctrl+Alt+P', next: '' }))
  const restored = session(app.values.get('nons-shortcut-settings'))
  await restored.initializeShortcuts()
  assert.deepEqual([...restored.registered.keys()], ['Ctrl+Alt+P'])
  const actions = []
  restored.setShortcutDispatcher((action) => actions.push(action))
  restored.registered.get('Ctrl+Alt+P')({ state: 'Released' })
  restored.registered.get('Ctrl+Alt+P')({ state: 'Pressed' })
  assert.deepEqual(actions, ['toggle'])
  await restored.saveShortcuts(config(restored, {}, false))
  assert.equal(restored.registered.size, 0)
})

test('registration failure removes partial bindings and restores previous shortcuts without saving', async () => {
  const app = session()
  await app.saveShortcuts(config(app, { toggle: 'Ctrl+Alt+P' }))
  const saved = app.values.get('nons-shortcut-settings')
  assert.equal(
    await app.saveShortcuts(config(app, { toggle: 'Ctrl+Alt+Q', next: 'Ctrl+Alt+N' })),
    false
  )
  assert.deepEqual([...app.registered.keys()], ['Ctrl+Alt+P'])
  assert.equal(app.values.get('nons-shortcut-settings'), saved)
  assert.match(app.getShortcutStatus().error, /occupied/)
  assert.equal(await app.saveShortcuts(config(app, { next: 'ctrl+alt+p' })), false)
  assert.deepEqual([...app.registered.keys()], ['Ctrl+Alt+P'])
})

test('space excludes editing, modifiers and composition; focused buttons allow playback', async () => {
  const app = session()
  const event = { code: 'Space', target: app.document.querySelector('main') }
  assert.equal(app.isPlaybackSpace(event), true)
  for (const selector of ['input', 'span'])
    assert.equal(
      app.isPlaybackSpace({ ...event, target: app.document.querySelector(selector) }),
      false
    )
  for (const key of ['isComposing', 'defaultPrevented', 'ctrlKey', 'altKey', 'metaKey', 'shiftKey'])
    assert.equal(app.isPlaybackSpace({ ...event, [key]: true }), false)
  assert.equal(
    app.isPlaybackSpace({ ...event, target: app.document.querySelector('button') }),
    true
  )
})

test('invalid saved preferences fall back to blank defaults', () => {
  for (const saved of ['broken', 'null', '{"bindings":{"toggle":42}}']) {
    const app = session(saved)
    assert.equal(app.getShortcutStatus().settings.enabled, false)
    assert.ok(
      Object.values(app.getShortcutStatus().settings.bindings).every((value) => value === '')
    )
  }
})

function playbackHarness() {
  const calls = [],
    likes = [],
    effects = [],
    refs = []
  const app = session()
  let cursor = 0,
    dispatch
  const state = {
    index: 0,
    queue: [{ source: { kind: 'netease', id: 7 } }],
    status: 'playing',
    volume: 0.98
  }
  const account = {
    profile: { reference: { source: 'netease', id: '1' } },
    likesReady: true,
    likedIds: new Set(),
    toggleLike: async (id) => {
      likes.push(id)
    }
  }
  const exports = {}
  const modules = {
    '@/features/music/use-music-sources': {
      useMusicSources: () => ({
        statuses: [
          {
            enabled: true,
            descriptor: { source: 'netease', capabilities: ['playlistWrite', 'favorites'] }
          }
        ]
      })
    },
    react: {
      useRef: (value) => (refs[cursor++] ??= { current: value }),
      useState: (value) => {
        const state = (refs[cursor++] ??= { current: value })
        return [
          state.current,
          (next) => {
            state.current = next
          }
        ]
      },
      useEffect: (fn) => effects.push(fn)
    },
    '@tauri-apps/api/core': { isTauri: () => true },
    '@/features/account/account': { useAccount: () => account },
    '@/lib/player': {
      usePlayer: () => state,
      getPlayer: () => state,
      nativeCall: async (command, args) => {
        calls.push({ command, args })
      }
    },
    '../use-volume-control': {
      playerVolume: () => state.volume,
      changePlayerVolume: (volume) =>
        calls.push({ command: 'player_volume', args: { volume: Math.max(0, Math.min(1, volume)) } })
    },
    '@/features/playback/shortcuts/shortcuts': {
      initializeShortcuts: async () => {},
      isPlaybackSpace: app.isPlaybackSpace,
      setShortcutDispatcher: (fn) => {
        dispatch = fn
      }
    }
  }
  const source = readFileSync(
    new URL('../src/features/playback/shortcuts/use-playback-shortcuts.ts', import.meta.url),
    'utf8'
  )
  runInNewContext(
    ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS } }).outputText,
    {
      exports,
      require: (name) =>
        name === '@/features/music/legacy-reference' ? musicReference : modules[name],
      window: app.window
    }
  )
  const render = () => {
    cursor = 0
    exports.usePlaybackShortcuts(() => {})
    effects.splice(0).forEach((fn) => fn())
  }
  render()
  return {
    calls,
    likes,
    state,
    account,
    render,
    window: app.window,
    document: app.document,
    dispatch: (name) => dispatch(name)
  }
}

test('all six playback actions reuse playback commands, clamp volume and favorite only eligible unliked tracks', () => {
  const app = playbackHarness()
  app.dispatch('toggle')
  app.dispatch('previous')
  app.dispatch('next')
  app.dispatch('volumeUp')
  app.dispatch('like')
  assert.deepEqual(
    app.calls.map(({ command, args }) => [command, args.action ?? args.volume]),
    [
      ['player_action', 'toggle'],
      ['player_action', 'previous'],
      ['player_action', 'next'],
      ['player_volume', 1]
    ]
  )
  assert.deepEqual(app.likes, ['7'])
  app.account.likedIds.add('7')
  app.dispatch('like')
  assert.deepEqual(app.likes, ['7'])
  app.state.status = 'paused'
  app.state.volume = 0.02
  app.render()
  app.dispatch('toggle')
  app.dispatch('volumeDown')
  assert.equal(app.calls.at(-2).args.action, 'toggle')
  assert.equal(app.calls.at(-1).args.volume, 0)
  app.state.queue = [{ source: { kind: 'local' } }]
  app.render()
  app.dispatch('like')
  assert.deepEqual(app.likes, ['7'])
  app.state.index = null
  app.render()
  const count = app.calls.length
  app.dispatch('toggle')
  app.dispatch('previous')
  app.dispatch('next')
  assert.equal(app.calls.length, count)
})

test('Space after clicking a button controls playback and prevents button handlers on both keydown and keyup', () => {
  const app = playbackHarness()
  const button = app.document.querySelector('button')
  button.click()
  button.focus()
  let activations = 0
  for (const type of ['keydown', 'keyup']) button.addEventListener(type, () => activations++)
  for (const [type, repeat] of [
    ['keydown', false],
    ['keydown', true],
    ['keyup', false]
  ]) {
    const event = new app.window.KeyboardEvent(type, {
      code: 'Space',
      key: ' ',
      repeat,
      bubbles: true,
      cancelable: true
    })
    button.dispatchEvent(event)
    assert.equal(event.defaultPrevented, true, `${type} should prevent the button default action`)
  }
  assert.equal(activations, 0)
  assert.equal(app.calls.length, 1)
  assert.equal(app.calls[0].args.action, 'toggle')
})

function loadKeys() {
  const exports = {}
  const source = readFileSync(
    new URL('../src/features/playback/shortcuts/shortcut-keys.ts', import.meta.url),
    'utf8'
  )
  runInNewContext(
    ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS } }).outputText,
    { exports }
  )
  return exports
}

test('recording accepts physical two/three/four-key chords, rejects single and five-key chords and normalizes aliases', () => {
  const keys = loadKeys()
  const event = { code: 'KeyP', ctrlKey: true }
  assert.equal(keys.captureShortcut(event).binding, 'Ctrl+P')
  assert.equal(keys.captureShortcut({ ...event, shiftKey: true }).binding, 'Ctrl+Shift+P')
  assert.equal(keys.captureShortcut({ code: 'Digit1', metaKey: true }).binding, 'Super+1')
  assert.equal(keys.captureShortcut({ code: 'KeyP' }).binding, undefined)
  assert.equal(
    keys.captureShortcut({ ...event, shiftKey: true, altKey: true }).binding,
    'Ctrl+Alt+Shift+P'
  )
  assert.match(
    keys.captureShortcut({ ...event, shiftKey: true, altKey: true, metaKey: true }).error,
    /最多四个键/
  )
  assert.equal(keys.captureShortcut({ ...event, code: 'ControlLeft' }).binding, undefined)
  assert.equal(keys.normalizeShortcut(' Shift + control + p '), 'Ctrl+Shift+P')
  for (const value of [
    'P',
    'Space',
    'Ctrl+Alt+Shift+Super+P',
    'Ctrl+Ctrl+P',
    'Ctrl+Alt',
    'Ctrl+bogus'
  ])
    assert.equal(keys.normalizeShortcut(value), undefined)
})

test('invalid legacy bindings are cleared, invalid save is rejected, and recording suspends global bindings then restores', async () => {
  const app = session('{"enabled":true,"bindings":{"toggle":"Space","next":"Ctrl+Alt+N"}}')
  assert.equal(app.getShortcutStatus().settings.bindings.toggle, '')
  await app.initializeShortcuts() // The mock reserves Ctrl+Alt+N, so startup exposes the conflict.
  await app.saveShortcuts(config(app, { next: '', toggle: 'Ctrl+P' }))
  const saved = app.values.get('nons-shortcut-settings')
  assert.equal(await app.saveShortcuts(config(app, { previous: 'Ctrl+Alt+Shift+Super+P' })), false)
  assert.equal(app.values.get('nons-shortcut-settings'), saved)
  await app.setShortcutRecording(true)
  assert.equal(app.registered.size, 0)
  await app.setShortcutRecording(false)
  assert.deepEqual([...app.registered.keys()], ['Ctrl+P'])
})
