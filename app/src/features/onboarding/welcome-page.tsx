import { motion } from 'motion/react'
import { Blocks, Check, ChevronRight, Music2, Package, Sparkles } from 'lucide-react'
import { useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { Button } from '@/components/ui/button'
import { Switch } from '@/components/ui/switch'
import { MusicPage } from '@/components/music/music-page'
import { ErrorNotice } from '@/components/error-notice'
import { errorText, nativeCall } from '@/lib/player'
import { usePlugins } from '@/plugins/host'
import type { PluginDescriptor } from '@/plugins/types'
import { sourcePreferences } from '@/features/music/source-preferences'
import { useMusicSources } from '@/features/music/use-music-sources'
import { sourceLabel } from '@/features/music/source-label'
import { markWelcomeCompleted } from './welcome-state'

function capabilityLabel(capability: string) {
  return (
    {
      search: '搜索',
      browse: '浏览',
      account: '账号',
      userLibrary: '音乐库',
      recommendations: '推荐',
      lyrics: '歌词'
    }[capability] ?? capability
  )
}

function permissionLabel(permission: string) {
  return (
    {
      'clipboard:read': '读取剪贴板',
      'http:request': '请求网络',
      'music:metadata': '读取音乐信息',
      'player:read': '读取播放状态',
      'player:control': '控制播放',
      storage: '保存插件数据',
      ui: '显示插件界面',
      config: '保存插件设置'
    }[permission] ?? permission
  )
}

function PluginReview({
  plugin,
  trusted,
  busy,
  onTrustedChange,
  onConfirm,
  onCancel
}: {
  plugin: PluginDescriptor
  trusted: boolean
  busy: boolean
  onTrustedChange: (value: boolean) => void
  onConfirm: () => void
  onCancel: () => void
}) {
  return (
    <div className="welcome-review">
      <p className="font-semibold">启用前确认</p>
      <p className="mt-1 text-xs leading-relaxed text-muted-foreground">
        前端插件与播放器运行在同一界面环境，只启用你信任的插件。
      </p>
      <div className="mt-3 flex flex-wrap gap-1.5">
        {plugin.manifest.permissions.length ? (
          plugin.manifest.permissions.map((permission) => (
            <span key={permission} className="welcome-chip">
              {permissionLabel(permission)}
            </span>
          ))
        ) : (
          <span className="welcome-chip">无需额外权限</span>
        )}
      </div>
      <label className="mt-3 flex min-h-11 items-center gap-2 text-sm">
        <Switch aria-label="同意插件权限" checked={trusted} onCheckedChange={onTrustedChange} />
        我信任此插件并同意以上权限
      </label>
      <div className="mt-2 flex flex-wrap gap-2">
        <Button size="sm" disabled={!trusted || busy} onClick={onConfirm}>
          确认启用
        </Button>
        <Button size="sm" variant="ghost" disabled={busy} onClick={onCancel}>
          取消
        </Button>
      </div>
    </div>
  )
}

function AdapterCard({
  source,
  selected,
  busy,
  onSelect
}: {
  source: ReturnType<typeof useMusicSources>['statuses'][number]
  selected: boolean
  busy: boolean
  onSelect: () => void
}) {
  const capabilities = source.descriptor.capabilities
    .filter((capability) => capabilityLabel(capability) !== capability)
    .slice(0, 4)
  return (
    <button
      type="button"
      className={`welcome-option ${selected ? 'welcome-option-selected' : ''}`}
      aria-pressed={selected}
      disabled={busy || !source.enabled}
      onClick={onSelect}
    >
      <span className="welcome-option-icon">
        {selected ? <Check aria-hidden="true" /> : <Music2 aria-hidden="true" />}
      </span>
      <span className="min-w-0 flex-1 text-left">
        <span className="flex flex-wrap items-center gap-2">
          <span className="font-semibold">{source.descriptor.displayName}</span>
          {selected && <span className="welcome-status">当前来源</span>}
          {!source.enabled && <span className="welcome-status">已停用</span>}
        </span>
        <span className="mt-1 block text-xs leading-relaxed text-muted-foreground">
          {source.descriptor.description ?? '音乐来源适配器'}
        </span>
        {capabilities.length > 0 && (
          <span className="mt-3 flex flex-wrap gap-1.5">
            {capabilities.map((capability) => (
              <span key={capability} className="welcome-chip">
                {capabilityLabel(capability)}
              </span>
            ))}
          </span>
        )}
      </span>
      <ChevronRight aria-hidden="true" className="welcome-option-arrow" />
    </button>
  )
}

function PluginCard({
  plugin,
  review,
  busy,
  onToggle,
  onConfirm,
  onCancel,
  trusted,
  onTrustedChange
}: {
  plugin: PluginDescriptor
  review: boolean
  busy: boolean
  onToggle: (enabled: boolean) => void
  onConfirm: () => void
  onCancel: () => void
  trusted: boolean
  onTrustedChange: (value: boolean) => void
}) {
  return (
    <article className={`welcome-option welcome-plugin ${review ? 'welcome-option-selected' : ''}`}>
      <span className="welcome-option-icon">
        {plugin.enabled ? <Check aria-hidden="true" /> : <Package aria-hidden="true" />}
      </span>
      <span className="min-w-0 flex-1">
        <span className="flex flex-wrap items-center gap-2">
          <span className="font-semibold">{plugin.manifest.name}</span>
          {plugin.enabled && <span className="welcome-status">已启用</span>}
          {plugin.error && <span className="welcome-status welcome-status-error">加载异常</span>}
        </span>
        <span className="mt-1 block text-xs leading-relaxed text-muted-foreground">
          {plugin.manifest.description ?? '功能插件'}
        </span>
        <span className="mt-3 flex flex-wrap gap-1.5">
          {plugin.manifest.permissions.slice(0, 4).map((permission) => (
            <span key={permission} className="welcome-chip">
              {permissionLabel(permission)}
            </span>
          ))}
        </span>
        {review && (
          <PluginReview
            plugin={plugin}
            trusted={trusted}
            busy={busy}
            onTrustedChange={onTrustedChange}
            onConfirm={onConfirm}
            onCancel={onCancel}
          />
        )}
      </span>
      <Switch
        aria-label={`${plugin.enabled ? '停用' : '启用'}${plugin.manifest.name}`}
        checked={plugin.enabled}
        disabled={busy}
        onCheckedChange={onToggle}
      />
    </article>
  )
}

export function WelcomePage({ onComplete }: { onComplete: () => void }) {
  const sources = useMusicSources()
  const { plugins } = usePlugins()
  const [busy, setBusy] = useState<string>()
  const [review, setReview] = useState<string>()
  const [trusted, setTrusted] = useState(false)
  const [error, setError] = useState<string>()

  const selectSource = async (source: string) => {
    if (busy || !isTauri()) return
    setBusy(`source:${source}`)
    setError(undefined)
    try {
      const intent = await sourcePreferences.beginIntent()
      await sourcePreferences.use(source, intent)
    } catch (cause) {
      setError(errorText(cause))
    } finally {
      setBusy(undefined)
    }
  }

  const togglePlugin = async (plugin: PluginDescriptor, enabled: boolean) => {
    if (busy) return
    if (enabled) {
      setReview(plugin.manifest.id)
      setTrusted(false)
      return
    }
    setBusy(`plugin:${plugin.manifest.id}`)
    setError(undefined)
    try {
      await nativeCall('plugin_action', { id: plugin.manifest.id, action: 'disable' })
    } catch (cause) {
      setError(errorText(cause))
    } finally {
      setBusy(undefined)
    }
  }

  const confirmPlugin = async () => {
    if (!review || !trusted) return
    setBusy(`plugin:${review}`)
    setError(undefined)
    try {
      await nativeCall('plugin_action', { id: review, action: 'enable', confirmed: true })
      setReview(undefined)
    } catch (cause) {
      setError(errorText(cause))
    } finally {
      setBusy(undefined)
    }
  }

  const finish = () => {
    markWelcomeCompleted()
    onComplete()
  }

  const current = sources.preferences?.current
  const availableSources = sources.statuses.filter((item) => item.descriptor.source !== 'local')
  return (
    <MusicPage className="welcome-page" aria-label="欢迎页">
      <motion.div
        className="welcome-shell"
        initial={{ opacity: 0, y: 12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.35, ease: 'easeOut' }}
      >
        <header className="welcome-hero">
          <div className="welcome-mark" aria-hidden="true">
            <Sparkles />
          </div>
          <div className="min-w-0 flex-1">
            <p className="welcome-eyebrow">NONS PLAYER / GET STARTED</p>
            <h1>先把你的音乐空间准备好</h1>
            <p>选择一个音乐适配器，再决定哪些插件陪你使用。之后也可以在设置中随时调整。</p>
          </div>
          <div className="welcome-count" aria-label="欢迎页步骤">
            <span>01</span>
            <span className="welcome-count-line" />
            <span>02</span>
          </div>
        </header>

        {error && <ErrorNotice error={error} />}

        <div className="welcome-grid">
          <section className="welcome-section" aria-labelledby="welcome-adapters-heading">
            <div className="welcome-section-heading">
              <div>
                <p className="welcome-section-kicker">MUSIC SOURCES</p>
                <h2 id="welcome-adapters-heading">选择适配器</h2>
                <p>适配器决定音乐库、搜索和播放从哪里读取。</p>
              </div>
              <Blocks aria-hidden="true" />
            </div>
            <div className="welcome-options" aria-busy={sources.loading}>
              {sources.loading ? (
                <div className="welcome-empty">
                  <span className="welcome-pulse" />
                  正在读取已安装的适配器…
                </div>
              ) : availableSources.length ? (
                availableSources.map((source) => (
                  <AdapterCard
                    key={source.descriptor.source}
                    source={source}
                    selected={current === source.descriptor.source}
                    busy={busy?.startsWith('source:') ?? false}
                    onSelect={() => void selectSource(source.descriptor.source)}
                  />
                ))
              ) : (
                <div className="welcome-empty">
                  <Music2 aria-hidden="true" />
                  <span>暂时没有可用的音乐适配器。</span>
                </div>
              )}
            </div>
          </section>

          <section className="welcome-section" aria-labelledby="welcome-plugins-heading">
            <div className="welcome-section-heading">
              <div>
                <p className="welcome-section-kicker">EXTENSIONS</p>
                <h2 id="welcome-plugins-heading">选择插件</h2>
                <p>插件可以把播放器扩展成更适合你的工作台。</p>
              </div>
              <Package aria-hidden="true" />
            </div>
            <div className="welcome-options">
              {plugins.length ? (
                plugins.map((plugin) => (
                  <PluginCard
                    key={plugin.manifest.id}
                    plugin={plugin}
                    review={review === plugin.manifest.id}
                    busy={busy === `plugin:${plugin.manifest.id}`}
                    trusted={trusted}
                    onTrustedChange={setTrusted}
                    onToggle={(enabled) => void togglePlugin(plugin, enabled)}
                    onConfirm={() => void confirmPlugin()}
                    onCancel={() => {
                      setReview(undefined)
                      setTrusted(false)
                    }}
                  />
                ))
              ) : (
                <div className="welcome-empty">
                  <Package aria-hidden="true" />
                  <span>暂时没有已安装的插件。</span>
                </div>
              )}
            </div>
          </section>
        </div>

        <footer className="welcome-footer">
          <p>
            {current
              ? `当前来源：${sourceLabel(
                  current,
                  sources.statuses.map((item) => item.descriptor)
                )}`
              : '还没有选择音乐来源'}
          </p>
          <div className="flex flex-wrap items-center gap-2">
            <Button variant="ghost" onClick={finish}>
              先跳过
            </Button>
            <Button className="gap-2 rounded-full px-5" onClick={finish}>
              进入 Nons
              <ChevronRight aria-hidden="true" />
            </Button>
          </div>
        </footer>
      </motion.div>
    </MusicPage>
  )
}
