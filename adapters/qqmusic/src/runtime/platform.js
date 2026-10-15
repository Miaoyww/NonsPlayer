import { Buffer } from 'buffer/'
import {
  httpRequest,
  websocketOpen,
  websocketSend,
  websocketClose,
  nextEvent
} from 'nons:host/transport@1.0.0'
export { Buffer }
const NativeAbortSignal = Reflect.get(globalThis, 'AbortSignal')
const deadlines = new WeakMap()
export const HostAbortSignal = {
  timeout(milliseconds) {
    const signal = NativeAbortSignal.timeout(milliseconds)
    deadlines.set(signal, Date.now() + milliseconds)
    return signal
  },
  any(signals) {
    if (!Array.isArray(signals) || signals.length > 32) throw new TypeError('Invalid signal list')
    // The pinned engine exposes any(), but combines live signals as already aborted.
    // Compose their events explicitly and keep the earliest Host deadline.
    const controller = new AbortController()
    const signal = controller.signal
    const handlers = new Map()
    const cleanup = () => {
      for (const [source, handler] of handlers) source.removeEventListener('abort', handler)
      handlers.clear()
    }
    for (const source of signals) {
      if (source.aborted) {
        controller.abort(source.reason)
        cleanup()
        break
      }
      const handler = () => {
        controller.abort(source.reason)
        cleanup()
      }
      handlers.set(source, handler)
      source.addEventListener('abort', handler, { once: true })
    }
    deadlines.set(signal, Math.min(...signals.map((value) => deadlines.get(value) ?? Infinity)))
    return signal
  }
}
export async function hostFetch(input, options = {}) {
  const request = new Request(input, options)
  if (request.signal.aborted || options.signal?.aborted) throw new Error('Request aborted')
  const deadline = deadlines.get(options.signal)
  const timeoutMs = Math.max(
    1,
    Math.min(10000, Number.isFinite(deadline) ? deadline - Date.now() : 10000)
  )
  const headers = Array.from(request.headers, ([name, value]) => ({ name, value })).filter(
    // Request constructs Content-Length itself; transport framing belongs to the Host.
    (header) => !['host', 'content-length'].includes(header.name.toLowerCase())
  )
  const body =
    request.method === 'GET' || request.method === 'HEAD'
      ? new Uint8Array()
      : new Uint8Array(await request.arrayBuffer())
  const result = httpRequest({ url: request.url, method: request.method, headers, body, timeoutMs })
  const response = new Response(
    request.method === 'HEAD' || [204, 205, 304].includes(result.status) ? null : result.body,
    { status: result.status, headers: result.headers.map((header) => [header.name, header.value]) }
  )
  const cookies = result.headers
    .filter((header) => header.name.toLowerCase() === 'set-cookie')
    .map((header) => header.value)
  Object.defineProperty(response.headers, 'getSetCookie', { value: () => [...cookies] })
  return response
}
const sockets = new Map()
export const mqttConnect = async (url, protocol, handlers) => {
  const socket = websocketOpen(url, protocol, 10000)
  sockets.set(socket, handlers)
  return {
    send: (data) =>
      websocketSend(socket, new Uint8Array(data.buffer, data.byteOffset, data.byteLength)),
    close: () => {
      sockets.delete(socket)
      websocketClose(socket)
    }
  }
}
export function dispatch(event) {
  if (!event) return
  const message = event.tag ? event.val : event
  const socket = typeof message === 'number' ? message : message.socket
  const handlers = sockets.get(socket)
  if (!handlers) return
  if (event.tag === 'closed' || event.kind === 'closed') {
    sockets.delete(socket)
    handlers.close()
  } else handlers.message(event.tag ? message.data : Buffer.from(message.data, 'base64'))
}
export async function drive(promise) {
  let settled = false
  let value, failure
  promise.then(
    (result) => {
      value = result
      settled = true
    },
    (error) => {
      failure = error
      settled = true
    }
  )
  while (!settled) {
    dispatch(nextEvent(25))
    await new Promise((resolve) => setTimeout(resolve, 0))
  }
  if (failure) throw failure
  return value
}
export function closeSockets() {
  for (const socket of sockets.keys()) websocketClose(socket)
  sockets.clear()
}
