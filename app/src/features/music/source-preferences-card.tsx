import { hasPersonalPages } from './source-capabilities'
import { useCallback, useEffect, useRef, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { ArrowUp, RefreshCw } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Switch } from '@/components/ui/switch'
import type { ReactNode } from 'react'

function SourceSettingsSection({
  title,
  description,
  children
}: {
  title: string
  description: string
  children: ReactNode
}) {
  return (
    <section className="min-w-0 rounded-xl border border-border p-4">
      <h3 className="text-sm font-semibold">{title}</h3>
      <p className="mt-1 mb-3 text-xs leading-relaxed text-muted-foreground">{description}</p>
      {children}
    </section>
  )
}
import { errorText, nativeCall } from '@/lib/player'
import type { SourceDescriptor } from './types'
import {
  selectCurrentSource,
  sourcePreferences,
  type SourcePreferences
} from './source-preferences'

interface SourceStatus {
  descriptor: SourceDescriptor
  enabled: boolean
  loading?: boolean
  loadError?: string | null
}

export function SourcePreferencesCard() {
  const [preferences, setPreferences] = useState<SourcePreferences>()
  const [statuses, setStatuses] = useState<SourceStatus[]>([])
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)
  const generation = useRef(0)
  const mounted = useRef(false)
  const saving = useRef(false)
  const refresh = useCallback(async () => {
    const serial = ++generation.current
    try {
      if (!isTauri()) return
      const [next, adapters] = await Promise.all([
        sourcePreferences.get(),
        nativeCall<SourceStatus[]>('adapter_list')
      ])
      if (mounted.current && serial === generation.current) {
        setPreferences(next)
        setStatuses(adapters)
        setError('')
      }
    } catch (cause) {
      if (mounted.current && serial === generation.current) setError(errorText(cause))
    }
  }, [])
  useEffect(() => {
    mounted.current = true
    void refresh()
    let disposed = false
    const stops: (() => void)[] = []
    if (isTauri())
      for (const event of ['music-source-preferences-changed', 'adapters-changed']) {
        void listen(event, () => {
          void refresh()
        })
          .then((stop) => {
            if (disposed) stop()
            else stops.push(stop)
          })
          .catch((cause) => {
            if (!disposed) setError(errorText(cause))
          })
      }
    return () => {
      disposed = true
      mounted.current = false
      generation.current++
      stops.forEach((stop) => stop())
    }
  }, [refresh])

  async function save(next: SourcePreferences) {
    if (saving.current) return
    saving.current = true
    setBusy(true)
    setError('')
    ++generation.current
    try {
      await sourcePreferences.set(next)
      if (mounted.current) await refresh()
    } catch (cause) {
      if (mounted.current) setError(errorText(cause))
    } finally {
      saving.current = false
      if (mounted.current) setBusy(false)
    }
  }
  const names = (source: string) =>
    statuses.find((item) => item.descriptor.source === source)?.descriptor.displayName ?? source
  const unavailable =
    preferences &&
    !statuses.some((item) => item.enabled && item.descriptor.source === preferences.current)
  const availability = (source: string) => {
    const status = statuses.find((item) => item.descriptor.source === source)
    return status?.loading
      ? ' · 加载中'
      : status?.loadError
        ? ' · 加载失败'
        : !status?.enabled
          ? ' · 不可用'
          : ''
  }
  return (
    <div className="grid gap-3 lg:grid-cols-2">
      <SourceSettingsSection
        title="音乐库与发现"
        description="选择音乐库与发现页面的平台，歌曲操作始终使用歌曲所属平台的账号。"
      >
        <div className="flex flex-wrap items-center gap-2">
          {preferences && (
            <span className="text-sm">
              {names(preferences.current)}
              {unavailable ? availability(preferences.current) : ''}
            </span>
          )}
          {statuses
            .filter(
              (item) =>
                item.enabled &&
                hasPersonalPages(item.descriptor) &&
                item.descriptor.source !== preferences?.current
            )
            .map(({ descriptor }) => (
              <Button
                key={descriptor.source}
                variant="outline"
                size="sm"
                disabled={busy || !preferences}
                onClick={() => {
                  if (preferences) void save(selectCurrentSource(preferences, descriptor.source))
                }}
              >
                使用{descriptor.displayName}
              </Button>
            ))}
          <Button
            variant="ghost"
            size="sm"
            disabled={busy}
            onClick={() => {
              void refresh()
            }}
          >
            <RefreshCw aria-hidden="true" />
            刷新来源配置
          </Button>
        </div>
      </SourceSettingsSection>
      <SourceSettingsSection title="补充来源" description="保存补充来源及优先顺序。">
        <div className="flex flex-col gap-3">
          {preferences &&
            [
              ...new Set([
                ...preferences.supplementary,
                ...statuses
                  .filter((item) => item.descriptor.capabilities.includes('search'))
                  .map((item) => item.descriptor.source)
              ])
            ]
              .filter((source) => source !== preferences.current)
              .map((source) => {
                const index = preferences.supplementary.indexOf(source)
                const status = statuses.find((item) => item.descriptor.source === source)
                return (
                  <div key={source} className="flex flex-wrap items-center justify-between gap-3">
                    <span className="text-sm">
                      {index >= 0 ? `${index + 1}. ` : ''}
                      {names(source)}
                      {availability(source)}
                    </span>
                    <div className="flex items-center gap-2">
                      {index > 0 && (
                        <Button
                          variant="ghost"
                          size="sm"
                          disabled={busy}
                          onClick={() => {
                            const supplementary = [...preferences.supplementary]
                            ;[supplementary[index - 1], supplementary[index]] = [
                              supplementary[index],
                              supplementary[index - 1]
                            ]
                            void save({ ...preferences, supplementary })
                          }}
                        >
                          <ArrowUp aria-hidden="true" />
                          上移
                        </Button>
                      )}
                      <Switch
                        aria-label={`补充来源${names(source)}`}
                        checked={index >= 0}
                        disabled={busy || (index < 0 && !status?.enabled)}
                        onCheckedChange={(checked) => {
                          void save({
                            ...preferences,
                            supplementary: checked
                              ? [...preferences.supplementary, source]
                              : preferences.supplementary.filter((item) => item !== source)
                          })
                        }}
                      />
                    </div>
                  </div>
                )
              })}
          {preferences &&
            statuses.filter(
              (item) =>
                item.descriptor.source !== preferences.current &&
                item.descriptor.capabilities.includes('search')
            ).length === 0 &&
            preferences.supplementary.length === 0 && (
              <p className="text-sm text-muted-foreground">暂无其他支持搜索的音乐来源。</p>
            )}
        </div>
      </SourceSettingsSection>
      {error && (
        <p role="alert" className="text-sm text-destructive">
          {error}
        </p>
      )}
    </div>
  )
}
