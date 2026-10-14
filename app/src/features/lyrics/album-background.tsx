import type { BackgroundRender, MeshGradientRenderer } from '@applemusic-like-lyrics/core'
import { useEffect, useRef, useState } from 'react'
import { useReducedMotion } from 'motion/react'
import { useCoverImageSource } from '@/components/music/use-cover-source'
import { useLyricsSettings } from '@/features/lyrics/use-lyrics-settings'

export function AlbumBackground({
  cover,
  playing,
  hasLyrics,
  active = true,
  enabled = true
}: {
  cover?: import('@/features/music/types').CoverInput
  playing: boolean
  hasLyrics: boolean
  active?: boolean
  enabled?: boolean
}) {
  const host = useRef<HTMLDivElement>(null)
  const renderer = useRef<BackgroundRender<MeshGradientRenderer> | null>(null)
  const activeRef = useRef(active)
  activeRef.current = active
  const reduced = useReducedMotion()
  const { backgroundSpeed } = useLyricsSettings()
  const [visible, setVisible] = useState(document.visibilityState !== 'hidden')
  const [ready, setReady] = useState(false)
  const { source: album, onError } = useCoverImageSource(
    cover,
    visible && active && enabled,
    'small'
  )
  useEffect(() => {
    const changed = () => setVisible(document.visibilityState !== 'hidden')
    document.addEventListener('visibilitychange', changed)
    return () => document.removeEventListener('visibilitychange', changed)
  }, [])
  useEffect(() => {
    setReady(false)
    if (!host.current || !album || !visible || !enabled) return
    let disposed = false
    let background: BackgroundRender<MeshGradientRenderer> | undefined
    // Keep shader compilation and canvas creation out of the slide-in animation.
    void import('@applemusic-like-lyrics/core')
      .then(async ({ BackgroundRender, MeshGradientRenderer }) => {
        if (disposed || !host.current || !activeRef.current) return
        background = BackgroundRender.new(MeshGradientRenderer)
        background.pause()
        background.setFPS(30)
        background.setRenderScale(0.5)
        const element = background.getElement()
        element.style.cssText = 'position:absolute;inset:0;width:100%;height:100%;display:block;'
        host.current.appendChild(element)
        renderer.current = background
        await background.setAlbum(album)
        if (!disposed) setReady(true)
      })
      .catch(() => {
        if (disposed) return
        background?.dispose()
        background = undefined
        renderer.current = null
      })
    return () => {
      disposed = true
      background?.dispose()
      renderer.current = null
    }
  }, [album, visible, enabled])
  useEffect(() => {
    const background = renderer.current
    if (!background) return
    background.setFlowSpeed(backgroundSpeed)
    background.setHasLyric(hasLyrics)
    background.setStaticMode(!!reduced)
    if (active && visible && playing && !reduced) background.resume()
    else background.pause()
  }, [album, visible, active, ready, playing, reduced, hasLyrics, backgroundSpeed])
  return (
    <div
      className="album-background pointer-events-none absolute inset-0 overflow-hidden"
      aria-hidden="true"
    >
      {album && (
        <img
          src={album}
          alt=""
          onError={onError}
          className="absolute size-full scale-125 object-cover opacity-70 blur-3xl"
        />
      )}
      <div ref={host} className="album-background-canvas absolute inset-0" data-ready={ready} />
      <div className="album-background-scrim absolute inset-0" />
    </div>
  )
}
