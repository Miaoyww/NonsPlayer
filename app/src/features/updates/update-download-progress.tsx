import { useEffect, useRef, useState } from 'react'
import { Progress, ProgressLabel } from '@/components/ui/progress'

function formatSize(bytes: number) {
  const units = ['B', 'KB', 'MB', 'GB']
  const unit = Math.min(3, Math.floor(Math.log(Math.max(1, bytes)) / Math.log(1024)))
  return `${(bytes / 1024 ** unit).toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`
}

export function UpdateDownloadProgress({
  downloaded,
  total
}: {
  downloaded: number
  total: number | null
}) {
  const latest = useRef(downloaded)
  const sample = useRef({ bytes: downloaded, time: performance.now() })
  const [speed, setSpeed] = useState<number | null>(null)

  useEffect(() => {
    if (downloaded < latest.current) {
      sample.current = { bytes: downloaded, time: performance.now() }
      setSpeed(null)
    }
    latest.current = downloaded
  }, [downloaded])

  useEffect(() => {
    const timer = window.setInterval(() => {
      const now = performance.now()
      setSpeed(
        (Math.max(0, latest.current - sample.current.bytes) * 1000) / (now - sample.current.time)
      )
      sample.current = { bytes: latest.current, time: now }
    }, 1000)
    return () => window.clearInterval(timer)
  }, [])

  const progress = total && total > 0 ? Math.min(100, Math.floor((downloaded / total) * 100)) : null
  return (
    <Progress value={progress} className="shrink-0 gap-2">
      <div className="flex w-full flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
        <ProgressLabel>下载进度{progress === null ? '' : ` ${progress}%`}</ProgressLabel>
        <span className="ml-auto flex flex-wrap justify-end gap-x-3 text-xs text-muted-foreground tabular-nums">
          {speed !== null && <span>{formatSize(speed)}/s</span>}
          <span>
            {formatSize(downloaded)} / {total && total > 0 ? formatSize(total) : '未知'}
          </span>
        </span>
      </div>
    </Progress>
  )
}
