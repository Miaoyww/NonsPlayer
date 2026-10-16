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
