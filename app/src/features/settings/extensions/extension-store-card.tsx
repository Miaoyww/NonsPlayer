import { useState, type ReactNode } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'
import { ArrowRight, Clock3, Download, UserRound } from 'lucide-react'
import githubIcon from '@/assets/icons/github.svg'
import { Button } from '@/components/ui/button'
import { ErrorNotice } from '@/components/error-notice'
import { reportCaughtError } from '@/lib/diagnostics'
import { errorText } from '@/lib/player'
import type { StoreEntry } from './extension-store-state'

function formatRelativeReleaseDate(value?: string) {
  if (!value) return undefined
  const timestamp = new Date(value).getTime()
  if (!Number.isFinite(timestamp)) return undefined

  const elapsedMinutes = Math.max(1, Math.floor((Date.now() - timestamp) / 60000))
  if (elapsedMinutes < 60) return `${elapsedMinutes}分钟前`

  const elapsedHours = Math.floor(elapsedMinutes / 60)
  if (elapsedHours < 24) return `${elapsedHours}小时前`

  const elapsedDays = Math.floor(elapsedHours / 24)
  if (elapsedDays < 30) return `${elapsedDays}天前`

  const elapsedMonths = Math.floor(elapsedDays / 30)
  if (elapsedMonths < 12) return `${elapsedMonths}个月前`

  return `${Math.floor(elapsedMonths / 12)}年前`
}

export function ExtensionStoreCard({
  entry,
  busy,
  onSelect,
  action
}: {
  entry: StoreEntry
  busy: boolean
  onSelect: () => void
  action: ReactNode
}) {
  const [linkError, setLinkError] = useState<string>()
  const authors = entry.authors?.map((author) => author.name).join('、') || '作者未提供'
  const releaseDate = formatRelativeReleaseDate(entry.release?.created_at)
  const count = entry.total_download_count
  const downloads =
    typeof count === 'number' && Number.isSafeInteger(count) && count >= 0 ? count : undefined
  const downloadText =
    downloads === undefined
      ? '暂无统计'
      : downloads >= 1000
        ? `${Math.round(downloads / 100) / 10}k`
        : String(downloads)
  const downloadLabel = `下载总量：${downloads === undefined ? '暂无统计' : downloads.toLocaleString('zh-CN')}`

  async function openRepository() {
    if (!entry.repository) return
    setLinkError(undefined)
    try {
      const url = new URL(entry.repository)
      if (url.protocol !== 'https:' && url.protocol !== 'http:')
        throw new Error('仓库链接须使用 HTTP 或 HTTPS')
      if (isTauri()) await openUrl(url.href)
      else window.open(url.href, '_blank', 'noopener,noreferrer')
    } catch (cause) {
      reportCaughtError('settings.extension-store.repository', cause)
      setLinkError(errorText(cause))
    }
  }

  return (
    <article className="min-w-0 space-y-2 rounded-xl border border-border/60 bg-card p-3 text-card-foreground">
      <div className="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-2">
        <div className="min-w-0 flex-1 basis-48 space-y-1">
          <h4>
            <Button
              variant="link"
              className="h-auto max-w-full justify-start rounded-sm p-0 text-left text-base leading-tight font-semibold whitespace-normal text-card-foreground [overflow-wrap:anywhere]"
              disabled={busy}
              onClick={onSelect}
            >
              {entry.name}
            </Button>
          </h4>
        </div>
        <div className="ml-auto min-w-0 max-w-full text-right">
          <div className="flex flex-wrap items-center justify-end gap-x-2 gap-y-1 text-sm leading-tight font-medium tabular-nums [overflow-wrap:anywhere]">
            {entry.installedVersion && entry.updateAvailable && (
              <>
                <span className="text-muted-foreground" aria-label="当前版本">
                  {entry.installedVersion}
                </span>
                <ArrowRight aria-hidden="true" className="size-4 shrink-0 text-muted-foreground" />
              </>
            )}
            <span aria-label={entry.updateAvailable ? '最新版本' : '版本'}>{entry.version}</span>
          </div>
        </div>
      </div>
      <p className="text-xs leading-normal break-words text-muted-foreground">
        {entry.description || '暂无描述'}
      </p>
      <div className="flex flex-wrap items-center justify-between gap-x-4 gap-y-3">
        <div className="flex min-w-0 flex-1 basis-56 flex-wrap items-center gap-x-3 gap-y-1.5 text-xs text-muted-foreground">
          <span className="inline-flex min-w-0 max-w-full items-center gap-1.5" title={authors}>
            <UserRound aria-hidden="true" className="size-4 shrink-0" />
            <span className="[overflow-wrap:anywhere]">{authors}</span>
          </span>
          <span className="inline-flex items-center gap-1.5" title="版本发布时间">
            <Clock3 aria-hidden="true" className="size-4 shrink-0" />
            {releaseDate ? (
              <time dateTime={entry.release?.created_at}>{releaseDate}</time>
            ) : (
              '发布时间未提供'
            )}
          </span>
          <span
            className="inline-flex items-center gap-1.5"
            title={downloadLabel}
            aria-label={downloadLabel}
          >
            <Download aria-hidden="true" className="size-4 shrink-0" />
            <span>{downloadText}</span>
          </span>
        </div>
        <div className="ml-auto flex min-w-0 max-w-full items-center gap-2">
          <span
            className="inline-flex min-h-8 min-w-0 items-center text-xs break-words text-muted-foreground"
            title="许可证"
          >
            {entry.license || '许可证未提供'}
          </span>
          {entry.repository && (
            <Button
              variant="ghost"
              size="icon-sm"
              className="rounded-lg"
              aria-label={`打开 ${entry.name} 的代码仓库`}
              title={entry.repository}
              onClick={() => void openRepository()}
            >
              <span
                aria-hidden="true"
                className="size-4 shrink-0 bg-current"
                style={{
                  mask: `url("${githubIcon}") center / contain no-repeat`,
                  WebkitMask: `url("${githubIcon}") center / contain no-repeat`
                }}
              />
            </Button>
          )}
          {action}
        </div>
      </div>
      {linkError && <ErrorNotice error={linkError} />}
    </article>
  )
}
