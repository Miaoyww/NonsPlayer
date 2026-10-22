import { useEffect, useRef, useState } from 'react'
import { Search, RotateCcw, Check, Minus, Plus } from 'lucide-react'
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
import { parseLyrics } from './parse-lyrics'
import { automaticLyrics, type LyricCandidate, type LyricCorrection } from './correction'

export function LyricCorrectionDialog({ track, onClose }: { track: Track; onClose: () => void }) {
  const { revision } = usePlayer('revision')
  const openingRevision = useRef(revision).current
  const alive = useRef(true)
  const searchSerial = useRef(0)
  const previewSerial = useRef(0)
  const [choice, setChoice] = useState<LyricCorrection>(automaticLyrics)
  const [ready, setReady] = useState(false)
  const [query, setQuery] = useState(`${track.title} ${track.artist}`.slice(0, 100))
  const [source, setSource] = useState<'qq' | 'netease'>('qq')
  const [candidates, setCandidates] = useState<LyricCandidate[]>([])
  const [searched, setSearched] = useState(false)
  const [searching, setSearching] = useState(false)
  const [previewing, setPreviewing] = useState(false)
  const [preview, setPreview] = useState<string[]>([])
  const [validPreview, setValidPreview] = useState(false)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string>()
  const current = () => alive.current && getPlayer().revision === openingRevision
  useEffect(() => {
    let disposed = false
    alive.current = true
    void nativeCall<LyricCorrection>('lyric_correction', { key: track.key })
      .then((value) => {
        if (disposed || !current()) return
        setChoice(value)
        setReady(true)
        if (value.candidate) void select(value.candidate)
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
    setChoice((value) => ({ ...value, candidate }))
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
  const offset = (value: number) =>
    setChoice((previous) => ({
      ...previous,
      offsetMs: Math.max(-60000, Math.min(60000, Math.round(value)))
    }))
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
          <form
            className="space-y-3"
            onSubmit={(event) => {
              event.preventDefault()
              void search()
            }}
          >
            <div className="flex gap-2" role="group" aria-label="搜索歌词来源">
              {(['qq', 'netease'] as const).map((value) => (
                <Button
                  key={value}
                  type="button"
                  variant={source === value ? 'secondary' : 'ghost'}
                  aria-pressed={source === value}
                  disabled={saving}
                  onClick={() => {
                    searchSerial.current++
                    setSearching(false)
                    setSearched(false)
                    setCandidates([])
                    setSource(value)
                  }}
                >
                  {value === 'qq' ? 'QQ 音乐' : '网易云音乐'}
                </Button>
              ))}
            </div>
            <div className="flex gap-2">
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
            </div>
          </form>
          <div className="grid gap-4 sm:grid-cols-2">
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
            <div
              className="max-h-64 overflow-y-auto rounded-xl bg-muted/40 p-4"
              aria-label="歌词预览"
              aria-busy={previewing}
            >
              <p className="mb-3 text-xs text-muted-foreground">
                {choice.candidate
                  ? `${choice.candidate.source === 'qq' ? 'QQ 音乐' : '网易云音乐'} · ${choice.candidate.title}`
                  : '自动匹配：AMLL DB → QQ 音乐 → 网易云音乐；本地歌词遵循现有设置。'}
              </p>
              {previewing ? (
                <p role="status">正在获取歌词…</p>
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
                onClick={() => offset(choice.offsetMs - 100)}
              >
                <Minus aria-hidden="true" />
              </Button>
              <Input
                id="lyric-offset"
                className="w-28"
                type="number"
                min={-60000}
                max={60000}
                step={100}
                value={choice.offsetMs}
                disabled={!ready || saving}
                onChange={(event) => offset(Number(event.target.value))}
              />
              <span>毫秒</span>
              <Button
                variant="outline"
                size="icon"
                aria-label="歌词延后 100 毫秒"
                disabled={!ready || saving}
                onClick={() => offset(choice.offsetMs + 100)}
              >
                <Plus aria-hidden="true" />
              </Button>
            </div>
            <p className="text-xs text-muted-foreground">
              负数提前，正数延后，范围 ±60 秒；保存后生效。
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
            disabled={!ready || saving || previewing || (!!choice.candidate && !validPreview)}
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
