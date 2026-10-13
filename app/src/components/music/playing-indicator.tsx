import { useRef } from 'react'
import { useInView, useReducedMotion } from 'motion/react'
import { AudioLines } from '@/components/animate-ui/icons/audio-lines'
import { Spinner } from '@/components/ui/spinner'
import { usePlayer } from '@/lib/player'

export function PlayingIndicator() {
  const { status } = usePlayer('status')
  const host = useRef<HTMLSpanElement>(null)
  const visible = useInView(host)
  const reducedMotion = useReducedMotion()
  const playing = status === 'playing'
  const loading = status === 'loading' || status === 'buffering'
  const label = loading ? '正在加载' : playing ? '正在播放' : '当前曲目'
  return (
    <span
      ref={host}
      className="inline-flex shrink-0 items-center justify-center text-[var(--music-accent)]"
      role="img"
      aria-label={label}
      title={label}
    >
      {loading ? (
        <Spinner
          className="size-[18px]"
          style={{ animationPlayState: visible && !reducedMotion ? 'running' : 'paused' }}
          aria-hidden="true"
        />
      ) : (
        <AudioLines size={18} animate={playing && visible && !reducedMotion} aria-hidden="true" />
      )}
    </span>
  )
}
