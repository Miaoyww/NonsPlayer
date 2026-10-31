import { useEffect, useRef, useState } from 'react'
import {
  Button,
  SongArtists,
  SongLikeButton,
  usePluginEvent,
  useSongPlayback,
  type PluginSong
} from '@nonsplayer/plugin-sdk'

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
