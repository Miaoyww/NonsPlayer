import { test } from 'node:test'
import assert from 'node:assert/strict'
import { readFile, mkdir } from 'node:fs/promises'
import { spawnSync } from 'node:child_process'
import { resolve, dirname } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'
const project = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const root = resolve(project, '../..')
const tools = resolve(root, 'packages/wasm-crypto/node_modules/@bytecodealliance')
const output = resolve(root, '.local/gdstudio-component-tests')
await mkdir(output, { recursive: true })
const result = spawnSync(
  process.execPath,
  [
    resolve(tools, 'jco/dist/jco.js'),
    'transpile',
    resolve(project, 'dist/adapter.component.wasm'),
    '-o',
    output,
    '--instantiation',
    'async'
  ],
  { encoding: 'utf8' }
)
assert.equal(result.status, 0, result.stderr)
const { instantiate } = await import(pathToFileURL(resolve(output, 'adapter.component.js')).href)
const { WASIShim } = await import(
  pathToFileURL(resolve(tools, 'preview2-shim/dist/common/instantiation.js')).href
)
const sample = new Uint8Array(65536)
for (let i = 0; i + 1044 < sample.length; i += 1044) sample.set([255, 251, 224, 0], i)
const size = Math.floor((180 * 44100) / 1152) * 1044
const request = {
  track: { reference: { source: 'netease', kind: 'track', id: '347230' }, durationMs: 180000 },
  preferredQuality: 'exhigh',
  allowDowngrade: false
}
let mode = 'full'
const requests = []
const diagnostics = []
const transport = new Proxy(
  {
    httpRequest(value) {
      requests.push(value)
      if (value.url.includes('types=search'))
        return {
          status: 200,
          headers: [],
          body: Buffer.from(
            JSON.stringify([
              {
                id: '347230',
                url_id: '347230',
                name: 'Song',
                artist: ['Artist'],
                album: 'Album',
                source: 'netease'
              }
            ])
          )
        }
      if (value.url.includes('music-api.gdstudio.xyz'))
        return {
          status: mode === 'offline' ? 503 : 200,
          headers: [],
          body: Buffer.from(
            JSON.stringify({
              code: 0,
              url:
                mode === 'private'
                  ? 'http://127.0.0.1/secret'
                  : 'https://m1.music.126.net/sample.mp3'
            })
          )
        }
      assert(
        !value.headers.some((h) => h.name.toLowerCase() === 'x-request-key'),
        'API key never reaches the audio CDN'
      )
      if (value.headers.some((h) => h.value === 'bytes=-512'))
        return {
          status: 206,
          headers: [{ name: 'content-range', value: `bytes ${size - 512}-${size - 1}/${size}` }],
          body: new Uint8Array(512)
        }
      return {
        status: 206,
        headers: [
          {
            name: 'content-range',
            value: `bytes 0-65535/${mode === 'preview' ? Math.floor(size / 6) : size}`
          }
        ],
        body: sample
      }
    }
  },
  {
    get(target, key) {
      return (
        target[key] ??
        (() => {
          throw new Error(`Unexpected Host capability: ${String(key)}`)
        })
      )
    }
  }
)
const instance = await instantiate(
  async (path) => WebAssembly.compile(await readFile(resolve(output, path))),
  {
    ...new WASIShim().getImportObject(),
    'nons:host/transport': transport,
    'nons:plugin/host': {
      call(_operation, args) {
        diagnostics.push(JSON.parse(args))
        return 'null'
      }
    }
  }
)
instance.initialize()
const call = (method, args = 'null') =>
  JSON.parse(instance.call(method, typeof args === 'string' ? args : JSON.stringify(args)))
test('real Component describes an independent searchable provider for verified platforms', () => {
  const response = call('music.resource-descriptor')
  assert.equal(response.status, 'ok')
  assert.equal(response.data.id, 'gdstudio')
  assert.deepEqual(response.data.searchPlatforms, ['netease'])
  const result = call('music.search-resources', {
    request: { platform: 'netease', keyword: 'Song', page: { cursor: null, limit: 1 } }
  })
  assert.equal(result.status, 'ok', JSON.stringify(result))
  assert.equal(result.data.items[0].reference.source, 'netease')
  assert.equal(result.data.items[0].reference.id, '347230')
  assert.equal(result.data.nextCursor, '2')
})
test('real Component resolves and inspects full MP3, isolates API headers and rejects unsafe URLs', () => {
  mode = 'full'
  requests.length = 0
  const response = call('music.resolve-resource', { request })
  assert.equal(
    response.status,
    'ok',
    JSON.stringify({ response, diagnostics, requests: requests.length })
  )
  assert.equal(response.data.metadata.actualQuality, '320k')
  assert.equal(response.data.metadata.extent.kind, 'full')
  assert.equal(requests.length, 3)
  assert.deepEqual(response.data.headers, [])
  mode = 'private'
  assert.equal(call('music.resolve-resource', { request }).data.code, 'permissionDenied')
  mode = 'offline'
  assert.equal(call('music.resolve-resource', { request }).data.code, 'network')
})
