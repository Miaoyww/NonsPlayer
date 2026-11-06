import { useEffect, useRef, useState } from 'react'
import { RotateCcw, Check, Minus, Plus, FileText, FolderOpen } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { ErrorNotice } from '@/components/error-notice'
import { errorText, getPlayer, nativeCall, usePlayer, type Lyrics, type Track } from '@/lib/player'
import { isAndroid } from '@/lib/platform'
import { reportCaughtError } from '@/lib/diagnostics'
import { cn } from '@/lib/utils'
import { automaticLyrics, type LyricCorrection } from './correction'
import { parseLyrics } from './parse-lyrics'

function fileName(path: string) {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path
}

export function LyricCorrectionContent({
  track,
  onClose,
  className
}: {
  track: Track
  onClose: () => void
  className?: string
}) {
  const { revision } = usePlayer('revision')
  const openingRevision = useRef(revision).current
  const alive = useRef(true)
  const [choice, setChoice] = useState<LyricCorrection>(automaticLyrics)
  const [ready, setReady] = useState(false)
  const [source, setSource] = useState<'qq' | 'netease' | 'local' | null>(null)
  const [picking, setPicking] = useState(false)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string>()
  const lyricSources: ('qq' | 'netease' | 'local')[] = isAndroid()
    ? ['qq', 'netease']
    : ['qq', 'netease', 'local']
  const current = () => alive.current && getPlayer().revision === openingRevision
  useEffect(() => {
    let disposed = false
    alive.current = true
    void nativeCall<LyricCorrection>('lyric_correction', { key: track.key })
      .then((value) => {
        if (disposed || !current()) return
        setChoice(value)
        setReady(true)
        setSource(value.localTtml ? 'local' : (value.source ?? value.candidate?.source ?? null))
      })
      .catch((cause) => {
        reportCaughtError('lyrics.correction.read', cause)
        if (!disposed && current()) setError(errorText(cause))
      })
    return () => {
      disposed = true
      alive.current = false
    }
  }, [track.key])
  useEffect(() => {
    if (revision !== openingRevision) onClose()
  }, [revision, openingRevision, onClose])

  function selectSource(value: 'qq' | 'netease' | 'local') {
    setSource(value)
    setChoice((previous) => ({
      ...previous,
      source: value === 'local' ? null : value,
      candidate: null,
      localTtml: value === 'local' ? previous.localTtml : null
    }))
    setError(undefined)
  }
  async function chooseLocal() {
    if (!current() || saving || picking) return
    setPicking(true)
    setError(undefined)
    try {
      const path = await nativeCall<string | null>('pick_local_ttml')
      if (!current() || !path) return
      const lyrics = await nativeCall<Lyrics>('preview_local_ttml', { path })
      if (!current()) return
      if (!parseLyrics(lyrics, track.durationMs).length) throw new Error('本地歌词内容为空。')
      setChoice((previous) => ({
        ...previous,
        source: null,
        candidate: null,
        localTtml: { path }
      }))
    } catch (cause) {
      reportCaughtError('lyrics.correction.select_local', cause)
      if (current()) setError(errorText(cause))
    } finally {
      if (current()) setPicking(false)
    }
  }
  async function save(value: LyricCorrection) {
    if (!current() || saving || picking) return
    setSaving(true)
    setError(undefined)
    try {
      await nativeCall('set_lyric_correction', {
        key: track.key,
        correction: value
      })
      if (current()) onClose()
    } catch (cause) {
      reportCaughtError('lyrics.correction.save', cause)
      if (current()) setError(errorText(cause))
    } finally {
      if (current()) setSaving(false)
    }
  }
  const offset = (value: number) => {
    if (!Number.isFinite(value)) return
    setChoice((previous) => ({
      ...previous,
      offsetMs: Math.max(-60000, Math.min(60000, Math.round(value)))
    }))
  }
  return (
    <div className={cn('flex min-h-0 flex-1 flex-col gap-5', className)}>
      <header className="shrink-0 space-y-2">
        <h2 className="text-lg font-semibold">歌词来源与纠错</h2>
        <p className="text-sm text-muted-foreground">
          {track.title} · {track.artist}。选择仅影响这首歌的歌词，重启后保留。
        </p>
      </header>
      <div className="lyric-correction-content-scroll min-h-0 flex-1 space-y-5 overflow-y-auto pr-1">
        <div className="flex flex-wrap gap-2" role="group" aria-label="歌词来源">
          {lyricSources.map((value) => (
            <Button
              key={value}
              type="button"
              variant={source === value ? 'secondary' : 'ghost'}
              aria-pressed={source === value}
              disabled={!ready || saving || picking}
              onClick={() => selectSource(value)}
            >
              {value === 'qq' ? 'QQ 音乐' : value === 'netease' ? '网易云音乐' : '本地歌词'}
            </Button>
          ))}
        </div>
        {source === 'local' && (
          <div className="space-y-3 rounded-xl border border-dashed p-4" aria-label="本地歌词文件">
            <p className="text-sm text-muted-foreground">选择本地 TTML 文件用于这首歌。</p>
            <Button
              type="button"
              variant="outline"
              disabled={!ready || saving || picking}
              onClick={() => void chooseLocal()}
            >
              <FolderOpen aria-hidden="true" />
              {picking ? '选择中…' : choice.localTtml ? '重新选择 TTML 文件' : '选择 TTML 文件'}
            </Button>
            {choice.localTtml && (
              <div className="flex min-w-0 items-start gap-2">
                <FileText className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
                <p className="min-w-0 break-all text-sm" title={choice.localTtml.path}>
                  {fileName(choice.localTtml.path)}
                </p>
              </div>
            )}
          </div>
        )}
        {isAndroid() && (
          <div className="space-y-2">
            <label htmlFor="lyric-offset" className="font-medium">
              单曲时间偏移
            </label>
            <div className="flex items-center gap-2">
              <Button
                variant="outline"
                size="icon"
                aria-label="歌词提前 100 毫秒"
                disabled={!ready || saving}
                onClick={() => offset((choice.offsetMs ?? 0) - 100)}
              >
                <Minus aria-hidden="true" />
              </Button>
              <Input
                id="lyric-offset"
                className="w-28"
                type="number"
                min={-60000}
                max={60000}
                step={1}
                value={choice.offsetMs ?? 0}
                disabled={!ready || saving}
                onChange={(event) => offset(event.currentTarget.valueAsNumber)}
              />
              <span>毫秒</span>
              <Button
                variant="outline"
                size="icon"
                aria-label="歌词延后 100 毫秒"
                disabled={!ready || saving}
                onClick={() => offset((choice.offsetMs ?? 0) + 100)}
              >
                <Plus aria-hidden="true" />
              </Button>
            </div>
            <p className="text-xs text-muted-foreground">
              在全局偏移上额外调整，负数提前，正数延后，范围 ±60 秒。
            </p>
          </div>
        )}
        <ErrorNotice error={error} />
      </div>
      <div className="flex shrink-0 flex-wrap justify-between gap-2">
        <Button
          variant="ghost"
          disabled={!ready || saving || picking}
          onClick={() => void save(automaticLyrics)}
        >
          <RotateCcw aria-hidden="true" />
          恢复自动匹配
        </Button>
        <Button
          disabled={!ready || saving || picking || (source === 'local' && !choice.localTtml)}
          onClick={() =>
            void save({
              ...choice,
              source: source === 'local' ? null : source,
              candidate: source === null ? choice.candidate : null,
              localTtml: source === 'local' ? choice.localTtml : null
            })
          }
        >
          <Check aria-hidden="true" />
          {saving ? '正在保存…' : '保存'}
        </Button>
      </div>
    </div>
  )
}
