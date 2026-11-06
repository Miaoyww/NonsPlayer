import { useEffect, useRef, useState } from 'react'
import { Search, Sparkles } from 'lucide-react'
import { isTauri } from '@tauri-apps/api/core'
import { Cover } from '@/components/music/cover'
import { ActionButton } from '@/components/music/action-button'
import { ErrorNotice } from '@/components/error-notice'
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
import { invalidateNativeCache } from '@/lib/runtime-cache'
import { errorText, nativeCall } from '@/lib/player'
import { reportCaughtError } from '@/lib/diagnostics'
import { ARTIST_AVATAR_SOURCE, type ArtistAvatarCandidate } from './types'

export function ArtistAvatarCandidateDialog({
  target,
  onClose,
  onError,
  onNotice
}: {
  target?: { id: string; name: string }
  onClose: () => void
  onError: (cause: unknown) => void
  onNotice: (message: string) => void
}) {
  const [keyword, setKeyword] = useState('')
  const [candidates, setCandidates] = useState<ArtistAvatarCandidate[]>([])
  const [searching, setSearching] = useState(false)
  const [saving, setSaving] = useState<string>()
  const [error, setError] = useState<string>()
  const generation = useRef(0)

  useEffect(() => {
    const current = ++generation.current
    setCandidates([])
    setError(undefined)
    setSaving(undefined)
    if (!target) {
      setSearching(false)
      return
    }
    setKeyword(target.name)
    setSearching(true)
    void nativeCall<ArtistAvatarCandidate[]>('local_artist_avatar_candidates', {
      keyword: target.name,
      source: ARTIST_AVATAR_SOURCE
    })
      .then((items) => {
        if (current === generation.current) setCandidates(items)
      })
      .catch((cause) => {
        reportCaughtError('app/src/features/local/artist-avatar/candidate-dialog.tsx', cause)
        if (current === generation.current) setError(errorText(cause))
      })
      .finally(() => {
        if (current === generation.current) setSearching(false)
      })
    return () => {
      generation.current++
    }
  }, [target?.id, target?.name])

  async function search() {
    const value = keyword.trim()
    if (!target || !value || searching || saving) return
    const current = ++generation.current
    setSearching(true)
    setError(undefined)
    try {
      const items = await nativeCall<ArtistAvatarCandidate[]>('local_artist_avatar_candidates', {
        keyword: value,
        source: ARTIST_AVATAR_SOURCE
      })
      if (current === generation.current) setCandidates(items)
    } catch (cause) {
      reportCaughtError('app/src/features/local/artist-avatar/candidate-dialog.tsx', cause)
      if (current === generation.current) setError(errorText(cause))
    } finally {
      if (current === generation.current) setSearching(false)
    }
  }

  async function select(candidate: ArtistAvatarCandidate) {
    if (!target || saving) return
    setSaving(candidate.platformId)
    setError(undefined)
    try {
      await nativeCall('local_artist_avatar_select', {
        artistId: target.id,
        source: candidate.source,
        platformId: candidate.platformId
      })
      invalidateNativeCache(
        ['local_entities', 'local_entity_detail'],
        'refresh',
        'nativeCall.local_artist_avatar_select'
      )
      onNotice(`已为「${target.name}」保存艺术家头像。`)
      onClose()
    } catch (cause) {
      reportCaughtError('app/src/features/local/artist-avatar/candidate-dialog.tsx', cause)
      onError(cause)
      setError(errorText(cause))
    } finally {
      setSaving(undefined)
    }
  }

  return (
    <Dialog open={!!target} onOpenChange={(open) => !open && !saving && onClose()}>
      <DialogContent
        className="max-h-[85dvh] overflow-y-auto sm:max-w-2xl"
        showCloseButton={!saving}
      >
        <DialogHeader>
          <DialogTitle>更换艺术家头像</DialogTitle>
          <DialogDescription>
            {target
              ? `为「${target.name}」选择来源中的艺术家头像。手动选择后会锁定，自动补齐不会覆盖。`
              : '选择来源中的艺术家头像。'}
          </DialogDescription>
        </DialogHeader>
        <form
          className="flex gap-2"
          onSubmit={(event) => {
            event.preventDefault()
            void search()
          }}
        >
          <Input
            className="flex-1"
            value={keyword}
            maxLength={100}
            disabled={searching || !!saving}
            aria-label="搜索艺术家"
            placeholder="搜索艺术家名称"
            onChange={(event) => setKeyword(event.target.value)}
          />
          <ActionButton
            type="submit"
            variant="outline"
            disabled={!isTauri() || !keyword.trim() || searching || !!saving}
          >
            {searching ? <Spinner /> : <Search aria-hidden="true" />}
            搜索
          </ActionButton>
        </form>
        <div className="max-h-[48dvh] min-h-28 overflow-y-auto pr-1">
          {searching && !candidates.length ? (
            <div className="flex items-center justify-center py-10 text-muted-foreground">
              <Spinner className="size-5" />
            </div>
          ) : candidates.length ? (
            <div className="flex flex-col gap-1">
              {candidates.map((candidate) => (
                <div
                  key={`${candidate.source}:${candidate.platformId}`}
                  className="flex items-center gap-3 rounded-2xl px-2 py-2 transition-colors hover:bg-muted/60"
                >
                  <Cover cover={candidate.cover} className="size-12 rounded-full" loading="lazy" />
                  <div className="min-w-0 flex-1">
                    <p className="truncate font-medium">{candidate.name}</p>
                    <p className="truncate text-xs text-muted-foreground">
                      {candidate.subtitle || '艺术家'}
                      {candidate.trackCount > 0 ? ` · ${candidate.trackCount} 首音乐` : ''}
                    </p>
                  </div>
                  <ActionButton
                    size="sm"
                    variant="outline"
                    disabled={!!saving}
                    onClick={() => void select(candidate)}
                  >
                    {saving === candidate.platformId ? (
                      <Spinner />
                    ) : (
                      <Sparkles aria-hidden="true" />
                    )}
                    使用此头像
                  </ActionButton>
                </div>
              ))}
            </div>
          ) : (
            <p className="py-10 text-center text-sm text-muted-foreground">
              {error ? '没有可显示的候选。' : '没有找到艺术家。'}
            </p>
          )}
        </div>
        {error && <ErrorNotice error={error} />}
        <DialogFooter>
          <ActionButton variant="outline" disabled={!!saving} onClick={onClose}>
            取消
          </ActionButton>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
