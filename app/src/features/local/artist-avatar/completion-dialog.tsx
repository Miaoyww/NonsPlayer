import { useCallback, useEffect, useRef, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { ImagePlus, ListChecks, Search, Sparkles } from 'lucide-react'
import { ActionButton } from '@/components/music/action-button'
import { Cover } from '@/components/music/cover'
import { ErrorNotice } from '@/components/error-notice'
import { InfiniteLoad } from '@/components/music/infinite-load'
import { Progress } from '@/components/ui/progress'
import { Spinner } from '@/components/ui/spinner'
import { Input } from '@/components/ui/input'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle
} from '@/components/ui/dialog'
import { usePagedList } from '@/lib/use-paged-list'
import { errorText, nativeCall } from '@/lib/player'
import { reportCaughtError } from '@/lib/diagnostics'
import { invalidateNativeCache } from '@/lib/runtime-cache'
import { useMusicSources } from '@/features/music/use-music-sources'
import { useArtistAvatarActions } from './actions'
import {
  ARTIST_AVATAR_SOURCE,
  artistAvatarStatusLabel,
  artistAvatarStatusTone,
  type ArtistAvatarItem,
  type ArtistAvatarPage,
  type ArtistAvatarProgress
} from './types'

export function ArtistAvatarCompletionButton({
  onError,
  onNotice
}: {
  onError: (cause: unknown) => void
  onNotice: (message: string) => void
}) {
  const [open, setOpen] = useState(false)
  return (
    <>
      <ActionButton variant="outline" disabled={!isTauri()} onClick={() => setOpen(true)}>
        <ImagePlus aria-hidden="true" />
        补齐头像
      </ActionButton>
      <ArtistAvatarCompletionDialog
        open={open}
        onOpenChange={setOpen}
        onError={onError}
        onNotice={onNotice}
      />
    </>
  )
}

function ArtistAvatarCompletionDialog({
  open,
  onOpenChange,
  onError,
  onNotice
}: {
  open: boolean
  onOpenChange: (open: boolean) => void
  onError: (cause: unknown) => void
  onNotice: (message: string) => void
}) {
  const actions = useArtistAvatarActions()
  const { statuses } = useMusicSources()
  const [input, setInput] = useState('')
  const [keyword, setKeyword] = useState('')
  const [revision, setRevision] = useState(0)
  const [progress, setProgress] = useState<ArtistAvatarProgress>()
  const [starting, setStarting] = useState(false)
  const [error, setError] = useState<string>()
  const [working, setWorking] = useState<string>()
  const generation = useRef(0)

  const loader = useCallback(
    async (offset: number) => {
      const page = await nativeCall<ArtistAvatarPage>('local_artist_avatars', {
        keyword,
        offset
      })
      return {
        items: page.items,
        more: page.more,
        metadata: page.total
      }
    },
    [keyword]
  )
  const list = usePagedList<ArtistAvatarItem, number>(loader, 30, open, {
    refreshKey: revision
  })
  const total = list.metadata ?? 0
  const running = starting || !!progress?.running

  useEffect(() => {
    if (!open) return
    const current = ++generation.current
    setError(undefined)
    setInput(keyword)
    let stopProgress: (() => void) | undefined
    let stopLibrary: (() => void) | undefined
    let disposed = false
    void (async () => {
      try {
        const [unlistenProgress, unlistenLibrary] = await Promise.all([
          listen<ArtistAvatarProgress>('local-artist-avatar-progress', ({ payload }) => {
            if (!disposed && current === generation.current) {
              setProgress(payload)
              if (!payload.running) setRevision((value) => value + 1)
            }
          }),
          listen('local-library-updated', () => {
            if (!disposed && current === generation.current) setRevision((value) => value + 1)
          })
        ])
        if (disposed || current !== generation.current) {
          unlistenProgress()
          unlistenLibrary()
          return
        }
        stopProgress = unlistenProgress
        stopLibrary = unlistenLibrary
        try {
          const status = await nativeCall<ArtistAvatarProgress>('local_artist_avatar_status')
          if (!disposed && current === generation.current) setProgress(status)
        } catch (cause) {
          reportCaughtError('app/src/features/local/artist-avatar/completion-dialog.tsx', cause)
          if (!disposed && current === generation.current) setError(errorText(cause))
        }
      } catch (cause) {
        reportCaughtError('app/src/features/local/artist-avatar/completion-dialog.tsx', cause)
        if (!disposed && current === generation.current) setError(errorText(cause))
      }
    })()
    return () => {
      disposed = true
      generation.current++
      stopProgress?.()
      stopLibrary?.()
    }
  }, [open])

  async function start(artistIds: string[], all: boolean, force = false) {
    if (running) return
    setStarting(true)
    setError(undefined)
    try {
      const value = await nativeCall<ArtistAvatarProgress>('local_artist_avatar_start', {
        artistIds,
        all,
        keyword: all ? keyword : undefined,
        source: ARTIST_AVATAR_SOURCE,
        force
      })
      setProgress(value)
      onNotice(all ? '已开始补齐艺术家头像。' : '已开始匹配艺术家头像。')
      invalidateNativeCache(['local_entities', 'local_entity_detail'])
    } catch (cause) {
      reportCaughtError('app/src/features/local/artist-avatar/completion-dialog.tsx', cause)
      setError(errorText(cause))
      onError(cause)
    } finally {
      setStarting(false)
    }
  }

  async function one(item: ArtistAvatarItem) {
    if (working || running) return
    setWorking(item.id)
    try {
      if (item.status === 'default') {
        await actions.rematch(item)
      } else {
        await start([item.id], false)
      }
    } finally {
      setWorking(undefined)
    }
  }

  function sourceName(source?: string) {
    if (!source) return ''
    return (
      statuses.find((status) => status.descriptor.source === source)?.descriptor.displayName ??
      source
    )
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[85dvh] overflow-y-auto sm:max-w-3xl">
        <DialogHeader>
          <DialogTitle>补齐艺术家头像</DialogTitle>
          <DialogDescription>
            从音乐来源匹配本地艺术家头像。自动匹配在后台执行，不阻塞扫描、页面浏览或播放；手动选择的结果不会被自动任务覆盖。
          </DialogDescription>
        </DialogHeader>
        <div className="flex flex-wrap items-center gap-2">
          <form
            className="flex min-w-48 flex-1 gap-2"
            onSubmit={(event) => {
              event.preventDefault()
              setKeyword(input.trim())
              setRevision((value) => value + 1)
            }}
          >
            <Input
              className="flex-1"
              value={input}
              maxLength={100}
              aria-label="搜索本地艺术家"
              placeholder="搜索本地艺术家"
              onChange={(event) => setInput(event.target.value)}
            />
            <ActionButton type="submit" variant="outline" disabled={!isTauri() || running}>
              <Search aria-hidden="true" />
              搜索
            </ActionButton>
          </form>
          <ActionButton
            disabled={!isTauri() || running || total === 0}
            onClick={() => void start([], true)}
          >
            {running ? <Spinner /> : <ListChecks aria-hidden="true" />}
            {running ? '正在补齐…' : keyword ? '补齐筛选结果' : '全部补齐'}
          </ActionButton>
        </div>
        <div className="flex flex-col gap-2">
          <p className="text-xs text-muted-foreground" role="status">
            {keyword ? `筛选「${keyword}」 · ` : ''}共 {total} 位本地艺术家
          </p>
          {progress && (progress.running || progress.processed > 0 || progress.error) && (
            <div className="rounded-2xl bg-muted/40 p-3">
              <Progress
                value={
                  progress.total > 0 ? Math.round((progress.processed / progress.total) * 100) : 0
                }
              >
                <span className="text-sm font-medium">
                  {progress.running ? '正在匹配' : '匹配完成'}
                </span>
                <span className="ml-auto text-xs text-muted-foreground tabular-nums">
                  {progress.processed} / {progress.total}
                </span>
              </Progress>
              <p className="mt-2 text-xs text-muted-foreground">
                自动匹配 {progress.matched} · 待确认 {progress.pending} · 未找到 {progress.notFound}{' '}
                · 失败 {progress.failed}
                {progress.skipped > 0 ? ` · 跳过 ${progress.skipped}` : ''}
              </p>
              {progress.current && progress.running && (
                <p className="mt-1 truncate text-xs text-muted-foreground">
                  正在处理：{progress.current}
                </p>
              )}
              {progress.error && <ErrorNotice error={progress.error} />}
            </div>
          )}
        </div>
        <div className="max-h-[48dvh] min-h-32 overflow-y-auto pr-1">
          {list.busy && !list.items.length ? (
            <div className="flex items-center justify-center py-10 text-muted-foreground">
              <Spinner className="size-5" />
            </div>
          ) : list.items.length ? (
            <div className="flex flex-col gap-1">
              {list.items.map((item) => (
                <div
                  key={item.id}
                  className="flex items-center gap-3 rounded-2xl px-2 py-2 transition-colors hover:bg-muted/60"
                >
                  <Cover cover={item.cover} className="size-12 rounded-full" loading="lazy" />
                  <div className="min-w-0 flex-1">
                    <p className="truncate font-medium">{item.name}</p>
                    <p
                      className={`truncate text-xs ${artistAvatarStatusTone(item.status)}`}
                      title={item.error}
                    >
                      {artistAvatarStatusLabel(item.status)}
                      {item.source && ` · ${sourceName(item.source)}`}
                      {item.width && item.height && ` · ${item.width}×${item.height}`}
                      {item.error && ` · ${item.error}`}
                    </p>
                  </div>
                  <div className="flex shrink-0 items-center gap-1">
                    {item.status === 'auto' || item.status === 'manual' ? (
                      <ActionButton
                        size="sm"
                        variant="outline"
                        disabled={running || !!working}
                        onClick={() => actions.openCandidates(item)}
                      >
                        <ImagePlus aria-hidden="true" />
                        更换
                      </ActionButton>
                    ) : (
                      <>
                        <ActionButton
                          size="sm"
                          variant="outline"
                          disabled={!isTauri() || running || !!working}
                          onClick={() => void one(item)}
                        >
                          {working === item.id ? (
                            <Spinner />
                          ) : item.status === 'default' ? (
                            <Sparkles aria-hidden="true" />
                          ) : (
                            <ImagePlus aria-hidden="true" />
                          )}
                          {item.status === 'default' ? '重新匹配' : '补齐'}
                        </ActionButton>
                        <ActionButton
                          size="sm"
                          variant="ghost"
                          disabled={running || !!working}
                          onClick={() => actions.openCandidates(item)}
                        >
                          选择
                        </ActionButton>
                      </>
                    )}
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <p className="py-10 text-center text-sm text-muted-foreground">
              {list.error ? '读取失败。' : '没有找到本地艺术家。'}
            </p>
          )}
          <InfiniteLoad
            more={list.more}
            busy={list.busy}
            error={list.error}
            onLoad={list.loadMore}
          />
        </div>
        {(error || list.error) && <ErrorNotice error={error ?? list.error} />}
        <DialogFooter>
          <ActionButton variant="outline" disabled={running} onClick={() => onOpenChange(false)}>
            关闭
          </ActionButton>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
