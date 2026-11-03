import { useEffect, useRef, useState } from 'react'
import { Search, RotateCcw, Check, Minus, Plus, FileText, FolderOpen } from 'lucide-react'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { ErrorNotice } from '@/components/error-notice'
import { errorText, getPlayer, nativeCall, usePlayer, type Lyrics, type Track } from '@/lib/player'
import { isAndroid } from '@/lib/platform'
import { parseLyrics } from './parse-lyrics'
import {
  automaticLyrics,
  type LocalTtml,
  type LyricCandidate,
  type LyricCorrection
} from './correction'
import { useLyricsSettings } from './use-lyrics-settings'

function fileName(path: string) {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path
}

export function LyricCorrectionDialog({ track, onClose }: { track: Track; onClose: () => void }) {
  const { revision } = usePlayer('revision')
  const { lyricsOffsetMs } = useLyricsSettings()
  const openingRevision = useRef(revision).current
  const alive = useRef(true)
  const searchSerial = useRef(0)
  const previewSerial = useRef(0)
  const [choice, setChoice] = useState<LyricCorrection>(automaticLyrics)
  const [ready, setReady] = useState(false)
  const [query, setQuery] = useState(`${track.title} ${track.artist}`.slice(0, 100))
  const [source, setSource] = useState<'qq' | 'netease' | 'local'>('qq')
  const [candidates, setCandidates] = useState<LyricCandidate[]>([])
  const [searched, setSearched] = useState(false)
  const [searching, setSearching] = useState(false)
  const [previewing, setPreviewing] = useState(false)
  const [picking, setPicking] = useState(false)
  const [preview, setPreview] = useState<string[]>([])
  const [validPreview, setValidPreview] = useState(false)
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
        if (value.localTtml) {
          setSource('local')
          void previewLocal(value.localTtml)
        } else if (value.candidate) {
          setSource(value.candidate.source)
          void select(value.candidate)
        }
      })
      .catch((cause) => {
        if (!disposed && current()) setError(errorText(cause))
      })
    return () => {
      disposed = true
      alive.current = false
      searchSerial.current++
      previewSerial.current++
    }
  }, [track.key])
  useEffect(() => {
    if (revision !== openingRevision) onClose()
  }, [revision, openingRevision, onClose])

  async function search() {
    const serial = ++searchSerial.current
    setSearching(true)
    setSearched(false)
    setCandidates([])
    setError(undefined)
    try {
      const values = await nativeCall<LyricCandidate[]>('lyric_candidates', {
        key: track.key,
        source,
        query
      })
      if (current() && serial === searchSerial.current) {
        setCandidates(values)
        setSearched(true)
      }
    } catch (cause) {
      if (current() && serial === searchSerial.current) setError(errorText(cause))
    } finally {
      if (current() && serial === searchSerial.current) setSearching(false)
    }
  }
  async function select(candidate: LyricCandidate) {
    const serial = ++previewSerial.current
    setChoice((value) => ({ ...value, candidate, localTtml: null }))
    setPreviewing(true)
    setValidPreview(false)
    setPreview([])
    setError(undefined)
    try {
      let lines
      for (const skipQrc of [false, true]) {
        const value = await nativeCall<Lyrics | null>('preview_lyric_candidate', {
          key: track.key,
          candidate,
          skipQrc
        })
        if (!current() || serial !== previewSerial.current) return
        if (!value) throw new Error('该候选暂无可用歌词，请选择其他版本。')
        try {
          lines = parseLyrics(value, candidate.durationMs || track.durationMs)
          break
        } catch (cause) {
          if (value.format !== 'qrc' || skipQrc) throw cause
        }
      }
      if (lines && current() && serial === previewSerial.current) {
        setPreview(lines.slice(0, 120).map((line) => line.words.map((word) => word.word).join('')))
        setValidPreview(true)
      }
    } catch (cause) {
      if (current() && serial === previewSerial.current) setError(errorText(cause))
    } finally {
      if (current() && serial === previewSerial.current) setPreviewing(false)
    }
  }
  async function previewLocal(local: LocalTtml) {
    const serial = ++previewSerial.current
    setPreviewing(true)
    setValidPreview(false)
    setPreview([])
    setError(undefined)
    try {
      const value = await nativeCall<Lyrics>('preview_local_ttml', { path: local.path })
      if (!current() || serial !== previewSerial.current) return
      const lines = parseLyrics(value, track.durationMs)
      if (!lines.length) throw new Error('本地歌词内容为空。')
      setPreview(lines.slice(0, 120).map((line) => line.words.map((word) => word.word).join('')))
      setValidPreview(true)
    } catch (cause) {
      if (current() && serial === previewSerial.current)
        setError(errorText(cause) || '本地歌词无法解析。')
    } finally {
      if (current() && serial === previewSerial.current) setPreviewing(false)
    }
  }
  async function chooseLocal() {
    if (!current() || saving || picking) return
    setPicking(true)
    setError(undefined)
    try {
      const path = await nativeCall<string | null>('pick_local_ttml')
      if (!current() || !path) return
      setSource('local')
      setChoice((value) => ({ ...value, candidate: null, localTtml: { path } }))
      await previewLocal({ path })
    } catch (cause) {
      if (current()) setError(errorText(cause))
    } finally {
      if (current()) setPicking(false)
    }
  }
  function selectSource(value: 'qq' | 'netease' | 'local') {
    if (value === source) return
    searchSerial.current++
    previewSerial.current++
    setSearching(false)
    setSearched(false)
    setCandidates([])
    setPreviewing(false)
    setValidPreview(false)
    setPreview([])
    setSource(value)
    setChoice((previous) =>
      value === 'local'
        ? { ...previous, candidate: null }
        : {
            ...previous,
            candidate: previous.candidate?.source === value ? previous.candidate : null,
            localTtml: null
          }
    )
  }
  async function save(value: LyricCorrection) {
    if (!current() || saving) return
    setSaving(true)
    setError(undefined)
    try {
      await nativeCall('set_lyric_correction', { key: track.key, correction: value })
      if (current()) onClose()
    } catch (cause) {
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
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) onClose()
      }}
    >
      <DialogContent className="flex max-h-[90dvh] flex-col gap-5 sm:max-w-3xl">
        <DialogHeader className="shrink-0 pr-8">
          <DialogTitle>歌词来源与纠错</DialogTitle>
          <DialogDescription>
            {track.title} · {track.artist}。选择仅影响这首歌的歌词，重启后保留。
          </DialogDescription>
        </DialogHeader>
        <div className="min-h-0 flex-1 space-y-5 overflow-y-auto">
          <div className="flex gap-2" role="group" aria-label="歌词来源">
            {lyricSources.map((value) => (
              <Button
                key={value}
                type="button"
                variant={source === value ? 'secondary' : 'ghost'}
                aria-pressed={source === value}
                disabled={!ready || saving}
                onClick={() => selectSource(value)}
              >
                {value === 'qq' ? 'QQ 音乐' : value === 'netease' ? '网易云音乐' : '本地歌词'}
              </Button>
            ))}
          </div>
          {source !== 'local' && (
            <form
              className="flex gap-2"
              onSubmit={(event) => {
                event.preventDefault()
                void search()
              }}
            >
              <Input
                aria-label="歌词搜索词"
                value={query}
                maxLength={100}
                onChange={(event) => {
                  searchSerial.current++
                  setSearching(false)
                  setSearched(false)
                  setCandidates([])
                  setQuery(event.target.value)
                }}
              />
              <Button type="submit" disabled={!ready || searching || saving || !query.trim()}>
                <Search aria-hidden="true" />
                {searching ? '搜索中…' : '搜索'}
              </Button>
            </form>
          )}
          <div className="grid gap-4 sm:grid-cols-2">
            {source === 'local' ? (
              <div
                className="min-h-32 space-y-3 rounded-xl border border-dashed p-4"
                aria-label="本地歌词文件"
              >
                <p className="text-sm text-muted-foreground">
                  仅支持 TTML 格式。选择后仅影响这首歌，文件更新会在下次读取时生效。
                </p>
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
            ) : (
              <div className="max-h-64 space-y-1 overflow-y-auto" aria-label="歌词候选">
                {candidates.map((candidate, index) => (
                  <button
                    type="button"
                    key={`${candidate.id}:${candidate.mid}:${index}`}
                    className="w-full rounded-lg p-3 text-left hover:bg-muted focus-visible:outline-2 focus-visible:outline-ring"
                    disabled={saving}
                    aria-pressed={JSON.stringify(choice.candidate) === JSON.stringify(candidate)}
                    onClick={() => void select(candidate)}
                  >
                    <p className="font-medium">{candidate.title}</p>
                    <p className="mt-1 text-xs text-muted-foreground">
                      {candidate.artist} · {candidate.album} ·{' '}
                      {Math.floor(candidate.durationMs / 60000)}:
                      {String(Math.floor(candidate.durationMs / 1000) % 60).padStart(2, '0')}
                    </p>
                  </button>
                ))}
                {searched && !candidates.length && (
                  <p role="status" className="p-3 text-muted-foreground">
                    没有找到候选，请调整搜索词。
                  </p>
                )}
              </div>
            )}
            <div
              className="max-h-64 overflow-y-auto rounded-xl bg-muted/40 p-4"
              aria-label="歌词预览"
              aria-busy={previewing}
            >
              <p className="mb-3 text-xs text-muted-foreground">
                {choice.localTtml
                  ? `本地歌词 · ${fileName(choice.localTtml.path)}`
                  : choice.candidate
                    ? `${choice.candidate.source === 'qq' ? 'QQ 音乐' : '网易云音乐'} · ${choice.candidate.title}`
                    : source === 'local'
                      ? '本地歌词：仅支持 TTML 文件。'
                      : '自动匹配：AMLL DB → QQ 音乐 → 网易云音乐；本地歌词遵循现有设置。'}
              </p>
              {previewing ? (
                <p role="status">正在获取歌词…</p>
              ) : !choice.candidate && !choice.localTtml && source === 'local' ? (
                <p className="text-sm text-muted-foreground">选择 TTML 文件后在此预览。</p>
              ) : (
                preview.map((line, index) => (
                  <p key={index} className="mb-2 break-words">
                    {line}
                  </p>
                ))
              )}
            </div>
          </div>
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
                onClick={() => offset((choice.offsetMs ?? lyricsOffsetMs) - 100)}
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
                value={choice.offsetMs ?? lyricsOffsetMs}
                disabled={!ready || saving}
                onChange={(event) => offset(event.currentTarget.valueAsNumber)}
              />
              <span>毫秒</span>
              <Button
                variant="outline"
                size="icon"
                aria-label="歌词延后 100 毫秒"
                disabled={!ready || saving}
                onClick={() => offset((choice.offsetMs ?? lyricsOffsetMs) + 100)}
              >
                <Plus aria-hidden="true" />
              </Button>
            </div>
            <p className="text-xs text-muted-foreground">
              负数提前，正数延后，范围 ±60 秒；默认跟随全局偏移，保存后覆盖全局值。
            </p>
          </div>
          <ErrorNotice error={error} />
        </div>
        <div className="flex shrink-0 flex-wrap justify-between gap-2">
          <Button
            variant="ghost"
            disabled={!ready || saving}
            onClick={() => void save(automaticLyrics)}
          >
            <RotateCcw aria-hidden="true" />
            恢复自动匹配
          </Button>
          <Button
            disabled={
              !ready ||
              saving ||
              picking ||
              previewing ||
              (source === 'local' && !choice.localTtml) ||
              (!!choice.candidate && !validPreview) ||
              (!!choice.localTtml && !validPreview)
            }
            onClick={() => void save(choice)}
          >
            <Check aria-hidden="true" />
            {saving ? '正在保存…' : '保存'}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  )
}
