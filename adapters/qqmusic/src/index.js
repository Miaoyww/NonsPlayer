import manifest from '../manifest.json'
import { call as hostCall } from 'nons:plugin/host@1.0.0'
import { account, prepareLogin, event, shutdown as close } from './auth.js'
import { business, readTrack, resolvePlayback } from './catalog.js'
const codes = new Set([
  'unauthenticated',
  'notFound',
  'permissionDenied',
  'regionRestricted',
  'rateLimited',
  'network',
  'unsupported',
  'sourceUnavailable',
  'cancelled',
  'deadlineExceeded',
  'staleContext',
  'invalidData',
  'internal'
])
export function initialize() {}
export function shutdown() {
  close()
}
export async function call(method, args) {
  try {
    let result
    if (method === 'music.descriptor') result = manifest.descriptor
    else if (method === 'runtime:event') {
      await event(JSON.parse(args))
      result = null
    } else {
      const input = JSON.parse(args)
      if (input.session?.source !== 'qqmusic') throw { code: 'permissionDenied' }
      if (method === 'music.prepare-login') {
        await prepareLogin(input.request)
        result = null
      } else if (method === 'music.read-track') result = await readTrack(input.request)
      else if (method === 'music.business') result = await business(input.request)
      else if (method === 'music.account') result = await account(input.request)
      else if (method === 'music.resolve-playback') result = await resolvePlayback(input.request)
      else throw { code: 'unsupported' }
    }
    const reply = JSON.stringify({ status: 'ok', data: result })
    if (new TextEncoder().encode(reply).length > 65536) throw { code: 'invalidData' }
    return reply
  } catch (error) {
    try {
      hostCall(
        'diagnostic.report',
        JSON.stringify({
          stage: error?.diagnostics?.failureStage ?? method.replaceAll(':', '.'),
          class: String(error?.name ?? (typeof error === 'string' ? 'host' : 'unknown')),
          code: codes.has(error?.code)
            ? error.code
            : Number.isSafeInteger(error?.upstreamCode)
              ? `upstream-${error.upstreamCode}`
              : 'sourceUnavailable'
        })
      )
    } catch {}
    // Never return upstream payloads, credentials, URLs or exception messages.
    const code = codes.has(error?.code)
      ? error.code
      : codes.has(error?.payload)
        ? error.payload
        : error?.name === 'AuthCredentialRejectedError'
          ? 'unauthenticated'
          : error?.httpStatus === 429
            ? 'rateLimited'
            : 'sourceUnavailable'
    return JSON.stringify({
      status: 'error',
      data: {
        code,
        retryAfterMs: Number.isFinite(error?.retryAfterMs) ? Math.max(0, error.retryAfterMs) : null
      }
    })
  }
}
