export const NOTICE_DURATION_MS = 3000
const MAX_NOTICES = 4

export function createErrorNotices(
  schedule: (callback: () => void, delay: number) => unknown = (callback, delay) =>
    setTimeout(callback, delay),
  cancel: (timer: unknown) => void = (timer) => clearTimeout(timer as ReturnType<typeof setTimeout>)
) {
  let serial = 0
  let messages: readonly { id: number; message: string }[] = []
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
    push(message: string | null | undefined) {
      if (!message?.trim() || messages.some((item) => item.message === message)) return
      if (messages.length === MAX_NOTICES) dismiss(messages[0].id)
      const id = ++serial
      messages = [...messages, { id, message }]
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
