import { httpRequest } from 'nons:host/transport@1.0.0'
import { call as hostCall } from 'nons:plugin/host@1.0.0'
import manifest from '../manifest.json' with { type: 'json' }
import {
  apiRequest,
  decodeResponse,
  inspectAudio,
  searchRequest,
  decodeSearch,
  fail
} from './protocol.js'
export function initialize() {}
export function shutdown() {}
const headersOf = (response) =>
  Object.fromEntries(response.headers.map(({ name, value }) => [name.toLowerCase(), value]))
function allowed(input) {
  let url
  try {
    url = new URL(input)
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
  const response = httpRequest({
    url: allowed(url),
    method: 'GET',
    headers,
    body: new Uint8Array(),
    timeoutMs: 4000
  })
  return { ...response, body: new Uint8Array(response.body) }
}
function apiJson(url, maxBytes) {
  let response
  try {
    response = request(url, [
      { name: 'User-Agent', value: 'NonsPlayer/1.3.3' },
      { name: 'Accept', value: 'application/json' }
    ])
  } catch {
    fail('network')
  }
  if (response.status === 429) fail('rateLimited')
  if (response.status < 200 || response.status >= 300) fail('network')
  if (response.body.length > maxBytes) fail('invalidData')
  try {
    return JSON.parse(new TextDecoder().decode(response.body))
  } catch {
    fail('invalidData')
  }
}
async function resolveResource(input) {
  const target = apiRequest(input)
  const body = apiJson(target.url, 8192)
  let url = allowed(decodeResponse(200, body))
  for (let attempt = 0; attempt < 4; attempt++) {
    let audio
    try {
      audio = request(url, [{ name: 'Range', value: 'bytes=0-65535' }])
    } catch {
      fail('network')
    }
    const headers = headersOf(audio)
    if ([301, 302, 303, 307, 308].includes(audio.status)) {
      if (!headers.location || attempt === 3) fail('network')
      url = allowed(new URL(headers.location, url).href)
      continue
    }
    let size
    if (audio.status === 206) {
      const match = /^bytes 0-(\d+)\/(\d+)$/.exec(headers['content-range'] ?? '')
      if (!match || Number(match[1]) + 1 !== audio.body.length) fail('invalidData')
      size = Number(match[2])
    } else if (audio.status === 200 && audio.body.length <= 65536)
      size = Number(headers['content-length'])
    else fail(audio.status === 429 ? 'rateLimited' : 'invalidData')
    let tail
    if (audio.body.length === size) tail = audio.body.slice(-512)
    else {
      let footer
      try {
        footer = request(url, [{ name: 'Range', value: 'bytes=-512' }])
      } catch {
        fail('network')
      }
      if (
        footer.status !== 206 ||
        headersOf(footer)['content-range'] !== `bytes ${size - 512}-${size - 1}/${size}` ||
        footer.body.length !== 512
      )
        fail('invalidData')
      tail = footer.body
    }
    const metadata = await inspectAudio(
      audio.body,
      size,
      input.track.durationMs,
      target.quality,
      input.allowDowngrade,
      tail
    )
    return { metadata, url, headers: [] }
  }
  fail('network')
}
export async function call(method, args) {
  try {
    let data
    if (method === 'music.resource-descriptor') data = manifest.resourceProvider
    else if (method === 'music.resolve-resource')
      data = await resolveResource(JSON.parse(args).request)
    else if (method === 'music.search-resources') {
      const target = searchRequest(JSON.parse(args).request)
      data = decodeSearch(apiJson(target.url, 65536), target.page, target.limit)
    } else fail('unsupported')
    const reply = JSON.stringify({ status: 'ok', data })
    if (new TextEncoder().encode(reply).length > 65536) fail('invalidData')
    return reply
  } catch (error) {
    try {
      hostCall(
        'diagnostic.report',
        JSON.stringify({
          stage: 'resource.resolve',
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
