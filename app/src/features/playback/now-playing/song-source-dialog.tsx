import { reportCaughtError } from '@/lib/diagnostics'
import { ErrorNotice } from '@/components/error-notice'
import { useEffect, useMemo, useRef, useState } from 'react'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle
} from '@/components/ui/dialog'
import { ActionButton } from '@/components/music/action-button'
import { useMusicSources } from '@/features/music/use-music-sources'
import { providerSearchSources, useResourceProviders } from '@/features/music/resource-providers'
import { sourceLabel, trackSource } from '@/features/music/source-label'
import { SourceTrackSearch } from '@/features/search/source-track-search'
import { errorText, getPlayer, nativeCall, usePlayer, type Track } from '@/lib/player'
import { cn } from '@/lib/utils'
import { Radio, RotateCcw } from 'lucide-react'

export function SongSourceContent({
  track,
  onClose,
  className,
  dialog = false
}: {
  track: Track
  onClose: () => void
  className?: string
  dialog?: boolean
}) {
  const state = usePlayer('revision', 'playbackSource')
  const openingRevision = useRef(state.revision).current
  useEffect(() => {
    if (state.revision !== openingRevision) onClose()
  }, [state.revision, openingRevision, onClose])
  const { statuses } = useMusicSources()
  const { providers, error: providerError } = useResourceProviders()
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string>()
  const descriptors = statuses.map((s) => s.descriptor)
  const platform = trackSource(track)
  const sources = useMemo(
    () => [
      ...statuses
        .filter((s) => s.enabled && s.descriptor.capabilities.includes('search'))
        .map((s) => ({ source: s.descriptor.source, name: s.descriptor.displayName })),
      ...providerSearchSources(providers, statuses)
    ],
    [providers, statuses]
  )
  async function select(provider?: string, alternate?: Track) {
    const active = getPlayer()
    if (
      active.queue[active.index ?? -1]?.key !== track.key ||
      active.revision !== openingRevision
    ) {
      onClose()
      return
    }
    setBusy(true)
    setError(undefined)
    try {
      await nativeCall('player_select_source', {
        revision: state.revision,
        provider: provider ?? null,
        alternateKey: alternate?.key ?? null
      })
      onClose()
    } catch (cause) {
      reportCaughtError('app/src/features/playback/now-playing/song-source-dialog.tsx', cause)

      setError(errorText(cause))
    } finally {
      setBusy(false)
    }
  }
  return (
    <div className={cn('flex min-h-0 flex-1 flex-col gap-4 overflow-hidden', className)}>
      <div className="min-h-0 max-h-[48dvh] shrink-0 space-y-4 overflow-x-hidden overflow-y-auto overscroll-contain [overflow-wrap:anywhere]">
        {dialog ? (
          <DialogHeader>
            <DialogTitle>歌曲来源</DialogTitle>
            <DialogDescription>
              曲目平台：{sourceLabel(platform, descriptors)}
              。手动选择仅对当前队列条目有效，重启后恢复自动。
            </DialogDescription>
          </DialogHeader>
        ) : (
          <header className="space-y-2">
            <h2 className="text-lg font-semibold">歌曲来源</h2>
            <p className="text-sm text-muted-foreground">
              曲目平台：{sourceLabel(platform, descriptors)}
              。手动选择仅对当前队列条目有效，重启后恢复自动。
            </p>
          </header>
        )}
        {(error || providerError) && <ErrorNotice error={error ?? providerError} />}
        <section aria-label="切换当前曲目的音源">
          <div className="flex flex-wrap gap-2">
            <ActionButton
              variant="outline"
              className="source-panel-action"
              disabled={busy}
              onClick={() => void select()}
            >
              <RotateCcw aria-hidden="true" />
              自动选择
            </ActionButton>
            {statuses.some((s) => s.enabled && s.descriptor.source === platform) && (
              <ActionButton
                variant="outline"
                className="source-panel-action"
                data-current={state.playbackSource?.provider === 'official' || undefined}
                disabled={busy}
                onClick={() => void select('official')}
              >
                <Radio aria-hidden="true" />
                {sourceLabel(platform, descriptors)}官方
              </ActionButton>
            )}
            {providers
              .filter((p) => p.enabled && p.descriptor.platforms.includes(platform))
              .map((p) => (
                <ActionButton
                  key={p.descriptor.id}
                  variant={
                    state.playbackSource?.provider === p.descriptor.id ? 'secondary' : 'outline'
                  }
                  className="source-panel-action"
                  data-current={state.playbackSource?.provider === p.descriptor.id || undefined}
                  disabled={busy}
                  onClick={() => void select(p.descriptor.id)}
                >
                  <Radio aria-hidden="true" />
                  {p.descriptor.displayName}
                </ActionButton>
              ))}
            {statuses
              .filter(
                (s) =>
                  s.enabled &&
                  s.descriptor.source !== platform &&
                  s.descriptor.platforms?.includes(platform)
              )
              .map(({ descriptor }) => (
                <ActionButton
                  key={`platform/${descriptor.source}`}
                  variant={
                    state.playbackSource?.provider === descriptor.source ? 'secondary' : 'outline'
                  }
                  className="source-panel-action"
                  data-current={state.playbackSource?.provider === descriptor.source || undefined}
                  disabled={busy}
                  onClick={() => void select(descriptor.source)}
                >
                  <Radio aria-hidden="true" />
                  {descriptor.displayName}
                </ActionButton>
              ))}
          </div>
          <p className="mt-2 text-xs text-muted-foreground">
            获取失败会继续尝试其它已启用音源；切换同一曲目的音源保留播放进度。
          </p>
        </section>
      </div>
      <section className="flex min-h-0 min-w-0 flex-1 flex-col" aria-label="选择其它平台对应歌曲">
        <h3 className="mb-2 font-semibold">其它平台歌曲</h3>
        <p className="mb-3 text-xs text-muted-foreground">
          请核对歌手、专辑、版本与时长；选择后将替换本条目的播放音频，原队列曲目与收藏对象保留。
        </p>
        <SourceTrackSearch
          className="source-track-search"
          initialKeyword={`${track.title} ${track.artist}`.slice(0, 85)}
          sources={sources}
          busy={busy}
          onSelect={(alternate, provider) => void select(provider, alternate)}
        />
      </section>
    </div>
  )
}

export function SongSourceDialog({ track, onClose }: { track: Track; onClose: () => void }) {
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) onClose()
      }}
    >
      <DialogContent className="flex h-[90dvh] min-h-0 flex-col gap-4 overflow-hidden sm:max-w-3xl">
        <SongSourceContent track={track} onClose={onClose} dialog />
      </DialogContent>
    </Dialog>
  )
}
