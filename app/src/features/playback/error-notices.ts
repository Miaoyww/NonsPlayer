export const NOTICE_DURATION_MS = 3000
const MAX_NOTICES = 4
export interface NoticeAction {
  label: string
  run: () => void | Promise<void>
}
interface Notice {
  id: number
  message: string
  actions?: readonly NoticeAction[]
  group?: string
}

export function createErrorNotices(
  schedule: (callback: () => void, delay: number) => unknown = (callback, delay) =>
    setTimeout(callback, delay),
  cancel: (timer: unknown) => void = (timer) => clearTimeout(timer as ReturnType<typeof setTimeout>)
) {
  let serial = 0
  let messages: readonly Notice[] = []
  const timers = new Map<number, unknown>()
  const listeners = new Set<() => void>()
  const publish = () => listeners.forEach((listener) => listener())
  function dismiss(id: number) {
    cancel(timers.get(id))
    timers.delete(id)
    messages = messages.filter((item) => item.id !== id)
    publish()
  }
  return {
    subscribe(listener: () => void) {
      listeners.add(listener)
      return () => {
        listeners.delete(listener)
      }
    },
    getSnapshot: () => messages,
    clearGroup(group: string) {
      for (const message of messages) if (message.group === group) dismiss(message.id)
    },
    push(message: string | null | undefined, actions?: readonly NoticeAction[], group?: string) {
      if (!message?.trim() || messages.some((item) => item.message === message)) return
      if (messages.length === MAX_NOTICES) dismiss(messages[0].id)
      const id = ++serial
      messages = [
        ...messages,
        { id, message, ...(actions?.length ? { actions } : {}), ...(group ? { group } : {}) }
      ]
      if (!actions?.length)
        timers.set(
          id,
          schedule(() => dismiss(id), NOTICE_DURATION_MS)
        )
      publish()
    },
    dismiss
  }
}

export const errorNotices = createErrorNotices()
