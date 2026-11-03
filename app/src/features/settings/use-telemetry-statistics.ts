import { useEffect, useState } from 'react'

const devSummaryPath = '/__dev/telemetry-summary'

interface TelemetryStatistics {
  usageCount?: string
  playCount?: string
  generatedAt?: string
}

function readCount(value: unknown) {
  if (typeof value === 'number')
    return Number.isSafeInteger(value) && value >= 0 ? String(value) : undefined
  return typeof value === 'string' && /^\d+$/.test(value) ? value : undefined
}

function parseTelemetryStatistics(value: unknown): TelemetryStatistics | undefined {
  if (!value || typeof value !== 'object') return
  const summary = value as Record<string, unknown>
  const usageCount = readCount(summary.usage_count)
  const playCount = readCount(summary.play_count)
  const generatedAt = typeof summary.generated_at === 'string' ? summary.generated_at.trim() : ''
  if (!usageCount || !playCount || !generatedAt) return
  return { usageCount, playCount, generatedAt }
}

const buildTimeStatistics: TelemetryStatistics = {
  usageCount: readCount(import.meta.env.VITE_TELEMETRY_USAGE_COUNT),
  playCount: readCount(import.meta.env.VITE_TELEMETRY_PLAY_COUNT),
  generatedAt:
    typeof import.meta.env.VITE_TELEMETRY_STATS_GENERATED_AT === 'string'
      ? import.meta.env.VITE_TELEMETRY_STATS_GENERATED_AT.trim()
      : undefined
}

export function useTelemetryStatistics() {
  const [statistics, setStatistics] = useState(buildTimeStatistics)

  useEffect(() => {
    if (!import.meta.env.DEV) return
    const controller = new AbortController()
    void fetch(devSummaryPath, { signal: controller.signal })
      .then(async (response) => {
        if (response.status === 204) return
        if (!response.ok) throw new Error(`HTTP ${response.status}`)
        const next = parseTelemetryStatistics(await response.json())
        if (!next) throw new Error('响应格式无效')
        setStatistics(next)
      })
      .catch((cause: unknown) => {
        if (!controller.signal.aborted) {
          console.warn('[telemetry] 本地开发统计快照获取失败', cause)
        }
      })
    return () => controller.abort()
  }, [])

  return statistics
}
