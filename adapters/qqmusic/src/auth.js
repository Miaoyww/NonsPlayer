import { call as hostCall } from 'nons:plugin/host@1.0.0'
import {
  createQrLoginService,
  createRelayQqQrDriver,
  createMqttListenOver,
  isAuthSession,
  adapterRefresh
} from 'qq-protocol/auth/qrLogin.js'
import { createFetchAuthHttpClient } from 'qq-fetch-auth'
import { isAndroidDevice } from 'qq-protocol/auth/androidDevice.js'
import {
  credentialRead,
  credentialPublish,
  credentialReplace,
  credentialInvalidate,
  stateGet,
  stateSet
} from 'nons:host/transport@1.0.0'
import { mqttConnect, drive, dispatch, closeSockets } from './runtime/platform.js'
import { accountRecord, fail } from './model.js'

const encoder = new TextEncoder()
let issued = null
let restored
let currentKey = null
let currentMethod = null
let published = false
let device = null
const listeners = new Map()
const http = createFetchAuthHttpClient()
export const authHttp = http
export async function session() {
  if (issued) return issued
  if (restored !== undefined) return restored
  const bytes = credentialRead()
  if (!bytes.length) return (restored = null)
  try {
    restored = JSON.parse(new TextDecoder().decode(bytes))
  } catch {
    fail('invalidData')
  }
  if (!isAuthSession(restored)) fail('invalidData')
  return restored
}
const resolver = {
  mode: 'stored',
  resolve: async (token) => (token === 'current' ? session() : null),
  issue: async (value) => {
    issued = { ...value, token: 'current' }
    return 'current'
  },
  revoke: async () => {
    issued = null
    restored = null
  }
}
const deviceRepository = {
  kind: 'host',
  load() {
    const value = stateGet('device')
    if (!value) return null
    let parsed
    try {
      parsed = JSON.parse(value)
    } catch {
      fail('invalidData')
    }
    if (!isAndroidDevice(parsed)) fail('invalidData')
    device = parsed
    return parsed
  },
  save(value) {
    device = value
    // Session bootstrap keys are authentication material, not persistent device settings.
    const safe = Object.fromEntries(
      Object.entries(value).filter(([key]) => !key.startsWith('session'))
    )
    stateSet('device', JSON.stringify(safe))
  },
  clear() {
    device = null
  }
}
const listen = createMqttListenOver(mqttConnect)
const relay = {
  async open(id, image, ttl) {
    if (listeners.size) fail('rateLimited')
    const entry = { image, events: [], listener: null }
    listeners.set(id, entry)
    entry.listener = listen(
      id,
      (event) => {
        if (entry.events.length >= 32) {
          entry.listener?.close()
          fail('rateLimited')
        }
        entry.events.push(event)
      },
      ttl
    )
    entry.listener.done.catch(() => {
      entry.events = [{ type: 'loginFailed', payload: null }]
    })
    await entry.listener.ready
  },
  async poll(id) {
    const entry = listeners.get(id)
    return entry ? entry.events.splice(0) : [{ type: 'loginFailed', payload: null }]
  },
  async image(id) {
    return listeners.get(id)?.image ?? fail('notFound')
  },
  async close(id) {
    const entry = listeners.get(id)
    listeners.delete(id)
    entry?.listener.close()
  }
}
const service = createQrLoginService({
  http,
  createSessionHttp: () => createFetchAuthHttpClient(),
  deviceRepository,
  sessionResolver: resolver,
  capabilities: { channels: ['qq', 'wechat'], sessionMode: 'stored' },
  checkBudgetMs: 1000,
  drivers: { qq: createRelayQqQrDriver({ relay, http, device: () => device }) }
})
export const authService = service
const rejected = (error) => [1000, 104401, 104400].includes(error?.upstreamCode)
export async function authenticated(operation) {
  const auth = await session()
  if (!auth) fail('unauthenticated')
  if (auth.expiresAt > Date.now()) {
    try {
      return await drive(operation())
    } catch (error) {
      if (!rejected(error)) throw error
    }
  }
  let next
  try {
    next = await drive(adapterRefresh(http, auth))
  } catch (error) {
    if (rejected(error) || error?.name === 'AuthCredentialRejectedError') {
      credentialInvalidate()
      fail('unauthenticated')
    }
    throw error
  }
  if (
    !isAuthSession(next) ||
    accountRecord(null, next.credential).reference.id !==
      accountRecord(null, auth.credential).reference.id
  )
    fail('invalidData')
  credentialReplace(encoder.encode(JSON.stringify(next)))
  // A successful refresh advances the account generation. Re-read under its new scope.
  fail('staleContext')
}
export async function prepareLogin(request) {
  if (!['qq', 'wechat'].includes(request.method ?? 'qq')) fail('unsupported')
  if (currentKey) {
    if (currentMethod !== (request.method ?? 'qq')) fail('staleContext')
    return
  }
  currentMethod = request.method ?? 'qq'
  currentKey = await drive(service.createSession(currentMethod))
}
export async function account(request) {
  if (request.operation === 'beginLogin') {
    await prepareLogin(request)
    const image = await drive(service.createQr(currentKey))
    return { type: 'challenge', data: { key: currentKey, image } }
  }
  if (request.operation === 'pollLogin') {
    if (request.key !== currentKey) fail('staleContext')
    const progress = await drive(service.checkQr(currentKey, 1000))
    if (progress.code === 800) {
      try {
        hostCall(
          'diagnostic.report',
          JSON.stringify({
            stage: progress.failureStage ?? 'qr-event',
            class: progress.failureReason ?? 'qr-timeout',
            code: Number.isSafeInteger(progress.upstreamCode)
              ? `upstream-${progress.upstreamCode}`
              : 'sourceUnavailable'
          })
        )
      } catch {}
    }
    if (progress.code === 803 && !published) {
      if (!issued) fail('invalidData')
      const profile = await drive(service.getUserDetail('current'))
      credentialPublish(
        JSON.stringify(accountRecord(profile, issued.credential)),
        encoder.encode(JSON.stringify(issued))
      )
      published = true
    }
    return {
      type: 'progress',
      data: {
        code: progress.code,
        message:
          progress.code === 803
            ? '登录成功'
            : progress.code === 802
              ? '已扫码，请在手机确认'
              : progress.code === 800
                ? '二维码失效或登录失败'
                : '等待扫码'
      }
    }
  }
  if (request.operation === 'profile') {
    const auth = await session()
    if (!auth) return { type: 'profile', data: null }
    const profile = await authenticated(() => service.getUserDetail('current'))
    return { type: 'profile', data: profile ? accountRecord(profile, auth.credential) : null }
  }
  fail('unsupported')
}
export async function event(value) {
  dispatch(value)
  if (value.kind === 'tick' && currentMethod !== 'qq') {
    await new Promise((resolve) => setTimeout(resolve, 0))
    return
  }
  // Pull driver awaits credential exchange before returning from this short event call.
  if (currentKey) await drive(service.checkQr(currentKey, 0))
}
export function shutdown() {
  if (currentKey) service.cancelSession(currentKey)
  for (const entry of listeners.values()) entry.listener.close()
  listeners.clear()
  closeSockets()
  issued = null
  restored = null
  device = null
}
