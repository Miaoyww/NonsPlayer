import { mkdir, writeFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import { platforms } from '../src/protocol.js'
import { sign } from '../src/signing.js'
import { inspectRemoteAudio } from '../src/audio.js'
import manifest from '../manifest.json' with { type: 'json' }

// Sequential, two samples per source, stop on rate limiting. No credentials,
// signed URLs or raw responses are persisted in the diagnostic report.
const keyword = process.argv[2] ?? 'Hello'
const selected = process.argv[3]?.split(',') ?? Object.keys(platforms)
if (selected.some((platform) => !Object.hasOwn(platforms, platform)))
  throw new Error('Unknown platform')
const output = resolve(process.argv[4] ?? '.local/gdstudio-expansion/probe.json')
const report = {
  startedAt: new Date().toISOString(),
  environment: `${process.platform}/${process.arch} ${process.version}`,
  keyword,
  results: []
}
let limited = false
function allowed(url) {
  return (
    ['http:', 'https:'].includes(url.protocol) &&
    !url.username &&
    !url.password &&
    !url.port &&
    manifest.permissions.httpHosts.some(
      (rule) =>
        rule === url.hostname || (rule.startsWith('*.') && url.hostname.endsWith(rule.slice(1)))
    )
  )
}
async function request(input, headers = {}, maximum = 65536) {
  const url = new URL(input)
  if (!allowed(url)) throw new Error('unauthorizedHost')
  const response = await fetch(url, {
    headers,
    redirect: 'manual',
    signal: AbortSignal.timeout(12000)
  })
  if (response.status === 429) limited = true
  const reader = response.body.getReader()
  const chunks = []
  let size = 0
  try {
    for (;;) {
      const { done, value } = await reader.read()
      if (done) break
      size += value.length
      if (size > maximum) throw new Error('responseTooLarge')
      chunks.push(value)
    }
  } finally {
    await reader.cancel()
  }
  return { status: response.status, headers: response.headers, bytes: Buffer.concat(chunks) }
}
async function api(type, platform, params) {
  const headers = {
    'User-Agent': 'Mozilla/5.0',
    Referer: 'https://music.gdstudio.org/',
    Origin: 'https://music.gdstudio.org',
    'X-Requested-With': 'XMLHttpRequest'
  }
  const time = await request('https://music.gdstudio.org/time', headers, 32)
  const url = new URL('https://music.gdstudio.org/api.php')
  url.search = new URLSearchParams({
    types: type,
    source: platforms[platform],
    ...params,
    s: sign(params.name ?? params.id, time.bytes.toString().trim(), 'music.gdstudio.org')
  })
  const response = await request(url, headers)
  let body
  try {
    body = JSON.parse(response.bytes.toString())
  } catch {}
  return { status: response.status, body }
}
async function inspect(input, duration) {
  let url = new URL(input, 'https://music.gdstudio.org/')
  const read = async (start, end) => {
    for (let hop = 0; hop <= 3; hop++) {
      const response = await request(url, { Range: `bytes=${start}-${end}` })
      if ([301, 302, 303, 307, 308].includes(response.status)) {
        url = new URL(response.headers.get('location'), url)
        continue
      }
      const range = /^bytes (\d+)-(\d+)\/(\d+)$/.exec(response.headers.get('content-range') ?? '')
      if (
        response.status !== 206 ||
        !range ||
        Number(range[1]) !== start ||
        Number(range[2]) !== Math.min(end, Number(range[3]) - 1) ||
        response.bytes.length !== Number(range[2]) - start + 1
      )
        throw new Error(`invalidRange:${response.status}`)
      return { bytes: response.bytes, size: Number(range[3]) }
    }
    throw new Error('tooManyRedirects')
  }
  const audio = await inspectRemoteAudio(read, duration, 'lossless', true)
  return { host: url.hostname, validation: 'fullAudioValidated', ...audio }
}
for (const platform of selected) {
  if (limited) break
  const started = performance.now()
  const result = { platform, samples: [] }
  try {
    const search = await api('search', platform, { name: keyword, count: '2', pages: '1' })
    result.searchStatus = search.status
    if (!Array.isArray(search.body)) result.error = search.body?.detail ?? 'invalidSearchResponse'
    else {
      result.count = search.body.length
      for (const item of search.body.slice(0, 2)) {
        if (limited) break
        const sample = { id: item.id, title: item.name, source: item.source }
        try {
          const resource = await api('url', platform, { id: String(item.id), br: '999' })
          sample.resolveStatus = resource.status
          sample.declaredBitrate = resource.body?.br
          sample.declaredSize = resource.body?.size
          if (
            !resource.body?.url ||
            Number(resource.body.br) === -1 ||
            Number(resource.body.size) === 0
          )
            sample.error = resource.body?.detail ?? 'emptyResource'
          else {
            let duration = Number(item.extra_data?.duration ?? 0) * 1000
            if (!duration && platform === 'bilibili') {
              const detail = await request(
                `https://api.bilibili.com/x/web-interface/view?bvid=${encodeURIComponent(item.id)}`
              )
              const info = JSON.parse(detail.bytes.toString())
              if (info.code === 0 && info.data?.bvid === item.id && info.data.videos === 1)
                duration = info.data.duration * 1000
            }
            sample.audio = await inspect(resource.body.url, duration)
          }
        } catch (error) {
          sample.error =
            error.code ??
            (['unauthorizedHost', 'responseTooLarge', 'tooManyRedirects'].includes(error.message) ||
            /^invalidRange:\d+$/.test(error.message)
              ? error.message
              : error.name)
          sample.diagnostic = error.diagnostic
        }
        result.samples.push(sample)
      }
    }
  } catch (error) {
    result.error = error.code ?? error.name
  }
  result.elapsedMs = Math.round(performance.now() - started)
  report.results.push(result)
  console.log(JSON.stringify(result))
}
report.rateLimited = limited
report.finishedAt = new Date().toISOString()
await mkdir(resolve(output, '..'), { recursive: true })
await writeFile(output, JSON.stringify(report, null, 2) + '\n')
console.log(`Saved report: ${output}`)
