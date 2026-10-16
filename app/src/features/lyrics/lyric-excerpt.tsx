import { TextSkeleton } from '@/components/music/loading'
import { ErrorNotice } from '@/components/error-notice'
import { useLocalPreferences } from '@/features/local/use-local-preferences'
import { useEffect, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { errorText, nativeCall, type Track } from '@/lib/player'
import { getLyricSources, useLyricSources } from '@/features/lyrics/use-lyric-sources'
import { loadLyrics } from '@/features/lyrics/load-lyrics'
import { Heart } from 'lucide-react'

export function LyricExcerpt({ tracks, enabled = true }: { tracks?: Track[]; enabled?: boolean }) {
  const { sources } = useLyricSources()
  const { options: localPreferences } = useLocalPreferences()
  const [lines, setLines] = useState<string[]>([])
  const [error, setError] = useState('')
  useEffect(() => {
    let disposed = false
    const isCurrent = () => !disposed && getLyricSources() === sources
    setLines([])
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
            setLines(excerpt.slice(start, start + 3))
            return
          }
          failures.push(`「${track.title}」没有可用歌词`)
        } catch (cause) {
          failures.push(`「${track.title}」歌词读取失败：${errorText(cause)}`)
        }
      }
      if (!isCurrent()) return
      try {
        const quote = (await nativeCall<string>('discovery_hitokoto')).trim()
        if (!isCurrent()) return
        if (!quote) throw new Error('返回内容为空')
        setLines([quote])
      } catch (cause) {
        if (isCurrent()) setError([...failures, `一言获取失败：${errorText(cause)}`].join('；'))
      }
    }
    void load()
    return () => {
      disposed = true
    }
  }, [tracks?.[0]?.key, tracks?.[1]?.key, sources, enabled, localPreferences])
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
