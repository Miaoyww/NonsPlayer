import { ecb } from '@noble/ciphers/aes.js'
import { md5 } from '@noble/hashes/legacy.js'
import { bytesToHex } from '@noble/hashes/utils.js'
import type { PluginClient, PluginHttpResponse } from '@app/plugin-sdk'
import { reportCaughtError } from '@app/plugin-sdk'

const utf8 = new TextEncoder()
export const qualities = ['standard', 'higher', 'exhigh', 'lossless', 'hires'] as const
export type Quality = (typeof qualities)[number]
export const qualityLabels: Record<Quality, string> = {
  standard: '标准',
  higher: '较高',
  exhigh: '极高',
  lossless: '无损',
  hires: 'Hi-Res'
}
export class LoginRequired extends Error {}

// Protocol compatibility only: these legacy algorithms are required by NetEase EAPI.
// See the pinned ncm-api-rs src/crypto.rs and src/api/song_download_url_v1.rs.
export function encryptRequest(path: string, payload: Record<string, unknown>) {
  const text = JSON.stringify(payload)
  const digest = bytesToHex(md5(utf8.encode(`nobody${path}use${text}md5forencrypt`)))
  const data = `${path}-36cd479b6b5-${text}-36cd479b6b5-${digest}`
  return bytesToHex(ecb(utf8.encode('e82ckenh8dichen8')).encrypt(utf8.encode(data))).toUpperCase()
}
export function mergeCookies(existing: string, incoming: string[]) {
  const cookies = new Map(
    existing
      .split(';')
      .map((part) => part.trim())
      .filter(Boolean)
      .map((part) => {
        const i = part.indexOf('=')
        return [part.slice(0, i), part.slice(i + 1)]
      })
  )
  for (const line of incoming) {
    const pair = line.split(';')[0]
    const index = pair.indexOf('=')
    if (index < 1) continue
    const name = pair.slice(0, index).trim()
    const value = pair.slice(index + 1)
    if (!value || /max-age=0(?:;|$)/i.test(line)) cookies.delete(name)
    else cookies.set(name, value)
  }
  const value = [...cookies].map(([key, value]) => `${key}=${value}`).join('; ')
  if (value.length > 16000) throw new Error('账号凭据超过大小限制')
  return value
}
export async function api(
  client: PluginClient,
  path: string,
  data: Record<string, unknown>,
  cookie: string
) {
  const parsed = Object.fromEntries(
    cookie
      .split(';')
      .map((part) => part.trim())
      .filter(Boolean)
      .map((part) => {
        const i = part.indexOf('=')
        return [part.slice(0, i), part.slice(i + 1)]
      })
  )
  const header = {
    os: 'pc',
    appver: '3.1.3',
    osver: 'Microsoft-Windows-10',
    deviceId: 'nons-download-plugin',
    channel: 'netease',
    __csrf: parsed.__csrf || '',
    requestId: `${Date.now()}_${Math.floor(Math.random() * 1000)}`,
    ...(parsed.MUSIC_U ? { MUSIC_U: parsed.MUSIC_U } : {})
  }
  const params = encryptRequest(path, { ...data, header, e_r: false })
  const response = await client.call<PluginHttpResponse>('http.request', {
    url: `https://interface.music.163.com/eapi/${path.slice(5)}`,
    method: 'POST',
    headers: {
      'Content-Type': 'application/x-www-form-urlencoded',
      Cookie: Object.entries(header)
        .map(([k, v]) => `${k}=${encodeURIComponent(v)}`)
        .join('; '),
      'User-Agent': 'NeteaseMusic/3.1.3 (Windows; NonsPlayer download plugin)'
    },
    body: new URLSearchParams({ params }).toString()
  })
  if (response.status < 200 || response.status >= 300)
    throw new Error(`网易云请求失败（HTTP ${response.status}），请重试`)
  let body: Record<string, any>
  try {
    body = JSON.parse(response.body)
  } catch (cause) {
    reportCaughtError('netease-download.response.parse', cause)
    throw new Error('网易云返回了无效响应，请重试')
  }
  if (body.code === 301 || body.code === 302)
    throw new LoginRequired('下载账号已失效，请重新扫码登录')
  return { body, cookies: response.cookies ?? [] }
}
export function qualityCandidates(preferred: Quality, fallback: boolean, minimum: Quality) {
  const first = qualities.indexOf(preferred)
  const last = qualities.indexOf(minimum)
  if (first < 0 || last < 0 || last > first)
    throw new Error('最低接受音质不能高于默认音质，请修改插件配置')
  return fallback ? qualities.slice(last, first + 1).reverse() : [preferred]
}
export function audioFormat(bytes: Uint8Array): 'mp3' | 'flac' | 'm4a' | 'ogg' | 'wav' | undefined {
  const text = (start: number, count: number) =>
    String.fromCharCode(...bytes.slice(start, start + count))
  if (text(0, 4) === 'fLaC') return 'flac'
  if (
    text(0, 3) === 'ID3' ||
    (bytes[0] === 0xff && (bytes[1] & 0xe0) === 0xe0 && (bytes[1] & 6) !== 0)
  )
    return 'mp3'
  if (text(4, 4) === 'ftyp') return 'm4a'
  if (text(0, 4) === 'OggS') return 'ogg'
  if (text(0, 4) === 'RIFF' && text(8, 4) === 'WAVE') return 'wav'
}
export function filename(title: string, artist: string, id: number) {
  const name = `${artist} - ${title}`.replace(/[<>:"/\\|?*%\x00-\x1f]/g, '_').replace(/[. ]+$/, '')

  return `${Array.from(name).slice(0, 80).join('') || '歌曲'} [${id}]`
}
export async function resolveResource(
  client: PluginClient,
  id: number,
  preferred: Quality,
  fallback: boolean,
  minimum: Quality,
  cookie: string
) {
  for (const quality of qualityCandidates(preferred, fallback, minimum)) {
    const { body } = await api(
      client,
      '/api/song/enhance/download/url/v1',
      { id: String(id), immerseType: 'c51', level: quality },
      cookie
    )
    if (body.code !== 200)
      throw new Error(`网易云拒绝下载（${body.code ?? '未知状态'}），请检查账号或稍后重试`)
    const data = Array.isArray(body.data) ? body.data[0] : body.data
    if (!data?.url) {
      if (data?.code && data.code !== 200 && data.code !== 404)
        throw new Error(`该歌曲没有可用下载资源（${data.code}）`)
      continue
    }
    if (data.id !== undefined && Number(data.id) !== id) throw new Error('下载资源与所选歌曲不一致')
    if (data.freeTrialInfo) throw new Error('该资源为试听片段，不能保存为完整歌曲')
    const actual = data.level as Quality
    if (!qualities.includes(actual)) throw new Error('下载资源没有可识别的实际音质')
    if (
      (!fallback && actual !== preferred) ||
      qualities.indexOf(actual) < qualities.indexOf(minimum) ||
      qualities.indexOf(actual) > qualities.indexOf(preferred)
    )
      continue
    const url = new URL(data.url)
    if (!['http:', 'https:'].includes(url.protocol)) throw new Error('下载地址无效')
    const size = Number(data.size)
    if (!Number.isSafeInteger(size) || size <= 0 || size > 512 * 1024 * 1024)
      throw new Error('下载资源大小无效或超过 512MiB')
    return { url: url.href, quality: actual, size }
  }
  throw new Error('账号可下载的音质不符合当前设置，请检查音质或下载资格')
}
