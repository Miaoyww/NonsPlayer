export const platforms = {
  netease: 'netease',
  joox: 'joox',
  kuwo: 'kuwo',
  tidal: 'tidal',
  qobuz: 'qobuz',
  applemusic: 'apple',
  bilibili: 'bilibili',
  ytmusic: 'ytmusic',
  embeat: 'embeat',
  qqmusic: 'tencent'
}
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
export function requestedQuality(quality) {
  if (!high.has(quality) && !['standard', '128k', 'low'].includes(quality)) fail('unsupported')
  if (lossless.has(quality))
    return ['hires', 'flac24bit', 'master', 'jymaster'].includes(quality) ? 'hires' : 'lossless'
  return high.has(quality) ? '320k' : '128k'
}
export function apiRequest(request) {
  const source = request.track?.reference?.source
  const platform = Object.hasOwn(platforms, source) ? platforms[source] : null
  const id = request.track?.reference?.id
  if (!platform || typeof id !== 'string' || !id || id.length > 1024) fail('invalidData')
  const quality = requestedQuality(request.preferredQuality)
  return {
    quality,
    payload: id,
    url: `https://music.gdstudio.org/api.php?types=url&source=${platform}&id=${encodeURIComponent(id)}&br=${lossless.has(quality) ? 999 : Number.parseInt(quality)}`
  }
}
export function decodeResponse(status, body) {
  if (status === 429) fail('rateLimited')
  if (status < 200 || status >= 300) fail('network')
  if (!body || typeof body !== 'object') fail('invalidData')
  if (body.isPreview === true || body.preview === true) fail('permissionDenied')
  if (typeof body.url !== 'string') fail('invalidData')
  if (!body.url || body.br === -1 || body.br === '-1' || body.size === 0 || body.size === '0')
    fail('notFound')
  return body.url
}
export function searchRequest(input) {
  if (
    !Object.hasOwn(platforms, input.platform) ||
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
    platform: input.platform,
    payload: input.keyword.trim(),
    // JOOX and YouTube ignore count and return fixed-size pages. A cursor
    // addresses an offset inside those upstream pages, so no rows are lost.
    url: `https://music.gdstudio.org/api.php?types=search&source=${platforms[input.platform]}&name=${encodeURIComponent(input.keyword.trim())}&count=${input.platform === 'joox' ? 30 : input.platform === 'ytmusic' ? 20 : limit}&pages=${input.platform === 'joox' || input.platform === 'ytmusic' ? Math.floor(((page - 1) * limit) / (input.platform === 'joox' ? 30 : 20)) + 1 : page}`
  }
}
export function decodeSearch(body, page, limit, platform = 'netease', more = false) {
  if (!Array.isArray(body) || body.length > limit || body.length > 100) fail('invalidData')
  const items = body.map((item) => {
    if (
      item.source !== platforms[platform] ||
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
      reference: { source: platform, kind: 'track', id: item.id },
      title: item.name,
      aliases: [],
      artist: item.artist.join(' / '),
      artists: item.artist.map((name) => ({ name, reference: null })),
      album: typeof item.album === 'string' ? item.album : '',
      albumReference: null,
      durationMs:
        Number.isFinite(item.extra_data?.duration) &&
        item.extra_data.duration > 0 &&
        item.extra_data.duration < 86400
          ? Math.round(item.extra_data.duration * 1000)
          : 0,
      cover: '',
      associations: []
    }
  })
  return {
    items,
    nextCursor: (more || items.length === limit) && page < 100 ? String(page + 1) : null
  }
}
