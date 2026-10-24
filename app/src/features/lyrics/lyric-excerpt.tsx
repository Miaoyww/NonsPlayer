import { reportCaughtError } from '@/lib/diagnostics'
import { TextSkeleton } from '@/components/music/loading'
import { ErrorNotice } from '@/components/error-notice'
import { useLocalPreferences } from '@/features/local/use-local-preferences'
import { useEffect, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { errorText, nativeCall, type Track } from '@/lib/player'
import { getLyricSources, useLyricSources } from '@/features/lyrics/use-lyric-sources'
import { loadLyrics, lyricTrackVersion } from '@/features/lyrics/load-lyrics'
import { Heart } from 'lucide-react'

const excerptCache = new Map<string, string[]>()
const MAX_EXCERPT_CACHE_ENTRIES = 32
const MAX_EXCERPT_CACHE_BYTES = 256 * 1024

function rememberExcerpt(key: string, lines: string[]) {
  if (JSON.stringify([key, lines]).length > 8192) return
  excerptCache.delete(key)
  excerptCache.set(key, lines)
  const size = () =>
    [...excerptCache].reduce(
      (total, [cacheKey, value]) => total + JSON.stringify([cacheKey, value]).length,
      0
    )
  while (excerptCache.size > MAX_EXCERPT_CACHE_ENTRIES || size() > MAX_EXCERPT_CACHE_BYTES) {
    const oldest = excerptCache.keys().next().value
    if (oldest) excerptCache.delete(oldest)
    else break
  }
}

export function LyricExcerpt({ tracks, enabled = true }: { tracks?: Track[]; enabled?: boolean }) {
  const { sources } = useLyricSources()
  const { options: localPreferences } = useLocalPreferences()
  const cacheKey = JSON.stringify([
    (tracks ?? []).slice(0, 2).map(lyricTrackVersion),
    sources,
    localPreferences
  ])
  const [display, setDisplay] = useState<{ key: string; lines: string[] }>({
    key: cacheKey,
    lines: excerptCache.get(cacheKey) ?? []
  })
  const [error, setError] = useState('')
  const lines = display.key === cacheKey ? display.lines : (excerptCache.get(cacheKey) ?? [])
  useEffect(() => {
    let disposed = false
    const isCurrent = () => !disposed && getLyricSources() === sources
    setDisplay({ key: cacheKey, lines: excerptCache.get(cacheKey) ?? [] })
    setError('')
    if (!enabled || !isTauri()) return
    const load = async () => {
      const failures: string[] = []
      for (const track of (tracks ?? []).slice(0, 2)) {
        if (!isCurrent()) return
        try {
          const { parseLyrics } = await import('@/features/lyrics/parse-lyrics')
          if (!isCurrent()) return
          const result = await loadLyrics(
            track,
            false,
            sources,
            (value) => (value ? parseLyrics(value, track.durationMs) : []),
            isCurrent
          )
          if (!isCurrent()) return
          const excerpt =
            result?.parsed
              .map((line) =>
                line.words
                  .map((word) => word.word)
                  .join('')
                  .trim()
              )
              .filter((line) => line && !/作词|作曲|纯音乐|编曲/.test(line)) ?? []
          if (excerpt.length) {
            const start = Math.floor(Math.random() * Math.max(1, excerpt.length - 2))
            const nextLines = excerpt.slice(start, start + 3)
            rememberExcerpt(cacheKey, nextLines)
            setDisplay({ key: cacheKey, lines: nextLines })
            return
          }
          failures.push(`「${track.title}」没有可用歌词`)
        } catch (cause) {
          reportCaughtError('app/src/features/lyrics/lyric-excerpt.tsx', cause)

          failures.push(`「${track.title}」歌词读取失败：${errorText(cause)}`)
        }
      }
      if (!isCurrent()) return
      try {
        const quote = (await nativeCall<string>('discovery_hitokoto')).trim()
        if (!isCurrent()) return
        if (!quote) throw new Error('返回内容为空')
        const nextLines = [quote]
        rememberExcerpt(cacheKey, nextLines)
        setDisplay({ key: cacheKey, lines: nextLines })
      } catch (cause) {
        reportCaughtError('app/src/features/lyrics/lyric-excerpt.tsx', cause)

        if (isCurrent()) setError([...failures, `一言获取失败：${errorText(cause)}`].join('；'))
      }
    }
    void load()
    return () => {
      disposed = true
    }
  }, [cacheKey, tracks?.[0]?.key, tracks?.[1]?.key, sources, enabled, localPreferences])
  if (!enabled) return <Heart className="size-10 opacity-30" aria-hidden="true" />
  if (!error && !lines.length) return <TextSkeleton />
  return (
    <>
      <ErrorNotice error={error} />
      <p className="library-lyric-excerpt" aria-live="polite">
        {lines.length
          ? lines.map((line, index) => (
              <span key={index}>
                {line}
                <br />
              </span>
            ))
          : error
            ? '暂无歌词'
            : '正在寻找歌词…'}
      </p>
    </>
  )
}
