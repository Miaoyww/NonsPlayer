import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
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
function harness(snapshot = null) {
  const slots = [],
    requests = [],
    effects = [],
    libraryRefreshes = []
  const events = new Map()
  let cursor = 0
  const react = {
    createContext: () => ({ Provider: 'provider' }),
    useContext() {},
    useState(initial) {
      const index = cursor++
      if (!(index in slots)) slots[index] = initial
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
  const { outputText } = ts.transpileModule(
    readFileSync(new URL('../src/features/account/account-state.tsx', import.meta.url), 'utf8'),
    { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX } }
  )
  runInNewContext(outputText, { exports, require: (name) => modules[name] })
  function render() {
    cursor = 0
    const value = exports.AccountProvider({ children: null }).props.value
    effects.splice(0).forEach((fn) => fn())
    return value
  }
  return {
    requests,
    render,
    libraryRefreshes,
    emit: (event, payload) => events.get(event)?.({ payload })
  }
}
const settle = () => new Promise((resolve) => setImmediate(resolve))
async function loggedIn() {
  const app = harness()
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

test('switching account discards old write responses', async () => {
  const app = await loggedIn()
  const old = app.render().toggleLike('123')
  app.render().setProfile(profile(2))
  app.render()
  app.requests[3].resolve(['456'])
  await settle()
  app.requests[2].resolve()
  await old
  assert.deepEqual([...app.render().likedIds], ['456'])
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

test('backend account changes discard pending likes and reload the selected account', async () => {
  const app = await loggedIn()
  const oldWrite = app.render().toggleLike('123')
  app.emit('music-account-changed', 'netease')
  app.render()
  await settle()
  const profile = app.requests.findLast((request) => request.command === 'music_account')
  profile.resolve({ userId: 2 })
  await settle()
  app.render()
  const likes = app.requests.findLast((request) => request.command === 'music_query')
  likes.resolve(['456'])
  app.requests.find((request) => request.command === 'music_write').resolve()
  await oldWrite
  await settle()
  const state = app.render()
  assert.equal(state.profile.userId, '2')
  assert.deepEqual([...state.likedIds], ['456'])
  assert.deepEqual(app.libraryRefreshes, [])
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
