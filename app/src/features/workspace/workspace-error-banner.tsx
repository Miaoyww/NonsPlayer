import { useEffect, useState } from 'react'
import { X } from 'lucide-react'
import { ActionButton } from '@/components/music/action-button'

export function WorkspaceErrorBanner({
  errors,
  onDismiss
}: {
  errors: (string | null | undefined)[]
  onDismiss: () => void
}) {
  const messages = [...new Set(errors.filter((value): value is string => !!value))]
  const fingerprint = JSON.stringify(messages)
  const [dismissed, setDismissed] = useState<string[]>([])
  useEffect(() => {
    const current: string[] = JSON.parse(fingerprint)
    setDismissed((previous) => previous.filter((message) => current.includes(message)))
  }, [fingerprint])
  const message = messages.find((value) => !dismissed.includes(value))
  if (!message) return null
  return (
    <div
      role="alert"
      className="flex items-start gap-3 border-b border-border bg-destructive/5 px-8 py-3 text-sm text-destructive"
    >
      <p className="min-w-0 flex-1">{message}</p>
      <ActionButton
        variant="ghost"
        size="icon-sm"
        aria-label="关闭提示"
        onClick={() => {
          setDismissed((previous) => [...new Set([...previous, ...messages])])
          onDismiss()
        }}
      >
        <X aria-hidden="true" />
      </ActionButton>
    </div>
  )
}
