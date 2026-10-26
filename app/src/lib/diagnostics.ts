import { invoke, isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

const MAX_MESSAGE_BYTES = 8192
const MAX_STACK_BYTES = 32 * 1024

function boundedUtf8(value: string, limit: number, suffix = '… [truncated]'): string {
  const encoder = new TextEncoder()
  const prefix = value.slice(0, limit)
  if (prefix.length === value.length && encoder.encode(prefix).length <= limit) return value
  const bytes = encoder.encode(prefix)
  const remaining = limit - encoder.encode(suffix).length
  let end = Math.min(remaining, bytes.length)
  while (end > 0 && (bytes[end] & 0xc0) === 0x80) end--
  return new TextDecoder().decode(bytes.subarray(0, end)) + suffix
}

export function redactDiagnostic(value: string): string {
  return boundedUtf8(
    value
      .slice(0, MAX_MESSAGE_BYTES)
      .replace(/https?:\/\/[^\s"'<>]+/gi, '<url>')
      .replace(/[\r\n\t]/g, ' ')
      .replace(/[\[{].*$/s, '<payload>')
      .replace(
        /((?:invalid (?:type|value): string|unknown variant|unknown field) )"(?:\\.|[^"\\])*(?:"|$)/g,
        '$1<redacted>'
      )
      .replace(/\b(cookie|authorization|bearer)\b\s+.*$/i, '$1=<redacted>')
      .replace(
        /\b(cookie|authorization|bearer|credential|password|token|secret|music_u|uin|skey|qqmusic_key)\b[\s"']*[:=].*$/i,
        '$1=<redacted>'
      ),
    MAX_MESSAGE_BYTES
  )
}

function sourceLocation(value: string): string {
  const match = value.match(/^(.*?)(:\d+(?::\d+)?)?$/)
  try {
    const source = new URL(match?.[1] ?? value)
    // Source paths and line/column numbers are useful; origins and query strings are not.
    return `source:${source.pathname}${match?.[2] ?? ''}`
  } catch {
    return redactDiagnostic(value)
  }
}

export function redactDiagnosticStack(value: string): string {
  const suffix = '\n… [stack truncated]'
  const bounded = boundedUtf8(value, MAX_STACK_BYTES * 4, suffix)
  const clean = bounded
    .split(/\r?\n/)
    .map((line) => {
      if (!/^\s*at\s|@(?:https?|tauri|asset):/.test(line)) return redactDiagnostic(line)
      return line
        .replace(/(?:https?|tauri|asset):\/\/[^\s"'<>\)]+/gi, sourceLocation)
        .replace(
          /\b(cookie|authorization|bearer|credential|password|token|secret|music_u|uin|skey|qqmusic_key)\b[\s"']*[:=].*$/i,
          '$1=<redacted>'
        )
        .replace(/[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f]/g, '')
    })
    .join('\n')
  return boundedUtf8(clean, MAX_STACK_BYTES, suffix)
}

type Diagnostic = {
  level: 'info' | 'warn' | 'error'
  command: string
  operation?: string
  source?: string
  code?: string
  stage?: string
  elapsedMs?: number
  message?: string
  stack?: string
  stackKind?: 'origin' | 'observation' | 'unavailable'
  location?: string
}
let pending = 0
const queue: Diagnostic[] = []
let dropped = 0
let sent = 0
let windowStart = Date.now()
let drainTimer: ReturnType<typeof setTimeout> | undefined

function drainDiagnostics() {
  if (Date.now() - windowStart >= 1000) {
    windowStart = Date.now()
    sent = 0
  }
  while (pending < 16 && (queue.length || dropped)) {
    // Leave capacity in the native 64/s budget for other WebViews.
    if (sent >= 48) {
      drainTimer ??= setTimeout(
        () => {
          drainTimer = undefined
          drainDiagnostics()
        },
        Math.max(1, 1000 - (Date.now() - windowStart))
      )
      return
    }
    const entry = queue.shift() ?? {
      level: 'warn',
      command: 'diagnostics',
      code: 'diagnosticOverflow',
      message: `dropped=${dropped}; diagnostic queue capacity exceeded`
    }
    if (entry.code === 'diagnosticOverflow') dropped = 0
    pending++
    sent++
    void invoke('diagnostic_report', { entry })
      .catch((cause) =>
        console.warn(
          '[NonsPlayer] 日志文件写入失败；诊断保留在控制台。',
          redactDiagnostic(errorFields(cause).message)
        )
      )
      .finally(() => {
        pending--
        drainDiagnostics()
      })
  }
}

export function reportDiagnostic(entry: Diagnostic) {
  const clean = {
    ...entry,
    command: redactDiagnostic(entry.command),
    operation: entry.operation && redactDiagnostic(entry.operation),
    source: entry.source && redactDiagnostic(entry.source),
    code: entry.code && redactDiagnostic(entry.code),
    stage: entry.stage && redactDiagnostic(entry.stage),
    message: entry.message && redactDiagnostic(entry.message),
    stack: entry.stack && redactDiagnosticStack(entry.stack),
    location: entry.location && redactDiagnostic(entry.location)
  }
  console[entry.level]('[NonsPlayer]', clean)
  if (!isTauri()) return
  if (queue.length < 48) queue.push(clean)
  else dropped++
  drainDiagnostics()
}

const expectedCodes = new Set([
  'staleContext',
  'cancelled',
  'cacheInvalidated',
  'writeRevisionChanged',
  'emptyLyrics'
])

const observationErrors = new WeakSet<object>()

function errorFields(cause: unknown) {
  const value =
    cause && typeof cause === 'object'
      ? (cause as {
          code?: unknown
          message?: unknown
          stack?: unknown
          stage?: unknown
          cause?: unknown
          stackKind?: unknown
        })
      : undefined
  const code = typeof value?.code === 'string' ? value.code : 'exception'
  const messages: string[] = []
  const stacks: string[] = []
  const seen = new Set<unknown>()
  let current = cause
  for (let depth = 0; current !== undefined && current !== null && depth < 8; depth++) {
    if (seen.has(current)) {
      messages.push('[cyclic cause]')
      break
    }
    seen.add(current)
    const error =
      typeof current === 'object'
        ? (current as { message?: unknown; stack?: unknown; cause?: unknown })
        : undefined
    messages.push(
      redactDiagnostic(
        typeof error?.message === 'string'
          ? error.message
          : typeof current === 'string'
            ? current
            : depth === 0
              ? code
              : 'Unknown cause'
      )
    )
    if (typeof error?.stack === 'string' && error.stack.trim()) {
      stacks.push(`${depth ? `Caused by (${depth}):\n` : ''}${error.stack}`)
    }
    current = error?.cause
    if (depth === 7 && current !== undefined) messages.push('[cause chain truncated]')
  }
  const originalStack = typeof value?.stack === 'string' && !!value.stack.trim()
  let stackKind: NonNullable<Diagnostic['stackKind']> =
    originalStack && !observationErrors.has(value!) ? 'origin' : 'observation'
  if (
    value?.stackKind === 'origin' ||
    value?.stackKind === 'observation' ||
    value?.stackKind === 'unavailable'
  ) {
    stackKind = value.stackKind
  }
  if (!originalStack || observationErrors.has(value!)) {
    const observation = originalStack
      ? (value!.stack as string)
      : new Error('Diagnostic observation').stack
    stacks.unshift(
      `Original stack unavailable; captured at diagnostic observation:\n${observation ?? 'Observation stack unavailable'}`
    )
    if (originalStack) stacks.splice(1, 1)
    if (!observation) stackKind = 'unavailable'
  }
  return {
    code,
    message: messages.join('; caused_by='),
    stage: typeof value?.stage === 'string' ? value.stage : undefined,
    stack: stacks.join('\n'),
    stackKind
  }
}

// Avoid duplicate reports when an IPC error crosses several recovery boundaries.
const reportedErrors = new WeakSet<object>()
export function reportCaughtError(operation: string, cause: unknown) {
  if (cause && typeof cause === 'object') {
    if (reportedErrors.has(cause)) return
    reportedErrors.add(cause)
  }
  const fields = errorFields(cause)
  reportDiagnostic({
    level: expectedCodes.has(fields.code) ? 'info' : 'error',
    command: 'handled_error',
    operation,
    ...fields
  })
}

// Records actual IPC failures before callers translate or handle them. Request bodies never enter logs.
export async function diagnosticInvoke<T>(
  command: string,
  args?: Record<string, unknown>
): Promise<T> {
  const started = performance.now()
  const request = args?.request as { operation?: unknown } | undefined
  const reference = args?.reference as { source?: unknown } | undefined
  const operation = typeof request?.operation === 'string' ? request.operation : command
  const source =
    typeof args?.source === 'string'
      ? args.source
      : typeof reference?.source === 'string'
        ? reference.source
        : undefined
  try {
    const result = await invoke<T>(command, args)
    if (
      [
        'music_use_source',
        'set_music_source_preferences',
        'music_select_account',
        'adapter_set_enabled',
        'adapter_reload'
      ].includes(command)
    )
      reportDiagnostic({
        level: 'info',
        command,
        operation,
        source: source ?? (typeof args?.current === 'string' ? args.current : undefined),
        elapsedMs: Math.round(performance.now() - started),
        code: 'ok'
      })
    return result
  } catch (cause) {
    // Native string rejections need one identity across cache and component recovery.
    const error = typeof cause === 'string' ? new Error(cause) : cause
    if (typeof cause === 'string') observationErrors.add(error as Error)
    if (error && typeof error === 'object') reportedErrors.add(error)
    const fields = errorFields(error)
    reportDiagnostic({
      level: expectedCodes.has(fields.code) ? 'info' : 'error',
      command,
      operation,
      source,
      elapsedMs: Math.round(performance.now() - started),
      ...fields
    })
    throw error
  }
}

export function installDiagnostics() {
  if (isTauri()) {
    void listen<{ level: number; message: string }>('log://log', ({ payload }) => {
      if (payload.message.includes(' nons::webview ')) return
      // Already sanitized by the native logger; never report back and create a logging loop.
      const level = payload.level >= 5 ? 'error' : payload.level === 4 ? 'warn' : 'info'
      console[level]('[NonsPlayer native]', payload.message)
    }).catch((cause) => reportCaughtError('diagnostics.subscribe', cause))
  }
  window.addEventListener('error', (event) => {
    const location = event.filename
      ? `${sourceLocation(event.filename)}:${event.lineno || 0}:${event.colno || 0}`
      : undefined
    reportDiagnostic({
      level: 'error',
      command: 'window.error',
      location,
      ...errorFields(event.error ?? event.message)
    })
  })
  window.addEventListener('unhandledrejection', (event) => {
    const fields = errorFields(event.reason)
    reportDiagnostic({
      level: expectedCodes.has(fields.code) ? 'info' : 'error',
      command: 'window.unhandledrejection',
      ...fields
    })
  })
}
