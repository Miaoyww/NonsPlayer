import { Skeleton } from '@/components/ui/skeleton'
import { reportCaughtError } from '@/lib/diagnostics'
import { nativeCall } from '@/lib/player'
import { isTauri } from '@tauri-apps/api/core'
import { useEffect, useState } from 'react'
import {
  formatListeningCount,
  formatListeningDuration,
  type ListeningStatistics
} from './listening-statistics'

export function ListeningStatisticsCard({ revision }: { revision?: string }) {
  const native = isTauri()
  const [stats, setStats] = useState<ListeningStatistics>()
  const [error, setError] = useState(false)

  useEffect(() => {
    if (!native) return
    let disposed = false
    setError(false)
    void nativeCall<ListeningStatistics>('listening_statistics')
      .then((value) => {
        if (!disposed) setStats(value)
      })
      .catch((cause) => {
        reportCaughtError('app/src/features/library/listening-statistics-card.tsx', cause)

        if (!disposed) setError(true)
      })
    return () => {
      disposed = true
    }
  }, [native, revision])

  if (!native) return null

  return (
    <section className="library-stats" aria-labelledby="library-stats-heading">
      <h2 className="library-stats-heading" id="library-stats-heading">
        听歌统计
      </h2>
      {error && !stats ? (
        <p className="library-stats-note">暂时无法读取本机统计。</p>
      ) : (
        <dl className="library-stats-list">
          <div>
            <dt>累计收听</dt>
            <dd>
              {stats ? (
                formatListeningDuration(stats.seconds)
              ) : (
                <Skeleton className="h-6 w-20" aria-hidden="true" />
              )}
            </dd>
          </div>
          <div>
            <dt>听歌天数</dt>
            <dd>
              {stats ? (
                `${formatListeningCount(stats.days)} 天`
              ) : (
                <Skeleton className="h-6 w-12" aria-hidden="true" />
              )}
            </dd>
          </div>
          <div>
            <dt>播放次数</dt>
            <dd>
              {stats ? (
                `${formatListeningCount(stats.plays)} 次`
              ) : (
                <Skeleton className="h-6 w-12" aria-hidden="true" />
              )}
            </dd>
          </div>
        </dl>
      )}
    </section>
  )
}
