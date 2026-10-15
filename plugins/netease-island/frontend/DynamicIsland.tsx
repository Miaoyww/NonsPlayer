import { useEffect, useRef, useState } from 'react'
import {
  Button,
  SongArtists,
  SongLikeButton,
  useSongPlayback,
  useCoverSource,
  usePluginEvent,
  usePluginConfig,
  type PluginSong
} from '@app/plugin-sdk'

export function DynamicIsland() {
  const config = usePluginConfig()
  const [preferences, setPreferences] = useState({ previewDuration: 8, pauseOnHover: true })
  useEffect(() => {
    let disposed = false
    let cleanup: (() => void) | undefined
    const apply = (snapshot: { values: Record<string, unknown> }) => {
      if (!disposed)
        setPreferences({
          previewDuration: Number(snapshot.values.previewDuration),
          pauseOnHover: !!snapshot.values.pauseOnHover
        })
    }
    void (async () => {
      const unlisten = await config.subscribe(apply)
      if (disposed) {
        unlisten()
        return
      }
      cleanup = unlisten
      apply(await config.getSnapshot())
    })().catch(console.error)
    return () => {
      disposed = true
      cleanup?.()
    }
  }, [config])
  const detected = usePluginEvent<PluginSong>('song-detected')
  const [song, setSong] = useState<PluginSong>()
  const [hovered, setHovered] = useState(false)
  const [focused, setFocused] = useState(false)
  const paused = (hovered && preferences.pauseOnHover) || focused
  const last = useRef<{ id: string; time: number } | undefined>(undefined)
  const remaining = useRef({ id: '', ms: 8000 })
  const dismiss = () => {
    setSong(undefined)
    setHovered(false)
    setFocused(false)
  }
  useEffect(() => {
    if (
      !detected ||
      (last.current?.id === detected.reference.id && Date.now() - last.current.time < 30000)
    )
      return
    last.current = { id: detected.reference.id, time: Date.now() }
    remaining.current = { id: detected.reference.id, ms: preferences.previewDuration * 1000 }
    setSong(detected)
  }, [detected, preferences.previewDuration])
  useEffect(() => {
    if (!song || paused) return
    const start = performance.now()
    const timer = window.setTimeout(dismiss, remaining.current.ms)
    return () => {
      window.clearTimeout(timer)
      if (remaining.current.id === song.reference.id)
        remaining.current.ms = Math.max(0, remaining.current.ms - (performance.now() - start))
    }
  }, [song, paused])
  const cover = useCoverSource(song?.cover, !!song)
  if (!song) return null
  return (
    <div
      className="nons-island"
      aria-label="发现的网易云歌曲"
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      onFocus={() => setFocused(true)}
      onBlur={(e) => {
        if (!e.currentTarget.contains(e.relatedTarget)) setFocused(false)
      }}
      onKeyDown={(e) => {
        if (e.key === 'Escape') {
          e.stopPropagation()
          dismiss()
        }
      }}
    >
      <style>{`.nons-island{pointer-events:auto;width:min(440px,calc(100vw - 32px));padding:16px;border:1px solid var(--border);border-radius:28px;background:var(--background);color:var(--foreground);box-shadow:0 12px 40px #0002;animation:nons-island-enter .25s ease-out}.nons-island-summary{display:flex;align-items:center;gap:14px}.nons-island-cover{width:56px;height:56px;flex-shrink:0;object-fit:cover;border-radius:16px;background:var(--muted)}.nons-island-text{min-width:0;flex:1}.nons-island-text p{margin:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.nons-island-label{font-size:11px;color:var(--muted-foreground);margin-bottom:4px!important}.nons-island-title{font-size:14px;font-weight:600}.nons-island-artist{margin-top:3px!important;font-size:12px;color:var(--muted-foreground)}.nons-island-actions{display:flex;flex-wrap:wrap;align-items:center;gap:8px;margin-top:14px;padding-top:12px;border-top:1px solid var(--border)}.nons-island-actions>button{flex:1}.nons-island-feedback{margin:10px 0 0;font-size:12px;color:var(--muted-foreground);overflow-wrap:anywhere}.nons-island-feedback[role=alert]{color:var(--destructive)}@keyframes nons-island-enter{from{opacity:0;transform:translateY(-8px) scale(.97)}to{opacity:1;transform:none}}@media(prefers-reduced-motion:reduce){.nons-island{animation:none}}`}</style>
      <div className="nons-island-summary">
        {cover ? (
          <img className="nons-island-cover" src={cover} alt={`${song.title} 封面`} />
        ) : (
          <div className="nons-island-cover" aria-hidden="true" />
        )}
        <div className="nons-island-text">
          <p className="nons-island-label" role="status">
            发现网易云歌曲分享
          </p>
          <p className="nons-island-title" title={song.title}>
            {song.title}
          </p>
          <p className="nons-island-artist" title={song.artist}>
            <SongArtists song={song} />
          </p>
        </div>
        <Button variant="ghost" size="icon" aria-label="关闭歌曲预览" onClick={dismiss}>
          ×
        </Button>
      </div>
      <SongControls key={song.reference.id} song={song} />
    </div>
  )
}

function SongControls({ song }: { song: PluginSong }) {
  const play = useSongPlayback()
  const [busy, setBusy] = useState<'now' | 'next'>()
  const [message, setMessage] = useState<string>()
  const [error, setError] = useState<string>()
  const pending = useRef(false)
  const active = useRef(true)
  useEffect(() => {
    active.current = true
    return () => {
      active.current = false
    }
  }, [])
  function onError(cause: unknown) {
    if (active.current) {
      setMessage(undefined)
      setError(cause instanceof Error ? cause.message : String(cause))
    }
  }
  async function perform(mode: 'now' | 'next') {
    if (pending.current) return
    pending.current = true
    setBusy(mode)
    setError(undefined)
    setMessage(undefined)
    try {
      await play(song.reference, mode)
      if (active.current) setMessage(mode === 'next' ? '已加入下一首播放' : '已开始播放')
    } catch (cause) {
      onError(cause)
    } finally {
      pending.current = false
      if (active.current) setBusy(undefined)
    }
  }
  return (
    <>
      <div className="nons-island-actions">
        <Button size="sm" onClick={() => void perform('now')} disabled={!!busy}>
          {busy === 'now' ? '正在播放…' : '立即播放'}
        </Button>
        <Button
          size="sm"
          variant="secondary"
          onClick={() => void perform('next')}
          disabled={!!busy}
        >
          {busy === 'next' ? '正在加入…' : '下一首播放'}
        </Button>
        <SongLikeButton song={song} onError={onError} />
      </div>
      {error ? (
        <p className="nons-island-feedback" role="alert">
          {error}
        </p>
      ) : (
        message && (
          <p className="nons-island-feedback" role="status">
            {message}
          </p>
        )
      )}
    </>
  )
}
