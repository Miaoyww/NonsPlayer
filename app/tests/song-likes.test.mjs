import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import ts from 'typescript'

// Run the real account provider with controlled requests and hook lifecycles.
const record = (id) => ({
  reference: { source: 'netease', id: String(id) },
  displayName: 'Listener',
  avatar: ''
})
const profile = (id) => ({
  reference: { source: 'netease', id: String(id) },
  userId: String(id),
  nickname: 'Listener',
  avatarUrl: ''
})
const descriptor = { source: 'netease', capabilities: ['account', 'favorites', 'userLibrary'] }
function harness(snapshot = null, storage = new Map()) {
  const slots = [],
    requests = [],
    effects = [],
    libraryRefreshes = []
  const events = new Map()
  const browserEvents = new Map()
  let now = 1_800_000_000_000
  let cursor = 0
  const react = {
    createContext: () => ({ Provider: 'provider' }),
    useContext: () => new Map(),
    useState(initial) {
      const index = cursor++
      if (!(index in slots)) slots[index] = typeof initial === 'function' ? initial() : initial
      return [
        slots[index],
        (value) => {
          slots[index] = typeof value === 'function' ? value(slots[index]) : value
        }
      ]
    },
    useRef(initial) {
      const index = cursor++
      return (slots[index] ??= { current: initial })
    },
    useCallback: (fn) => fn,
    useEffect(fn, deps) {
      const index = cursor++,
        old = slots[index]
      if (!old || deps.some((value, i) => value !== old.deps[i])) {
        old?.cleanup?.()
        slots[index] = { deps }
        effects.push(() => {
          slots[index].cleanup = fn()
        })
      }
    }
  }
  const request = (command, args, convert = (value) => value) =>
    new Promise((resolve, reject) =>
      requests.push({ command, args, resolve: (value) => resolve(convert(value)), reject })
    )
  const modules = {
    '@/features/music/use-music-sources': {
      useMusicSources: () => ({
        preferences: { current: 'netease' },
        statuses: [{ descriptor, enabled: true }]
      })
    },
    '@/lib/runtime-cache': { invalidateNativeCache() {} },
    '@/features/music/client': {
      musicClient: {
        currentAccount: async () =>
          snapshot
            ? { ...record(snapshot.userId), displayName: snapshot.nickname ?? 'Listener' }
            : null,
        account: (source, args) =>
          request('music_account', { source, request: args }, (value) => ({
            type: 'profile',
            data: value ? record(value.userId) : null
          })),
        query: (source, args) =>
          request('music_query', { source, request: args }, (value) => ({
            type: 'favorites',
            data: value.map((id) => ({ source, kind: 'track', id: String(id) }))
          })),
        write: (source, args) => request('music_write', { source, request: args })
      }
    },
    react,
    'react/jsx-runtime': { jsx: (type, props) => ({ type, props }) },
    '@tauri-apps/api/core': { isTauri: () => true },
    '@tauri-apps/api/event': {
      listen: async (event, callback) => {
        events.set(event, callback)
        return () => events.delete(event)
      }
    },
    '@/features/library/library-api': {
      resetAccountCache() {},
      setLibraryAccount() {},
      invalidateMusicLibrary() {
        libraryRefreshes.push('invalidate')
      },
      getMusicLibrary(userId) {
        libraryRefreshes.push(userId)
        return Promise.resolve()
      }
    },
    '@/lib/player': {
      errorText: String,
      nativeCall(command, args) {
        if (command === 'account_profile_snapshot') return Promise.resolve(snapshot)
        return new Promise((resolve, reject) => requests.push({ command, args, resolve, reject }))
      }
    }
  }
  const exports = {}
  modules['@/features/music/client'].musicClient.refresh =
    modules['@/features/music/client'].musicClient.query
  const { outputText } = ts.transpileModule(
    readFileSync(new URL('../src/features/account/account-state.tsx', import.meta.url), 'utf8'),
    { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX } }
  )
  const localStorage = {
    getItem: (key) => storage.get(key) ?? null,
    setItem: (key, value) => storage.set(key, value)
  }
  const document = {
    visibilityState: 'visible',
    addEventListener: (name, fn) => browserEvents.set(name, fn),
    removeEventListener: (name) => browserEvents.delete(name)
  }
  const window = {
    addEventListener: (name, fn) => browserEvents.set(name, fn),
    removeEventListener: (name) => browserEvents.delete(name),
    setInterval: () => 1,
    clearInterval() {}
  }
  runInNewContext(outputText, {
    exports,
    require: (name) => modules[name],
    localStorage,
    window,
    document,
    Date: { now: () => now }
  })
  function render() {
    cursor = 0
    const tree = exports.AccountProvider({ children: null })
    const value = tree.type(tree.props).props.value.get('netease')
    effects.splice(0).forEach((fn) => fn())
    return value
  }
  return {
    requests,
    render,
    libraryRefreshes,
    storage,
    focus: () => browserEvents.get('focus')?.(),
    visible: (visible) => {
      document.visibilityState = visible ? 'visible' : 'hidden'
      browserEvents.get('visibilitychange')?.()
    },
    advance: (ms) => {
      now += ms
    },
    emit: (event, payload) => events.get(event)?.({ payload })
  }
}
const settle = () => new Promise((resolve) => setImmediate(resolve))
async function loggedIn(storage) {
  const app = harness(null, storage)
  app.render()
  await settle()
  app.requests[0].resolve({ userId: 1 })
  await settle()
  app.render()
  app.requests[1].resolve([123])
  await settle()
  return app
}

test('likes update only after success and duplicate clicks issue one request', async () => {
  const app = await loggedIn()
  const state = app.render()
  assert.equal(state.likedIds.has('123'), true)
  const first = state.toggleLike('123')
  await state.toggleLike('123')
  assert.equal(app.requests.length, 3)
  assert.deepEqual(JSON.parse(JSON.stringify(app.requests[2].args)), {
    source: 'netease',
    request: {
      operation: 'setFavorite',
      track: { source: 'netease', kind: 'track', id: '123' },
      liked: false
    }
  })
  assert.equal(app.render().likedIds.has('123'), true)
  assert.equal(app.render().pendingLikes.has('123'), true)
  app.requests[2].resolve()
  await first
  assert.equal(app.render().likedIds.has('123'), false)
  assert.equal(app.render().pendingLikes.size, 0)
  assert.equal(app.render().likesRevision, 1)
  assert.deepEqual(app.libraryRefreshes, ['invalidate'])
})

test('failed writes preserve the liked state and release the button', async () => {
  const app = await loggedIn()
  const request = app.render().toggleLike('123')
  app.requests[2].reject(new Error('network failure'))
  await assert.rejects(request, /network failure/)
  assert.equal(app.render().likedIds.has('123'), true)
  assert.equal(app.render().pendingLikes.size, 0)
  assert.deepEqual(app.libraryRefreshes, [])
})

test('switching account preserves global likes and applies a successful pending write', async () => {
  const app = await loggedIn()
  const old = app.render().toggleLike('123')
  app.render().setProfile(profile(2))
  app.requests[2].resolve()
  await old
  assert.deepEqual([...app.render().likedIds], [])
})

test('switching account discards old liked list responses', async () => {
  const app = harness()
  app.render()
  await settle()
  app.requests[0].resolve({ userId: 1 })
  await settle()
  app.render()
  app.render().setProfile(profile(2))
  app.render()
  app.requests[2].resolve(['456'])
  await settle()
  app.requests[1].resolve([123])
  await settle()
  assert.deepEqual([...app.render().likedIds], ['456'])
})

test('backend account changes keep global likes visible while revalidating', async () => {
  const app = await loggedIn()
  const oldWrite = app.render().toggleLike('123')
  app.emit('music-account-changed', 'netease')
  app.render()
  await settle()
  const profile = app.requests.findLast((request) => request.command === 'music_account')
  profile.resolve({ userId: 2 })
  await settle()
  app.render()
  app.requests.find((request) => request.command === 'music_write').resolve()
  await oldWrite
  await settle()
  const state = app.render()
  assert.equal(state.profile.userId, '2')
  assert.deepEqual([...state.likedIds], [])
  assert.equal(app.requests.filter((request) => request.command === 'music_query').length, 2)
  assert.deepEqual(app.libraryRefreshes, ['invalidate'])
})

test('global likes persist across provider recreation and account changes', async () => {
  const storage = new Map()
  const first = await loggedIn(storage)
  assert.deepEqual([...first.render().likedIds], ['123'])
  first.render().setProfile(profile(2))
  assert.deepEqual([...first.render().likedIds], ['123'])

  const restarted = harness({ userId: 2 }, storage)
  restarted.render()
  await settle()
  const validation = restarted.requests.find((request) => request.command === 'music_account')
  validation.resolve({ userId: 2 })
  await settle()
  assert.deepEqual([...restarted.render().likedIds], ['123'])
  assert.equal(restarted.requests.filter((request) => request.command === 'music_query').length, 1)
})

test('saved current account appears before remote validation and network failure preserves its display', async () => {
  const saved = { userId: 7, nickname: 'Saved', avatarUrl: '' }
  const app = harness(saved)
  app.render()
  await settle()
  const state = app.render()
  assert.equal(state.profile.userId, '7')
  assert.equal(state.loading, false)
  const validation = app.requests.find((request) => request.command === 'music_account')
  assert.ok(validation)
  validation.reject(new Error('offline'))
  await settle()
  assert.equal(app.render().profile.userId, '7')
  assert.match(app.render().error, /offline/)
})

test('stored likes display immediately and are corrected by one background read', async () => {
  const storage = new Map([
    [
      'nons:music:favorites:v2:netease',
      JSON.stringify({ ids: ['old'], updatedAt: 1_799_999_999_000 })
    ]
  ])
  const app = harness({ userId: 1 }, storage)
  assert.equal(app.render().favoriteStatus('old'), 'liked')
  await settle()
  app.render()
  const requests = app.requests.filter((request) => request.command === 'music_query')
  assert.equal(requests.length, 1)
  assert.equal(app.render().likesRefreshing, true)
  requests[0].resolve(['new'])
  await settle()
  const state = app.render()
  assert.equal(state.favoriteStatus('new'), 'liked')
  assert.equal(state.favoriteStatus('old'), 'unliked')
  assert.equal(state.likesStale, false)
})

test('focus revalidates expired likes once and keeps the last good snapshot on failure', async () => {
  const app = await loggedIn()
  app.render()
  app.advance(10 * 60 * 1000 - 1)
  app.focus()
  app.render()
  assert.equal(app.requests.length, 2)
  app.advance(2)
  app.focus()
  app.render()
  app.focus()
  app.render()
  assert.equal(app.requests.length, 3)
  app.requests[2].reject(new Error('offline'))
  await settle()
  const state = app.render()
  assert.equal(state.favoriteStatus('123'), 'liked')
  assert.equal(state.likesReady, true)
  assert.equal(state.likesStale, true)
  assert.match(state.likesError, /offline/)
  app.focus()
  app.render()
  assert.equal(app.requests.length, 3, 'failure backoff suppresses focus bursts')
})

test('a successful mutation wins over a refresh that started before it', async () => {
  const app = await loggedIn()
  app.render().reloadLikes()
  app.render()
  const write = app.render().toggleLike('123')
  app.requests[3].resolve()
  await write
  assert.equal(app.render().favoriteStatus('123'), 'unliked')
  app.requests[2].resolve([123])
  await settle()
  assert.equal(app.render().favoriteStatus('123'), 'unliked')
})

test('confirmed own-playlist members add only positive evidence and cannot undo a local unlike', async () => {
  const app = await loggedIn()
  const track = (id) => ({
    source: { kind: 'online', track: { reference: { source: 'netease', kind: 'track', id } } }
  })
  app.render().confirmLikedTracks([track('new')])
  assert.equal(app.render().favoriteStatus('new'), 'liked')
  const write = app.render().toggleLike('123')
  app.requests[2].resolve()
  await write
  app.render().confirmLikedTracks([track('123')])
  assert.equal(app.render().favoriteStatus('123'), 'unliked')
  app.render().reloadLikes()
  app.render()
  app.requests[3].resolve(['new'])
  await settle()
  app.render().confirmLikedTracks([track('123')])
  assert.equal(
    app.render().favoriteStatus('123'),
    'unliked',
    'an old playlist page cannot undo a confirmed unlike after revalidation'
  )
})

test('platform playlist writes refresh the shared favorite state through music-changed', async () => {
  const app = await loggedIn()
  app.emit('music-changed', 'qqmusic')
  app.render()
  assert.equal(app.requests.length, 2)
  app.emit('music-changed', 'netease')
  app.render()
  assert.equal(app.render().favoriteStatus('123'), 'liked')
  app.requests[2].resolve(['new'])
  await settle()
  assert.equal(app.render().favoriteStatus('new'), 'liked')
  assert.equal(app.render().favoriteStatus('123'), 'unliked')
})

test('hidden windows wait until visible before revalidating an expired snapshot', async () => {
  const app = await loggedIn()
  app.render()
  app.visible(false)
  app.advance(10 * 60 * 1000 + 1)
  app.focus()
  app.render()
  assert.equal(app.requests.length, 2)
  app.visible(true)
  app.render()
  assert.equal(app.requests.length, 3)
  app.requests[2].resolve([123])
  await settle()
  assert.equal(app.render().likesStale, false)
})

test('a failed read in the same millisecond as a success still uses the failure backoff', async () => {
  const app = await loggedIn()
  app.render().reloadLikes()
  app.render()
  app.requests[2].reject(new Error('offline'))
  await settle()
  app.render()
  app.advance(30_001)
  app.focus()
  app.render()
  assert.equal(app.requests.length, 4)
  app.requests[3].resolve([123])
  await settle()
  assert.equal(app.render().likesStale, false)
})

test('write event bursts queue one replacement read and cannot publish an obsolete response', async () => {
  const app = await loggedIn()
  app.render().reloadLikes()
  app.render()
  for (let index = 0; index < 5; index++) {
    app.emit('music-changed', 'netease')
    app.render()
  }
  assert.equal(app.requests.length, 3, 'an in-flight read is not restarted for every event')
  app.requests[2].resolve(['obsolete'])
  await settle()
  app.render()
  assert.equal(app.requests.length, 4)
  assert.equal(app.render().favoriteStatus('obsolete'), 'unliked')
  app.requests[3].resolve(['latest'])
  await settle()
  assert.equal(app.render().favoriteStatus('latest'), 'liked')
})
