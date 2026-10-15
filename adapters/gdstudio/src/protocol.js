import { parseFromTokenizer } from 'music-metadata'
import { fromBuffer } from 'strtok3/core'

export const platforms = { netease: 'netease' }
export function fail(code, diagnostic = 'validation') {
  throw Object.assign(new Error(code), { code, diagnostic })
}
const high = new Set([
  '320k',
  'exhigh',
  'higher',
  'lossless',
  'hires',
  'jyeffect',
  'sky',
  'jymaster',
  'flac',
  'flac24bit',
  'master',
  'atmos'
])
const lossless = new Set([
  'lossless',
  'hires',
  'jyeffect',
  'sky',
  'jymaster',
  'flac',
  'flac24bit',
  'master',
  'atmos'
])
export function requestedQuality(quality, downgrade) {
  if (lossless.has(quality) && !downgrade) fail('permissionDenied')
  if (!high.has(quality) && !['standard', '128k', 'low'].includes(quality)) fail('unsupported')
  return high.has(quality) ? '320k' : '128k'
}
export function apiRequest(request) {
  const platform = platforms[request.track?.reference?.source]
  const id = request.track?.reference?.id
  if (!platform || typeof id !== 'string' || !id || id.length > 1024) fail('invalidData')
  const quality = requestedQuality(request.preferredQuality, request.allowDowngrade)
  return {
    quality,
    url: `https://music-api.gdstudio.xyz/api.php?types=url&source=${platform}&id=${encodeURIComponent(id)}&br=${Number.parseInt(quality)}`
  }
}
export function decodeResponse(status, body) {
  if (status === 429) fail('rateLimited')
  if (status < 200 || status >= 300) fail('network')
  if (!body || typeof body !== 'object') fail('invalidData')
  if (body.isPreview === true || body.preview === true) fail('permissionDenied')
  if (typeof body.url !== 'string') fail('invalidData')
  if (!body.url) fail('notFound')
  return body.url
}
export function searchRequest(input) {
  if (
    input.platform !== 'netease' ||
    typeof input.keyword !== 'string' ||
    !input.keyword.trim() ||
    input.keyword.length > 1024
  )
    fail('invalidData')
  const page = input.page.cursor === null ? 1 : Number(input.page.cursor)
  const limit = input.page.limit
  if (
    !Number.isSafeInteger(page) ||
    page < 1 ||
    page > 100 ||
    !Number.isSafeInteger(limit) ||
    limit < 1 ||
    limit > 100
  )
    fail('invalidData')
  return {
    page,
    limit,
    url: `https://music-api.gdstudio.xyz/api.php?types=search&source=netease&name=${encodeURIComponent(input.keyword.trim())}&count=${limit}&pages=${page}`
  }
}
export function decodeSearch(body, page, limit) {
  if (!Array.isArray(body) || body.length > limit) fail('invalidData')
  const items = body.map((item) => {
    if (
      item.source !== 'netease' ||
      typeof item.id !== 'string' ||
      !item.id ||
      typeof item.name !== 'string' ||
      !item.name ||
      !Array.isArray(item.artist) ||
      item.artist.some((artist) => typeof artist !== 'string') ||
      (item.url_id && item.url_id !== item.id)
    )
      fail('invalidData')
    return {
      reference: { source: 'netease', kind: 'track', id: item.id },
      title: item.name,
      aliases: [],
      artist: item.artist.join(' / '),
      artists: item.artist.map((name) => ({ name, reference: null })),
      album: typeof item.album === 'string' ? item.album : '',
      albumReference: null,
      durationMs: 0,
      cover: '',
      associations: []
    }
  })
  return { items, nextCursor: items.length === limit && page < 100 ? String(page + 1) : null }
}
/** A bounded prefix plus HTTP total length suffices for CBR or a trusted VBR
 * frame-count header. No whole-song download, guest cache or hand-written codec. */
export async function inspectAudio(bytes, size, expectedDuration, requested, downgrade, tail) {
  if (
    !(bytes instanceof Uint8Array) ||
    bytes.length > 65536 ||
    !Number.isSafeInteger(size) ||
    size <= 0 ||
    size > 512 * 1024 * 1024 ||
    bytes.length > size ||
    expectedDuration <= 0
  )
    fail('invalidData', 'probe-input')
  const tokenizer = fromBuffer(bytes, { fileInfo: { mimeType: 'audio/mpeg' } })
  // BufferTokenizer otherwise substitutes the prefix length for the complete file.
  tokenizer.fileInfo.size = size
  if (!(tail instanceof Uint8Array) || tail.length !== Math.min(512, size))
    fail('invalidData', 'probe-footer')
  const end = fromBuffer(tail)
  const peek = tokenizer.peekBuffer.bind(tokenizer)
  // Expose only the downloaded prefix and footer to the library's tokenizer.
  // Requests for missing middle bytes still fail instead of allocating the file.
  tokenizer.peekBuffer = (target, options = {}) => {
    const position = options.position ?? tokenizer.position
    return position >= size - tail.length
      ? end.peekBuffer(target, { ...options, position: position - (size - tail.length) })
      : peek(target, options)
  }
  let metadata
  try {
    metadata = await parseFromTokenizer(tokenizer, {
      skipCovers: true,
      skipPostHeaders: true,
      duration: false
    })
  } catch (error) {
    fail('invalidData', error.name)
  }
  const { format } = metadata
  if (
    format.container !== 'MPEG' ||
    !format.codec?.includes('Layer 3') ||
    !format.duration ||
    !format.bitrate
  )
    fail(
      'invalidData',
      !format.duration
        ? 'missing-duration'
        : !format.bitrate
          ? 'missing-bitrate'
          : 'unsupported-format'
    )
  if (Math.abs(format.duration * 1000 - expectedDuration) > Math.max(2000, expectedDuration * 0.01))
    fail('permissionDenied')
  const bitrate = format.bitrate / 1000
  const actual =
    Math.abs(bitrate - 320) <= 12 ? '320k' : Math.abs(bitrate - 128) <= 8 ? '128k' : null
  if (!actual || (!downgrade && Number.parseInt(actual) < Number.parseInt(requested)))
    fail('permissionDenied')
  return { actualQuality: actual, extent: { kind: 'full' }, expiresAtMs: null }
}
