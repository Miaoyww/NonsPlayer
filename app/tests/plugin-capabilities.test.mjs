import { coverImage } from './helpers/cover-image.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from 'node:vm'
import { test } from 'node:test'
import ts from 'typescript'

function load(path, modules) {
  modules = { '@/features/music/cover-image': coverImage, ...modules }
  const exports = {}
  runInNewContext(
    ts.transpileModule(readFileSync(new URL(path, import.meta.url), 'utf8'), {
      compilerOptions: { module: ts.ModuleKind.CommonJS }
    }).outputText,
    {
      exports,
      require: (name) => {
        assert.ok(name in modules, name)
        return modules[name]
      }
    }
  )
  return exports
}
function harness(
  permissions,
  { listen = async () => () => {}, nativeCall = async () => 'null' } = {}
) {
  const scope = {
    active: true,
    descriptor: { generation: 7, manifest: { id: 'sample', permissions } }
  }
  const react = {
    createContext: () => ({}),
    useContext: () => scope,
    useMemo: (fn) => fn(),
    useCallback: (fn) => fn
  }
  const scopes = load('../src/plugins/scope.tsx', { react })
  const sdk = load('../src/plugins/sdk.ts', {
    react,
    './collection-tabs': load('../src/plugins/collection-tabs.ts', { './scope': scopes }),
    './scope': scopes,
    './types': {},
    './song-components': {},
    '@tauri-apps/api/event': { listen },
    '@/lib/player': { nativeCall },
    '@/features/settings/use-theme': {},
    '@/components/music/use-cover-source': {},
    '@/features/workspace/music-navigation': {},
    '@/components/music/music-page': { MusicPageHeader: 'header' },
    '@/components/ui/button': {},
    '@/components/ui/progress': {},
    '@/components/ui/input': {},
    '@/components/animate-ui/components/base/tabs': {},
    './downloaded-song-list': {},
    './icon': {}
  })
  return { scope, sdk }
}

test('clipboard subscription filters identity and generation and cleans up without replay', async () => {
  let listener,
    disposed = 0
  const { scope, sdk } = harness(['clipboard:read'], {
    listen: async (event, callback) => {
      assert.equal(event, 'plugin-clipboard-changed')
      listener = callback
      return () => disposed++
    }
  })
  const received = []
  const unsubscribe = await sdk.usePluginClipboard().subscribe((text) => received.push(text))
  assert.deepEqual(received, [])
  const emit = (pluginId, generation, text) => listener({ payload: { pluginId, generation, text } })
  emit('other', 7, 'private')
  emit('sample', 6, 'stale')
  emit('sample', 7, 'plain text without a link')
  emit('sample', 7, '')
  assert.deepEqual(received, ['plain text without a link', ''])
  unsubscribe()
  unsubscribe()
  emit('sample', 7, 'late')
  assert.equal(disposed, 1)
  assert.equal(scope.cleanups.size, 0)
  const cleanup = await sdk.usePluginClipboard().subscribe((text) => received.push(text))
  scope.active = false
  scope.cleanups.forEach((fn) => fn())
  emit('sample', 7, 'disabled')
  assert.equal(disposed, 2)
  cleanup()
  assert.deepEqual(received, ['plain text without a link', ''])
})

test('new capabilities reject missing permission and late subscription or read results', async () => {
  const denied = harness([]).sdk
  assert.throws(() => denied.usePluginClipboard(), /clipboard:read/)
  assert.throws(() => denied.usePluginHttp(), /http:request/)
  let finishListen,
    finishRead,
    disposed = 0
  const { scope, sdk } = harness(['clipboard:read'], {
    listen: () =>
      new Promise((resolve) => {
        finishListen = resolve
      }),
    nativeCall: () =>
      new Promise((resolve) => {
        finishRead = resolve
      })
  })
  const clipboard = sdk.usePluginClipboard()
  const subscribed = clipboard.subscribe(() => {})
  const read = clipboard.readText()
  scope.active = false
  finishListen(() => disposed++)
  finishRead('"late text"')
  await assert.rejects(subscribed, /禁用/)
  await assert.rejects(read, /禁用/)
  assert.equal(disposed, 1)
})

test('HTTP and music facades bind host requests to the plugin instance', async () => {
  const calls = []
  const { sdk } = harness(['http:request', 'music:metadata', 'player:control'], {
    nativeCall: async (command, args) => {
      calls.push({ command, ...args })
      return args.operation === 'http.request'
        ? '{"status":302,"headers":{"location":"/next"},"body":""}'
        : 'null'
    }
  })
  const request = { url: 'https://example.org/', responseType: 'none' }
  const response = await sdk.usePluginHttp().request(request)
  assert.equal(response.status, 302)
  assert.equal(response.headers.location, '/next')
  await sdk.useMusicSource().getTrack({ source: 'qqmusic', kind: 'track', id: 'opaque' })
  await sdk.useSongPlayback()({ source: 'qqmusic', kind: 'track', id: 'opaque' }, 'next')
  assert.deepEqual(
    calls.map((call) => call.operation),
    ['http.request', 'music.read-track', 'music.play-track']
  )
  for (const call of calls) {
    assert.equal(call.command, 'plugin_host_call')
    assert.equal(call.id, 'sample')
    assert.equal(call.generation, 7)
  }
  assert.deepEqual(JSON.parse(calls[0].args), request)
})

test('detail tab registration is bound to UI permission and plugin lifecycle', () => {
  const denied = harness([])
  assert.throws(
    () =>
      denied.sdk
        .createPluginClient(denied.scope, () => {})
        .registerCollectionTab({ id: 'comments', label: '评论', component() {} }),
    /ui/
  )
  const { scope, sdk } = harness(['ui'])
  const client = sdk.createPluginClient(scope, () => {})
  const tab = { id: 'comments', label: '评论', component() {} }
  const dispose = client.registerCollectionTab(tab)
  assert.equal(scope.cleanups.size, 1)
  assert.throws(() => client.registerCollectionTab(tab), /重复/)
  for (let i = 0; i < 7; i++) client.registerCollectionTab({ ...tab, id: `tab-${i}` })
  assert.throws(() => client.registerCollectionTab({ ...tab, id: 'overflow' }), /最多/)
  scope.active = false
  scope.cleanups.forEach((cleanup) => cleanup())
  assert.equal(scope.cleanups.size, 0)
  assert.doesNotThrow(dispose)
  assert.throws(() => client.registerCollectionTab({ ...tab, id: 'late' }), /禁用/)
})

test('uniform music client preserves opaque references and separates library and write permissions', async () => {
  const calls = []
  const { scope, sdk } = harness(['music:metadata'], {
    nativeCall: async (command, args) => {
      assert.equal(command, 'plugin_host_call')
      calls.push({ operation: args.operation, args: JSON.parse(args.args) })
      return 'null'
    }
  })
  const client = sdk.useMusicSource()
  const reference = { source: 'other', kind: 'track', id: 'A:non-numeric/01' }
  await client.getTrack(reference)
  assert.deepEqual(calls[0], { operation: 'music.read-track', args: { reference } })
  assert.throws(() => client.query('other', { operation: 'favorites' }), /music:library/)
  assert.throws(
    () => client.write('other', { operation: 'setFavorite', track: reference, liked: true }),
    /music:write/
  )
  scope.descriptor.manifest.permissions.push('music:library', 'music:write')
  await client.query('other', { operation: 'favorites' })
  await client.write('other', { operation: 'setFavorite', track: reference, liked: true })
  assert.equal(calls[1].operation, 'music.query')
  assert.equal(calls[2].operation, 'music.write')
  scope.active = false
  assert.throws(() => client.getTrack(reference), /插件已禁用或卸载/)
})
