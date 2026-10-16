import { reportCaughtError } from '@/lib/diagnostics'
import { ErrorNotice } from '@/components/error-notice'
import { useEffect, useRef, useState } from 'react'
import { Check, RefreshCw, Trash2 } from 'lucide-react'
import { AccountAvatar } from './account-avatar'
import { ActionButton } from '@/components/music/action-button'
import { musicClient } from '@/features/music/client'
import { sourcePreferences } from '@/features/music/source-preferences'
import type { AccountRecord, AccountRef } from '@/features/music/types'
import { errorText } from '@/lib/player'

export function SavedAccounts({
  open,
  source = '',
  sourceName = '',
  sources,
  switchPageSource = true,
  current,
  showSeparator = true,
  enabled = true,
  onChanged
}: {
  open: boolean
  source?: string
  sourceName?: string
  sources?: { source: string; name: string; current?: AccountRef; enabled: boolean }[]
  switchPageSource?: boolean
  current?: AccountRef
  showSeparator?: boolean
  enabled?: boolean
  onChanged?: () => void
}) {
  const entries = sources ?? [{ source, name: sourceName, current, enabled }]
  const scope = JSON.stringify(entries)
  const sourcesKey = entries.map((item) => item.source).join('|')
  const [records, setRecords] = useState<AccountRecord[]>([])
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)
  const [removing, setRemoving] = useState<string>()
  const [revision, setRevision] = useState(0)
  const generation = useRef(0)
  const operationGeneration = useRef(0)
  useEffect(() => {
    operationGeneration.current++
    setError('')
    return () => {
      operationGeneration.current++
    }
  }, [open, sourcesKey])
  useEffect(() => {
    const serial = ++generation.current
    if (!open) return
    setBusy(true)
    setRemoving(undefined)
    setRecords([])
    void Promise.all(entries.map((entry) => musicClient.accounts(entry.source)))
      .then((groups) => groups.flat())
      .then((result) => {
        if (serial === generation.current) setRecords(result)
      })
      .catch((cause) => {
        reportCaughtError('app/src/features/account/saved-accounts.tsx', cause)

        if (serial === generation.current) setError(errorText(cause))
      })
      .finally(() => {
        if (serial === generation.current) setBusy(false)
      })
    return () => {
      generation.current++
    }
  }, [open, scope, revision])
  async function change(record: AccountRecord, remove = false) {
    const serial = operationGeneration.current
    setBusy(true)
    setError('')
    try {
      if (remove) await musicClient.removeAccount(record.reference)
      else {
        const intent = switchPageSource ? await sourcePreferences.beginIntent() : undefined
        await musicClient.selectAccount(record.reference)
        try {
          if (intent !== undefined) await sourcePreferences.use(record.reference.source, intent)
        } catch (cause) {
          reportCaughtError('app/src/features/account/saved-accounts.tsx', cause)

          throw new Error(`账号选择成功，切换来源失败：${errorText(cause)}。可点击账号重试。`)
        }
      }
      if (serial === operationGeneration.current) {
        setRemoving(undefined)
        setRevision((value) => value + 1)
        onChanged?.()
      }
    } catch (cause) {
      reportCaughtError('app/src/features/account/saved-accounts.tsx', cause)

      if (serial === operationGeneration.current) setError(errorText(cause))
    } finally {
      if (serial === operationGeneration.current) setBusy(false)
    }
  }
  if (!open) return null
  return (
    <section
      aria-label={`保存的${sourceName}账号`}
      className={showSeparator ? 'mt-4 border-t border-border pt-3' : undefined}
    >
      <div className="flex items-center justify-between gap-2">
        <p className="text-xs text-muted-foreground">保存的账号</p>
        <ActionButton
          variant="ghost"
          size="icon-sm"
          aria-label="刷新保存的账号"
          disabled={busy}
          onClick={() => setRevision((value) => value + 1)}
        >
          <RefreshCw aria-hidden="true" />
        </ActionButton>
      </div>
      {busy && (
        <p role="status" className="text-sm text-muted-foreground">
          正在处理账号…
        </p>
      )}
      {error && <ErrorNotice error={error} />}
      <div className="max-h-56 overflow-y-auto">
        {records.map((record) => {
          const entry = entries.find((item) => item.source === record.reference.source) ?? {
            name: record.reference.source,
            enabled: false,
            current: undefined
          }
          const selected =
            record.reference.source === entry.current?.source &&
            record.reference.id === entry.current?.id
          return (
            <div key={`${record.reference.source}:${record.reference.id}`} className="mt-2">
              <div className="flex items-center gap-2">
                <ActionButton
                  variant="ghost"
                  className="min-w-0 flex-1 justify-start"
                  disabled={busy || !entry.enabled}
                  aria-pressed={selected}
                  onClick={() => void change(record)}
                >
                  <span className="relative shrink-0">
                    <AccountAvatar avatar={record.avatar} />
                    {selected && (
                      <Check
                        className="absolute -right-1 -bottom-1 size-3 rounded-full bg-background text-primary"
                        aria-hidden="true"
                      />
                    )}
                  </span>
                  <span className="truncate">{record.displayName}</span>
                  <span className="ml-auto shrink-0 text-xs text-muted-foreground">
                    {entry.name}
                  </span>
                </ActionButton>
                <ActionButton
                  variant="ghost"
                  size="icon-sm"
                  aria-label={`删除保存的账号 ${record.displayName}`}
                  disabled={busy}
                  onClick={() => setRemoving(`${record.reference.source}:${record.reference.id}`)}
                >
                  <Trash2 aria-hidden="true" />
                </ActionButton>
              </div>
              {removing === `${record.reference.source}:${record.reference.id}` && (
                <div className="rounded-md bg-muted p-2 text-sm">
                  <p>删除保存的账号及登录凭据？</p>
                  <div className="mt-2 flex justify-end gap-2">
                    <ActionButton
                      variant="ghost"
                      size="sm"
                      disabled={busy}
                      onClick={() => setRemoving(undefined)}
                    >
                      取消
                    </ActionButton>
                    <ActionButton
                      variant="outline"
                      size="sm"
                      disabled={busy}
                      onClick={() => void change(record, true)}
                    >
                      确定删除
                    </ActionButton>
                  </div>
                </div>
              )}
            </div>
          )
        })}
      </div>
      {!busy && !records.length && !error && (
        <p className="mt-2 text-sm text-muted-foreground">暂无保存的账号。</p>
      )}
    </section>
  )
}
