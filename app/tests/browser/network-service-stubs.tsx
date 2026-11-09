type OpenApiConfig = {
  enabled: boolean
  port: number
  lan: boolean
}

type OpenApiStatus = {
  config: OpenApiConfig
  running: boolean
  addresses: string[]
  connections: number
  error: string | null
}

type EventHandler<T> = (event: { payload: T }) => void

let current: OpenApiStatus = {
  config: { enabled: true, port: 46321, lan: false },
  running: true,
  addresses: ['127.0.0.1:46321'],
  connections: 0,
  error: null
}

export const calls: [string, unknown?][] = []
export const copied: string[] = []
export const opened: string[] = []
const listeners = new Map<string, Set<EventHandler<unknown>>>()

export const isTauri = () => true
export const invoke = async () => undefined
export const errorText = (error: unknown) =>
  error instanceof Error ? error.message : String(error)
export const writeText = async (value: string) => {
  copied.push(value)
}
export const openUrl = async (value: string) => {
  opened.push(value)
}

export async function listen<T>(name: string, handler: EventHandler<T>) {
  const entries = listeners.get(name) ?? new Set()
  entries.add(handler as EventHandler<unknown>)
  listeners.set(name, entries)
  return () => entries.delete(handler as EventHandler<unknown>)
}

export async function nativeCall<T>(command: string, args?: { config: OpenApiConfig }) {
  calls.push([command, args])
  if (command === 'open_api_status') return current as T
  if (command === 'open_api_configure') {
    current = { ...current, config: args!.config }
    return current as T
  }
  throw new Error(`Unexpected command: ${command}`)
}

export function emitStatus(patch: Partial<OpenApiStatus>) {
  current = { ...current, ...patch }
  for (const listener of listeners.get('open-api-status-changed') ?? [])
    listener({ payload: current })
}
