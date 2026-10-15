import { invoke, isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

export function redactDiagnostic(value: string): string {
  return value
    .replace(/https?:\/\/[^\s"'<>]+/gi, '<url>')
    .replace(/[\r\n\t]/g, ' ')
    .replace(/[\[{].*$/s, '<payload>')
    .replace(
      /\b(cookie|authorization|bearer|credential|password|token|secret|music_u|uin|skey|qqmusic_key)\b[\s"':=]+.*$/i,
      '$1=<redacted>'
    )
    .slice(0, 2048)
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
}
let pending = 0
export function reportDiagnostic(entry: Diagnostic) {
  const clean = {
    ...entry,
    command: redactDiagnostic(entry.command),
    operation: entry.operation && redactDiagnostic(entry.operation),
    source: entry.source && redactDiagnostic(entry.source),
    code: entry.code && redactDiagnostic(entry.code),
    stage: entry.stage && redactDiagnostic(entry.stage),
    message: entry.message && redactDiagnostic(entry.message)
  }
  console[entry.level]('[NonsPlayer]', clean)
  if (!isTauri() || pending >= 16) return
  pending++
  void invoke('diagnostic_report', { entry: clean })
    .catch(() => console.warn('[NonsPlayer] 日志文件写入失败；诊断保留在控制台。'))
    .finally(() => {
      pending--
    })
}

function errorFields(cause: unknown) {
  const value =
    cause && typeof cause === 'object'
      ? (cause as { code?: unknown; message?: unknown; stack?: unknown; stage?: unknown })
      : undefined
  const code = typeof value?.code === 'string' ? value.code : 'exception'
  const message =
    typeof value?.stack === 'string'
      ? value.stack
      : typeof value?.message === 'string'
        ? value.message
        : typeof cause === 'string'
          ? cause
          : code
  return { code, message, stage: typeof value?.stage === 'string' ? value.stage : undefined }
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
    const fields = errorFields(cause)
    reportDiagnostic({
      level: ['staleContext', 'cancelled'].includes(fields.code) ? 'info' : 'error',
      command,
      operation,
      source,
      elapsedMs: Math.round(performance.now() - started),
      ...fields
    })
    throw cause
  }
}

export function installDiagnostics() {
  if (isTauri()) {
    void listen<{ level: number; message: string }>('log://log', ({ payload }) => {
      if (payload.message.includes(' nons::webview ')) return
      // Already sanitized by the native logger; never report back and create a logging loop.
      const level = payload.level >= 5 ? 'error' : payload.level === 4 ? 'warn' : 'info'
      console[level]('[NonsPlayer native]', payload.message)
    }).catch(() => console.warn('[NonsPlayer] 原生日志控制台订阅失败；请查看本机日志文件。'))
  }
  window.addEventListener('error', (event) => {
    reportDiagnostic({
      level: 'error',
      command: 'window.error',
      ...errorFields(event.error ?? event.message)
    })
  })
  window.addEventListener('unhandledrejection', (event) => {
    const fields = errorFields(event.reason)
    reportDiagnostic({
      level: ['staleContext', 'cancelled'].includes(fields.code) ? 'info' : 'error',
      command: 'window.unhandledrejection',
      ...fields
    })
  })
}
