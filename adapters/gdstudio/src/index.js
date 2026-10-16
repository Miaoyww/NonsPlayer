import { httpRequest } from 'nons:host/transport@1.0.0'
import { call as hostCall } from 'nons:plugin/host@1.0.0'
import manifest from '../manifest.json' with { type: 'json' }
import { apiRequest, decodeResponse, fail } from './protocol.js'
import { sign } from './signing.js'
import { inspectRemoteAudio } from './audio.js'
export function initialize() {}
export function shutdown() {}
const base = 'https://music.gdstudio.org/'
const headersOf = (response) =>
  Object.fromEntries(response.headers.map(({ name, value }) => [name.toLowerCase(), value]))
function allowed(input) {
  let url
  try {
    url = new URL(input, base)
  } catch {
    fail('invalidData')
  }
  const host = url.hostname
  if (
    !['http:', 'https:'].includes(url.protocol) ||
    url.username ||
    url.password ||
    url.port ||
    url.hash ||
    !manifest.permissions.httpHosts.some(
      (rule) =>
        rule === host ||
        (rule.startsWith('*.') && host.endsWith(rule.slice(1)) && host.length > rule.length - 1)
    )
  )
    fail('permissionDenied')
  return url.href
}
function request(url, headers = []) {
  try {
    const response = httpRequest({
      url: allowed(url),
      method: 'GET',
      headers,
      body: new Uint8Array(),
      timeoutMs: 4000
    })
    return { ...response, body: new Uint8Array(response.body) }
  } catch (error) {
    if (error.code) throw error
    fail('network')
  }
}
function check(response, maxBytes) {
  if (response.status === 429) fail('rateLimited')
  if (response.status === 401 || response.status === 403) fail('permissionDenied', 'upstream-auth')
  if (response.status < 200 || response.status >= 300) fail('network')
  if (response.body.length > maxBytes) fail('invalidData')
}
function json(response, maxBytes) {
  check(response, maxBytes)
  try {
    return JSON.parse(new TextDecoder().decode(response.body))
  } catch {
    fail('invalidData')
  }
}
function apiJson(target) {
  const headers = [
    {
      name: 'User-Agent',
      value:
        'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/149.0.0.0 Safari/537.36'
    },
    { name: 'Referer', value: base },
    { name: 'Origin', value: base.slice(0, -1) },
    { name: 'X-Requested-With', value: 'XMLHttpRequest' },
    { name: 'Accept', value: 'application/json' }
  ]
  // Refresh per request: the signature's time bucket lasts only ten seconds.
  const time = request(`${base}time`, headers)
  check(time, 32)
  let signature
  try {
    signature = sign(
      target.payload,
      new TextDecoder().decode(time.body).trim(),
      new URL(base).hostname
    )
  } catch {
    fail('invalidData', 'server-time')
  }
  return json(request(`${target.url}&s=${signature}`, headers), 65536)
}
async function resolveResource(input) {
  const target = apiRequest(input)
  const resource = apiJson(target)
  let url = allowed(decodeResponse(200, resource))
  let duration = input.track.durationMs
  // Only the selected BVID is hydrated; no per-row search-page requests.
  if (!duration && input.track.reference.source === 'bilibili') {
    const detail = json(
      request(
        `https://api.bilibili.com/x/web-interface/view?bvid=${encodeURIComponent(input.track.reference.id)}`
      ),
      65536
    )
    if (
      detail.code !== 0 ||
      detail.data?.bvid !== input.track.reference.id ||
      detail.data.videos !== 1
    )
      fail('notFound', 'bilibili-detail')
    duration = detail.data.duration * 1000
  }
  const read = async (start, end) => {
    for (let hop = 0; hop <= 3; hop++) {
      const response = request(url, [{ name: 'Range', value: `bytes=${start}-${end}` }])
      if ([301, 302, 303, 307, 308].includes(response.status)) {
        const location = headersOf(response).location
        if (!location || hop === 3) fail('network')
        url = allowed(new URL(location, url).href)
        continue
      }
      check(response, 65536)
      const range = /^bytes (\d+)-(\d+)\/(\d+)$/.exec(headersOf(response)['content-range'] ?? '')
      if (
        response.status !== 206 ||
        !range ||
        Number(range[1]) !== start ||
        Number(range[2]) !== Math.min(end, Number(range[3]) - 1) ||
        Number(range[2]) - start + 1 !== response.body.length
      )
        fail('invalidData', 'audio-range')
      return { size: Number(range[3]), bytes: response.body }
    }
    fail('network')
  }
  const { metadata } = await inspectRemoteAudio(
    read,
    duration,
    target.quality,
    input.allowDowngrade
  )
  return { metadata, url, headers: [] }
}
export async function call(method, args) {
  try {
    let data
    if (method === 'music.resource-descriptor') data = manifest.resourceProvider
    else if (method === 'music.resolve-resource')
      data = await resolveResource(JSON.parse(args).request)
    else fail('unsupported')
    const reply = JSON.stringify({ status: 'ok', data })
    if (new TextEncoder().encode(reply).length > 65536) fail('invalidData')
    return reply
  } catch (error) {
    try {
      hostCall(
        'diagnostic.report',
        JSON.stringify({
          stage: method === 'music.search-resources' ? 'resource.search' : 'resource.resolve',
          class: error.diagnostic ?? 'unknown',
          code: error.code ?? 'internal'
        })
      )
    } catch {}
    const codes = new Set([
      'network',
      'invalidData',
      'permissionDenied',
      'rateLimited',
      'internal',
      'notFound',
      'unsupported'
    ])
    return JSON.stringify({
      status: 'error',
      data: { code: codes.has(error.code) ? error.code : 'internal', retryAfterMs: null }
    })
  }
}
