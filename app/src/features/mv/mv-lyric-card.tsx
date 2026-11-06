import { MorphingText } from '@animate-ui/react/primitives/texts/morphing'
import type { LyricLine } from '@applemusic-like-lyrics/core'
import { motion, useReducedMotion } from 'motion/react'
import { useMemo } from 'react'
import { lyricPronunciation, lyricText } from '@/features/desktop-lyrics/lyric-display'
import { currentPosition, useProgress } from '@/lib/player'
import { findLyricLine } from './mv-lyric-line'

export function MvLyricCard({
  lines,
  offsetMs = 0,
  showTranslation,
  showPronunciation
}: {
  lines: LyricLine[]
  offsetMs?: number
  showTranslation: boolean
  showPronunciation: boolean
}) {
  const progress = useProgress()
  const reduced = useReducedMotion()
  const position =
    (progress.status === 'playing' ? currentPosition() : progress.positionMs) - offsetMs
  const line = useMemo(() => findLyricLine(lines, position), [lines, position])
  if (!line) return null
  const translation = showTranslation ? line.translatedLyric.trim() : ''
  const pronunciation = showPronunciation ? lyricPronunciation(line).trim() : ''
  return (
    <div className="mv-lyric-overlay">
      <div className="mv-lyric-card" role="status" aria-live="polite">
        <motion.div
          className="mv-lyric-card-surface"
          aria-hidden="true"
          layout={reduced ? false : 'size'}
          transition={{ layout: { duration: 0.28, ease: [0.22, 1, 0.36, 1] } }}
        />
        <MorphingText className="mv-lyric-original" text={lyricText(line)} />
        {translation && <MorphingText className="mv-lyric-auxiliary" text={translation} />}
        {pronunciation && <MorphingText className="mv-lyric-auxiliary" text={pronunciation} />}
      </div>
    </div>
  )
}
