import { useEffect, useRef, useState } from 'react'
import {
  Button,
  SongArtists,
  SongLikeButton,
  useFullscreenPlayerPanel,
  usePlayer,
  usePluginEvent,
  useSongPlayback,
  type PluginSong
} from '@nonsplayer/plugin-sdk'

export function FullscreenDetailsPanel() {
  const player = usePlayer()
  const panel = useFullscreenPlayerPanel('fullscreen-details')
  return (
    <section>
      <h2>{panel.label}</h2>
      <p>{player.track?.title ?? '当前没有歌曲'}</p>
    </section>
  )
}

export function FullscreenDetailsAction() {
  const panel = useFullscreenPlayerPanel('fullscreen-details')
  return (
    <Button aria-label={panel.label} aria-pressed={panel.active} onClick={panel.show}>
      {panel.label}
    </Button>
  )
}

export function Preview() {
  const song = usePluginEvent<PluginSong>('track')
  const play = useSongPlayback()
  const pending = useRef(false)
  const [message, setMessage] = useState('')
  useEffect(() => {
    if (!song) return
    const timer = window.setTimeout(() => setMessage('expired'), 60000)
    return () => window.clearTimeout(timer)
  }, [song])
  if (!song) return null
  return (
    <div>
      <span>{song.title}</span>
      <SongArtists song={song} />
      <SongLikeButton song={song} onError={(error) => setMessage(String(error))} />
      <Button
        onClick={async () => {
          if (pending.current) return
          pending.current = true
          try {
            await play(song.reference, 'next')
            setMessage('queued')
          } finally {
            pending.current = false
          }
        }}
      >
        Queue
      </Button>
      <p role="status">{message}</p>
    </div>
  )
}
