import { useEffect } from 'react'
import { getPlayer, nativeCall } from '@/lib/player'
import { useMusicNavigation } from '@/features/workspace/music-navigation'

declare global {
  interface Window {
    __NONS_PERFORMANCE__?: boolean
  }
}
const pause = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms))
const probe = <T,>(action: string, value: Record<string, unknown> = {}) =>
  nativeCall<T>('performance_probe', { action, value })

/** Mounted only in explicitly instrumented builds with an isolated data directory. */
export default function PerformanceProbe({ openLyrics }: { openLyrics: (open: boolean) => void }) {
  const { navigate } = useMusicNavigation()
  useEffect(() => {
    let disposed = false
    const run = async () => {
      if (import.meta.env.DEV) throw new Error('性能基线必须使用嵌入的生产前端')
      await new Promise<void>((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(() => resolve()))
      )
      if (disposed) return
      await probe('mark', {
        event: 'first_screen',
        fcpMs: performance.getEntriesByName('first-contentful-paint')[0]?.startTime ?? null,
        viewport: [innerWidth, innerHeight],
        dpr: devicePixelRatio,
        userAgent: navigator.userAgent
      })
      const config = await probe<{ sampleSeconds: number; audioRuns: number }>('config')
      const initialization = performance.now()
      while (!(await probe<boolean>('ready'))) {
        if (performance.now() - initialization > 90000) throw new Error('适配器／插件初始化未完成')
        await pause(250)
      }
      const mark = (event: string, value: Record<string, unknown> = {}) =>
        probe('mark', { event, ...value })
      const play = async (count: number) => {
        // Drain the previous pipeline before arming the next output measurement.
        // Otherwise an old song's buffer could satisfy the new request's probe.
        await nativeCall('player_action', { action: 'stop' })
        const stopping = performance.now()
        while (getPlayer().status !== 'stopped') {
          if (performance.now() - stopping > 5000) throw new Error('旧播放管线未停止')
          await pause(50)
        }
        const started = performance.now()
        await probe('prepare', { count })
        while (getPlayer().queue.length !== count || getPlayer().status !== 'playing') {
          if (disposed) throw new Error('性能探测页面已卸载')
          if (performance.now() - started > 20000 || getPlayer().status === 'error')
            throw new Error('播放探测未就绪')
          await pause(50)
        }
        let audio = await probe<{ firstBrowserPlayingMs: number } | null>('audio')
        while (!audio) {
          if (performance.now() - started > 20000) throw new Error('未捕获输出 PCM')
          await pause(50)
          audio = await probe('audio')
        }
        await mark('play_ready', { count, frontendReadyMs: performance.now() - started, ...audio })
      }
      const sample = async (phase: string) => {
        await pause(2000)
        await mark('phase_start', { phase })
        await pause(config.sampleSeconds * 1000)
        await mark('phase_end', { phase })
      }
      await sample('idle')
      for (let i = 0; i < config.audioRuns; i++) {
        await play(1)
        await pause(300)
      }
      await sample('playing')
      await probe('hide')
      await sample('background')
      await probe('show')
      openLyrics(true)
      const lyricsStarted = performance.now()
      while (!document.querySelector('.lyric-renderer')?.textContent?.includes('Baseline lyric')) {
        if (performance.now() - lyricsStarted > 15000) throw new Error('真实歌词渲染器未就绪')
        await pause(100)
      }
      await mark('lyrics_ready', { lines: 300 })
      await sample('lyrics')
      openLyrics(false)
      await pause(500)
      navigate('queue')
      for (const count of [1000, 10000]) {
        await play(count)
        await sample(`queue_${count}`)
      }
      await probe('finish', { success: true })
    }
    void run().catch(async (cause) => {
      try {
        await probe('show')
        await probe('finish', { success: false, error: String(cause) })
      } catch {
        /* Process watchdog owns failure reporting. */
      }
    })
    return () => {
      disposed = true
    }
  }, [])
  return null
}
