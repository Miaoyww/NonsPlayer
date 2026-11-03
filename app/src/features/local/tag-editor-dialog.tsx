import { reportCaughtError } from '@/lib/diagnostics'
import { ErrorNotice } from '@/components/error-notice'
import { Cover } from '@/components/music/cover'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle
} from '@/components/ui/dialog'
import { Skeleton } from '@/components/ui/skeleton'
import { errorText, nativeCall, type Track } from '@/lib/player'
import { open } from '@tauri-apps/plugin-dialog'
import {
  CalendarDays,
  Camera,
  Check,
  Disc3,
  Hash,
  ImageOff,
  Layers,
  MessageSquareText,
  Mic,
  Tag,
  Type,
  Users
} from 'lucide-react'
import { useEffect, useState, type ReactNode } from 'react'

interface LocalTrackTags {
  title: string
  artist: string
  albumArtist: string
  album: string
  year: string
  trackNumber: number | null
  discNumber: number | null
  genre: string
  comment: string
}

interface TagForm {
  title: string
  artist: string
  albumArtist: string
  album: string
  year: string
  trackNumber: string
  discNumber: string
  genre: string
  comment: string
}

type CoverChange = { action: 'keep' } | { action: 'remove' } | { action: 'replace'; path: string }

function form(tags: LocalTrackTags): TagForm {
  return {
    title: tags.title,
    artist: tags.artist,
    albumArtist: tags.albumArtist,
    album: tags.album,
    year: tags.year,
    trackNumber: tags.trackNumber?.toString() ?? '',
    discNumber: tags.discNumber?.toString() ?? '',
    genre: tags.genre,
    comment: tags.comment
  }
}

function number(value: string, label: string) {
  const text = value.trim()
  if (!text) return null
  if (!/^\d+$/.test(text)) throw new Error(`${label}必须是正整数。`)
  const parsed = Number(text)
  if (!Number.isSafeInteger(parsed) || parsed < 1 || parsed > 4_294_967_295)
    throw new Error(`${label}超出支持范围。`)
  return parsed
}

function TagField({
  icon,
  label,
  value,
  disabled,
  inputMode,
  placeholder,
  onChange
}: {
  icon: ReactNode
  label: string
  value: string
  disabled: boolean
  inputMode?: 'numeric' | 'text'
  placeholder?: string
  onChange: (value: string) => void
}) {
  const id = `local-tag-${label}`
  return (
    <label className="local-tag-field" htmlFor={id}>
      <span>{label}</span>
      <span className="local-tag-input">
        <span className="local-tag-field-icon" aria-hidden="true">
          {icon}
        </span>
        <input
          id={id}
          value={value}
          disabled={disabled}
          inputMode={inputMode}
          placeholder={placeholder}
          autoComplete="off"
          spellCheck={false}
          onChange={(event) => onChange(event.target.value)}
        />
      </span>
    </label>
  )
}

export function LocalTagEditorDialog({
  track,
  onClose,
  onSaved
}: {
  track: Track
  onClose: () => void
  onSaved: (track: Track) => void
}) {
  const [initial, setInitial] = useState<TagForm>()
  const [value, setValue] = useState<TagForm>()
  const [cover, setCover] = useState<CoverChange>({ action: 'keep' })
  const [coverPreview, setCoverPreview] = useState<string>()
  const [loading, setLoading] = useState(true)
  const [choosingCover, setChoosingCover] = useState(false)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string>()

  useEffect(() => {
    let disposed = false
    setLoading(true)
    setError(undefined)
    void nativeCall<LocalTrackTags>('local_track_tags', { key: track.key })
      .then((tags) => {
        if (disposed) return
        const next = form(tags)
        setInitial(next)
        setValue(next)
      })
      .catch((cause) => {
        reportCaughtError('app/src/features/local/tag-editor-dialog.tsx', cause)
        if (!disposed) setError(errorText(cause))
      })
      .finally(() => {
        if (!disposed) setLoading(false)
      })
    return () => {
      disposed = true
    }
  }, [track.key])

  const busy = loading || saving || choosingCover
  const changed =
    !!value && (JSON.stringify(value) !== JSON.stringify(initial) || cover.action !== 'keep')
  const removed = cover.action === 'remove'

  function update(patch: Partial<TagForm>) {
    setValue((current) => (current ? { ...current, ...patch } : current))
    setError(undefined)
  }

  async function chooseCover() {
    if (busy) return
    setChoosingCover(true)
    setError(undefined)
    try {
      const selected = await open({
        multiple: false,
        title: '选择内嵌封面',
        filters: [{ name: '图片', extensions: ['jpg', 'jpeg', 'png', 'bmp', 'gif'] }]
      })
      if (typeof selected !== 'string') return
      const preview = await nativeCall<string>('local_tag_cover_preview', { path: selected })
      setCover({ action: 'replace', path: selected })
      setCoverPreview(preview)
    } catch (cause) {
      reportCaughtError('app/src/features/local/tag-editor-dialog.tsx', cause)
      setError(errorText(cause))
    } finally {
      setChoosingCover(false)
    }
  }

  async function save() {
    if (!value || busy || !changed) return
    setSaving(true)
    setError(undefined)
    try {
      const updated = await nativeCall<Track>('local_track_update_tags', {
        key: track.key,
        tags: {
          ...value,
          year: value.year.trim(),
          trackNumber: number(value.trackNumber, '曲目号'),
          discNumber: number(value.discNumber, '碟号'),
          cover
        }
      })
      onSaved(updated)
    } catch (cause) {
      reportCaughtError('app/src/features/local/tag-editor-dialog.tsx', cause)
      setError(errorText(cause))
    } finally {
      setSaving(false)
    }
  }

  return (
    <Dialog
      open
      onOpenChange={(next, details) => {
        if (next) return
        if (saving) {
          details.cancel()
          return
        }
        onClose()
      }}
    >
      <DialogContent className="local-tag-editor-dialog" showCloseButton={!saving}>
        <DialogHeader>
          <DialogTitle>编辑歌曲标签</DialogTitle>
          <DialogDescription>
            保存后会直接改写源文件中的标签与内嵌封面，无法撤销。
          </DialogDescription>
        </DialogHeader>
        <form
          className="local-tag-editor-form"
          onSubmit={(event) => {
            event.preventDefault()
            void save()
          }}
        >
          <div className="local-tag-editor-scroll">
            <div className="local-tag-cover-area">
              <button
                type="button"
                className="local-tag-cover"
                disabled={busy}
                aria-label="选择内嵌封面"
                onClick={() => void chooseCover()}
              >
                {coverPreview ? (
                  <img className="local-tag-cover-image" src={coverPreview} alt="" />
                ) : removed ? (
                  <span className="local-tag-cover-empty">
                    <ImageOff aria-hidden="true" />
                  </span>
                ) : (
                  <Cover cover={track.cover} className="local-tag-cover-image" loading="eager" />
                )}
                <span className="local-tag-cover-badge" aria-hidden="true">
                  {coverPreview ? <Check /> : <Camera />}
                </span>
              </button>
              <div className="local-tag-cover-actions">
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  disabled={busy}
                  onClick={() => void chooseCover()}
                >
                  {coverPreview ? '更换封面' : '选择封面'}
                </Button>
                {!removed && (
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    disabled={busy}
                    onClick={() => {
                      setCover({ action: 'remove' })
                      setCoverPreview(undefined)
                    }}
                  >
                    移除内嵌封面
                  </Button>
                )}
              </div>
            </div>
            {loading ? (
              <div className="local-tag-editor-loading" aria-label="正在读取歌曲标签">
                <Skeleton className="h-[68px] w-full" />
                <Skeleton className="h-[68px] w-full" />
                <Skeleton className="h-[68px] w-full" />
              </div>
            ) : !value ? null : (
              <>
                <TagField
                  icon={<Type />}
                  label="标题"
                  value={value.title}
                  disabled={busy}
                  onChange={(title) => update({ title })}
                />
                <TagField
                  icon={<Mic />}
                  label="艺术家"
                  value={value.artist}
                  disabled={busy}
                  onChange={(artist) => update({ artist })}
                />
                <TagField
                  icon={<Users />}
                  label="专辑艺术家"
                  value={value.albumArtist}
                  disabled={busy}
                  onChange={(albumArtist) => update({ albumArtist })}
                />
                <TagField
                  icon={<Disc3 />}
                  label="专辑"
                  value={value.album}
                  disabled={busy}
                  onChange={(album) => update({ album })}
                />
                <TagField
                  icon={<CalendarDays />}
                  label="年份"
                  value={value.year}
                  disabled={busy}
                  placeholder="例如 2024 或 2024-06-15"
                  onChange={(year) => update({ year })}
                />
                <div className="local-tag-number-row">
                  <TagField
                    icon={<Hash />}
                    label="曲目号"
                    value={value.trackNumber}
                    disabled={busy}
                    inputMode="numeric"
                    onChange={(trackNumber) => update({ trackNumber })}
                  />
                  <TagField
                    icon={<Layers />}
                    label="碟号"
                    value={value.discNumber}
                    disabled={busy}
                    inputMode="numeric"
                    onChange={(discNumber) => update({ discNumber })}
                  />
                </div>
                <TagField
                  icon={<Tag />}
                  label="流派"
                  value={value.genre}
                  disabled={busy}
                  onChange={(genre) => update({ genre })}
                />
                <TagField
                  icon={<MessageSquareText />}
                  label="备注"
                  value={value.comment}
                  disabled={busy}
                  onChange={(comment) => update({ comment })}
                />
                <p className="local-tag-hint">留空会清除对应标签项；曲目号和碟号留空会删除编号。</p>
              </>
            )}
            {error && <ErrorNotice error={error} />}
          </div>
          <DialogFooter className="local-tag-editor-footer">
            <Button type="button" variant="outline" disabled={saving} onClick={onClose}>
              取消
            </Button>
            <Button type="submit" disabled={busy || !changed}>
              {saving ? '正在写入…' : '保存标签'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
