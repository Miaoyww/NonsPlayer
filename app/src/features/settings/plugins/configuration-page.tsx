import { ErrorNotice } from '@/components/error-notice'
import { TextSkeleton } from '@/components/music/loading'
import { useCallback, useEffect, useRef, useState } from 'react'
import { ArrowLeft, FolderOpen, RotateCw, Trash2 } from 'lucide-react'
import { listen } from '@tauri-apps/api/event'
import { isTauri } from '@tauri-apps/api/core'
import { Button } from '@/components/ui/button'
import { Switch } from '@/components/ui/switch'
import { errorText, nativeCall } from '@/lib/player'
import { PluginPageHost } from '@/plugins/host'
import { pluginPath, type PluginDescriptor } from '@/plugins/types'
import type { PluginSettings } from '@/plugins/configuration-types'
import { ConfigurationField } from './configuration-field'

export function PluginConfigurationPage({
  plugin,
  back
}: {
  plugin: PluginDescriptor
  back: () => void
}) {
  const [settings, setSettings] = useState<PluginSettings>()
  const [error, setError] = useState<string>()
  const [busy, setBusy] = useState(false)
  const [writable, setWritable] = useState(false)
  const [path, setPath] = useState<string>()
  const [resetEpoch, setResetEpoch] = useState(0)
  const current = useRef<PluginSettings | undefined>(undefined)
  const mounted = useRef(true)
  const tail = useRef(Promise.resolve())
  const request = useCallback(
    (operation: string, args: unknown = {}) =>
      nativeCall<PluginSettings>('plugin_settings', {
        id: plugin.manifest.id,
        generation: plugin.generation,
        operation,
        args
      }),
    [plugin.manifest.id, plugin.generation]
  )
  const accept = useCallback((value: PluginSettings) => {
    if (
      !mounted.current ||
      (value.snapshot &&
        current.current?.snapshot &&
        value.snapshot.revision < current.current.snapshot.revision)
    )
      return
    current.current = value
    setSettings(value)
  }, [])
  useEffect(() => {
    mounted.current = true
    let disposed = false
    let unlisten: (() => void) | undefined
    const refresh = () => {
      void request('get')
        .then((v) => {
          if (!disposed) accept(v)
        })
        .catch((e) => {
          if (!disposed) setError(errorText(e))
        })
    }
    if (isTauri())
      void listen<{ pluginId: string }>('plugin-config-changed', ({ payload }) => {
        if (payload.pluginId === plugin.manifest.id) refresh()
      })
        .then((cleanup) => {
          if (disposed) cleanup()
          else unlisten = cleanup
        })
        .catch((e) => {
          if (!disposed) setError(errorText(e))
        })
    refresh()
    return () => {
      disposed = true
      mounted.current = false
      unlisten?.()
    }
  }, [request, accept, plugin.manifest.id])
  function change(operation: string, args: unknown = {}) {
    const work = tail.current
      .catch(() => {})
      .then(async () => {
        if (mounted.current) setBusy(true)
        try {
          const revision = current.current?.snapshot?.revision
          accept(await request(operation, { ...(args as object), revision }))
          if (mounted.current) setError(undefined)
        } catch (error) {
          // Refresh the authoritative revision, retaining each field's failed draft.
          await request('get')
            .then(accept)
            .catch(() => {})
          throw error
        } finally {
          if (mounted.current) setBusy(false)
        }
      })
    tail.current = work
    return work
  }
  const definition = settings?.definition
  const snapshot = settings?.snapshot
  const customId = definition?.page.mode === 'custom' ? definition.page.pageId : undefined
  const custom = plugin.manifest.contributes.pages.find((p) => p.id === customId)
  const generated = definition?.page.mode === 'generated'
  const fields = definition?.fields.filter((f) => f.editor) ?? []
  const groups = [...new Set(fields.map((f) => f.group ?? ''))]
  return (
    <div className="space-y-6">
      <div>
        <Button variant="ghost" size="sm" className="-ml-3 mb-4" disabled={busy} onClick={back}>
          <ArrowLeft aria-hidden="true" />
          返回插件列表
        </Button>
        <h2 className="text-xl font-bold">{plugin.manifest.name} · 配置</h2>
        <p className="mt-2 text-sm text-muted-foreground">
          {generated ? '修改后自动保存。' : '管理插件配置与目录授权。'}
        </p>
      </div>
      {error && (
        <div className="space-y-2 text-sm text-destructive">
          <ErrorNotice error={error} />
          <Button
            variant="outline"
            size="sm"
            onClick={() => {
              void request('get')
                .then((v) => {
                  accept(v)
                  setError(undefined)
                })
                .catch((e) => setError(errorText(e)))
            }}
          >
            重试
          </Button>
        </div>
      )}
      {!settings && !error && <TextSkeleton />}
      {snapshot?.pendingReload && (
        <div
          role="status"
          className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-border bg-muted/30 p-4"
        >
          <p className="text-sm">配置已保存，部分设置需要重载插件后生效。</p>
          <Button
            variant="outline"
            disabled={busy}
            onClick={() => {
              void nativeCall('plugin_action', {
                id: plugin.manifest.id,
                action: 'reload',
                confirmed: false
              }).catch((e) => setError(errorText(e)))
            }}
          >
            <RotateCw aria-hidden="true" />
            重载插件
          </Button>
        </div>
      )}
      {generated && snapshot && (
        <div className="space-y-5">
          <div className="flex items-center justify-between">
            <h3 className="text-sm font-semibold">插件配置</h3>
            <Button
              variant="outline"
              size="sm"
              disabled={busy}
              onClick={() => {
                void change('reset')
                  .then(() => setResetEpoch((v) => v + 1))
                  .catch((e) => setError(errorText(e)))
              }}
            >
              全部恢复默认
            </Button>
          </div>
          {groups.map((group) => (
            <div key={group} className="rounded-xl border border-border px-5">
              {group && (
                <h3 className="border-b border-border py-4 text-sm font-semibold">{group}</h3>
              )}
              {fields
                .filter((f) => (f.group ?? '') === group)
                .map((field) => (
                  <ConfigurationField
                    key={`${resetEpoch}:${field.key}`}
                    field={field}
                    value={snapshot.values[field.key]}
                    diagnostic={snapshot.diagnostics[field.key]}
                    disabled={busy}
                    grants={settings?.grants}
                    authorize={(key) => change('grant', { writable: true, key })}
                    save={(key, value) => change('update', { patch: { [key]: value } })}
                    reset={(key) => change('reset', { keys: [key] })}
                  />
                ))}
            </div>
          ))}
          {fields.length === 0 && (
            <p className="text-sm text-muted-foreground">此插件没有可在统一页面编辑的配置项。</p>
          )}
        </div>
      )}
      {custom &&
        (plugin.loaded ? (
          <PluginPageHost
            path={pluginPath(plugin.manifest.id, path ?? custom.path)}
            navigate={setPath}
          />
        ) : (
          <p
            role="status"
            className="rounded-xl border border-border p-5 text-sm text-muted-foreground"
          >
            自定义配置页需要插件正常加载。请返回列表启用或重新加载插件。
          </p>
        ))}
      {plugin.manifest.permissions.includes('files:data') && (
        <div className="rounded-xl border border-border p-5">
          <h3 className="text-sm font-semibold">专属数据目录</h3>
          <p className="mt-2 text-xs leading-5 text-muted-foreground">
            插件可自行管理此目录中的文件，卸载时可以选择保留。
          </p>
          <Button
            variant="outline"
            className="mt-3"
            disabled={busy}
            onClick={() => {
              void change('open-data').catch((e) => setError(errorText(e)))
            }}
          >
            <FolderOpen aria-hidden="true" />
            打开数据目录
          </Button>
        </div>
      )}
      {plugin.manifest.permissions.includes('files:selected') && (
        <section className="rounded-xl border border-border p-5">
          <h3 className="text-sm font-semibold">外部目录授权</h3>
          <p className="mt-2 text-xs leading-5 text-muted-foreground">
            仅授权你选择的目录。撤销后插件无法继续访问；卸载不会清理这些目录。
          </p>
          <label className="my-4 flex items-center gap-2 text-sm">
            <Switch checked={writable} onCheckedChange={setWritable} disabled={busy} />
            允许新增、修改和删除文件
          </label>
          <Button
            variant="outline"
            disabled={busy}
            onClick={() => {
              void change('grant', { writable }).catch((e) => setError(errorText(e)))
            }}
          >
            <FolderOpen aria-hidden="true" />
            选择并授权目录
          </Button>
          <ul className="mt-4 space-y-3">
            {settings?.grants.map((grant) => (
              <li key={grant.id} className="flex items-center gap-3">
                <div className="min-w-0 flex-1">
                  <p className="break-all text-sm">{grant.path}</p>
                  <p className="text-xs text-muted-foreground">
                    {grant.id} · {grant.writable ? '读写' : '只读'}
                  </p>
                </div>
                <Button
                  variant="ghost"
                  size="icon"
                  disabled={busy}
                  aria-label={`撤销 ${grant.path} 的授权`}
                  title="撤销授权"
                  onClick={() => {
                    void change('revoke', { root: grant.id }).catch((e) => setError(errorText(e)))
                  }}
                >
                  <Trash2 aria-hidden="true" />
                </Button>
              </li>
            ))}
          </ul>
        </section>
      )}
    </div>
  )
}
