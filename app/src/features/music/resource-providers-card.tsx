import { useState } from 'react'
import { Switch } from '@/components/ui/switch'
import { useResourceProviders } from './resource-providers'
import { useMusicSources } from './use-music-sources'
import { sourceLabel } from './source-label'
import { errorText, nativeCall } from '@/lib/player'

export function ResourceProvidersCard() {
  const { providers, error: loadError } = useResourceProviders()
  const { statuses } = useMusicSources()
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string>()
  async function enable(id: string, enabled: boolean) {
    setBusy(true)
    setError(undefined)
    try {
      await nativeCall('resource_provider_set_enabled', { id, enabled })
    } catch (cause) {
      setError(errorText(cause))
    } finally {
      setBusy(false)
    }
  }
  return (
    <section aria-label="独立音源适配器" className="rounded-xl border border-border p-4">
      <h3 className="font-semibold">独立音源适配器</h3>
      <p className="mt-1 text-sm text-muted-foreground">
        官方音源获取失败时尝试已启用的备用音源。切换会明确显示实际音源；歌曲所属平台和收藏对象保留。
      </p>
      {(error || loadError) && (
        <p role="alert" className="mt-2 text-sm text-destructive">
          {error ?? loadError}
        </p>
      )}
      {providers.map(({ descriptor, enabled, loading, loadError }) => (
        <div key={descriptor.id} className="mt-4 flex items-center justify-between gap-4">
          <div>
            <h4 className="text-sm font-semibold">{descriptor.displayName}</h4>
            {loading && (
              <p role="status" className="text-xs text-muted-foreground">
                正在加载音源…
              </p>
            )}
            {loadError && (
              <p role="alert" className="text-xs text-destructive">
                {loadError}
              </p>
            )}
            <p className="text-xs text-muted-foreground">
              {descriptor.platforms
                .map((p) =>
                  sourceLabel(
                    p,
                    statuses.map((s) => s.descriptor)
                  )
                )
                .join('、')}{' '}
              · {descriptor.searchPlatforms.length ? '支持歌曲搜索' : '仅解析播放资源'}
            </p>
          </div>
          <Switch
            aria-label={`启用${descriptor.displayName}音源`}
            checked={enabled}
            disabled={busy || loading || !!loadError}
            onCheckedChange={(value) => void enable(descriptor.id, value)}
          />
        </div>
      ))}
      {!providers.length && (
        <p className="mt-3 text-sm text-muted-foreground">暂无已加载的独立音源。</p>
      )}
    </section>
  )
}
