import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { writeText } from '@tauri-apps/plugin-clipboard-manager'
import { openUrl } from '@tauri-apps/plugin-opener'
import { BookOpen, Copy, Globe2, Link2, Server } from 'lucide-react'
import { useCallback, useEffect, useRef, useState } from 'react'
import { ErrorNotice } from '@/components/error-notice'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Switch } from '@/components/ui/switch'
import { SettingsCard } from '@/features/settings/shell/settings-card'
import { reportCaughtError } from '@/lib/diagnostics'
import { errorText, nativeCall } from '@/lib/player'

type OpenApiConfig = {
  enabled: boolean
  port: number
  lan: boolean
}

type OpenApiStatus = {
  config: OpenApiConfig
  running: boolean
  addresses: string[]
  connections: number
  error: string | null
}

const defaultConfig: OpenApiConfig = {
  enabled: false,
  port: 46321,
  lan: false
}

function httpAddress(address: string) {
  const normalized = address.replace(/\/+$/, '')
  return /^https?:\/\//.test(normalized) ? normalized : `http://${normalized}`
}

function websocketAddress(address: string) {
  return `${httpAddress(address).replace(/^http/, 'ws')}/v1/events`
}

export function NetworkServicePage() {
  const [status, setStatus] = useState<OpenApiStatus>()
  const [port, setPort] = useState(String(defaultConfig.port))
  const [lan, setLan] = useState(defaultConfig.lan)
  const [loadingError, setLoadingError] = useState<string>()
  const [saveError, setSaveError] = useState<string>()
  const [actionError, setActionError] = useState<string>()
  const [loading, setLoading] = useState(false)
  const [saving, setSaving] = useState(false)
  const [copied, setCopied] = useState<string>()
  const busy = useRef(false)
  const mounted = useRef(true)
  const dirtyRef = useRef(false)
  const copyTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)

  const acceptStatus = useCallback((value: OpenApiStatus) => {
    setStatus(value)
    if (!dirtyRef.current) {
      setPort(String(value.config.port))
      setLan(value.config.lan)
    }
  }, [])

  const loadStatus = useCallback(async () => {
    if (!isTauri()) {
      setStatus({
        config: defaultConfig,
        running: false,
        addresses: [],
        connections: 0,
        error: null
      })
      return
    }
    setLoading(true)
    setLoadingError(undefined)
    try {
      const value = await nativeCall<OpenApiStatus>('open_api_status')
      if (mounted.current) acceptStatus(value)
    } catch (cause) {
      reportCaughtError('app/src/features/settings/network-service/network-service-page.tsx', cause)
      if (mounted.current) setLoadingError(errorText(cause))
    } finally {
      if (mounted.current) setLoading(false)
    }
  }, [acceptStatus])

  useEffect(() => {
    mounted.current = true
    void loadStatus()
    let stop: (() => void) | undefined
    if (isTauri())
      void listen<OpenApiStatus>('open-api-status-changed', ({ payload }) => {
        if (mounted.current) acceptStatus(payload)
      })
        .then((unlisten) => {
          if (mounted.current) stop = unlisten
          else unlisten()
        })
        .catch((cause) =>
          reportCaughtError(
            'app/src/features/settings/network-service/network-service-page.tsx',
            cause
          )
        )
    return () => {
      mounted.current = false
      stop?.()
      clearTimeout(copyTimer.current)
    }
  }, [acceptStatus, loadStatus])

  const portNumber = Number(port)
  const portError =
    !Number.isInteger(portNumber) || portNumber < 1 || portNumber > 65535
      ? '请输入 1–65535 之间的端口号。'
      : undefined
  const dirty = !!status && (portNumber !== status.config.port || lan !== status.config.lan)
  dirtyRef.current = dirty

  async function configure(patch: Partial<OpenApiConfig>) {
    if (!status || busy.current) return
    const usesDraft = patch.port !== undefined || patch.lan !== undefined
    const config = usesDraft
      ? {
          ...status.config,
          ...patch,
          port: patch.port ?? Number(port),
          lan: patch.lan ?? lan
        }
      : { ...status.config, ...patch }
    if (!Number.isInteger(config.port) || config.port < 1 || config.port > 65535) return
    busy.current = true
    setSaving(true)
    setSaveError(undefined)
    try {
      const value = await nativeCall<OpenApiStatus>('open_api_configure', { config })
      setStatus(value)
      setPort(String(value.config.port))
      setLan(value.config.lan)
    } catch (cause) {
      reportCaughtError('app/src/features/settings/network-service/network-service-page.tsx', cause)
      setSaveError(errorText(cause))
    } finally {
      busy.current = false
      setSaving(false)
    }
  }

  async function copy(value: string, key: string) {
    try {
      if (isTauri()) await writeText(value)
      else await navigator.clipboard.writeText(value)
      setCopied(key)
      clearTimeout(copyTimer.current)
      copyTimer.current = setTimeout(() => setCopied(undefined), 1800)
    } catch (cause) {
      reportCaughtError('app/src/features/settings/network-service/network-service-page.tsx', cause)
      setActionError(errorText(cause))
    }
  }

  async function openDocs() {
    const address = status?.addresses[0]
    if (!address) return
    const url = `${httpAddress(address)}/v1/docs`
    setActionError(undefined)
    try {
      if (isTauri()) await openUrl(url)
      else window.open(url, '_blank', 'noopener,noreferrer')
    } catch (cause) {
      reportCaughtError('app/src/features/settings/network-service/network-service-page.tsx', cause)
      setActionError(errorText(cause))
    }
  }

  const statusLabel = status?.error
    ? '启动失败'
    : status?.running
      ? '运行中'
      : status?.config.enabled
        ? '正在启动'
        : '已关闭'
  const statusTone = status?.error
    ? 'bg-destructive/10 text-destructive'
    : status?.running
      ? 'bg-emerald-500/10 text-emerald-700 dark:text-emerald-300'
      : 'bg-muted text-muted-foreground'

  return (
    <div className="flex flex-col gap-8">
      <div>
        <h2 className="text-xl font-bold">网络与服务</h2>
        <p className="mt-2 text-sm text-muted-foreground">
          无需身份验证，允许连接的应用可读取信息并控制播放。
        </p>
      </div>
      <section aria-labelledby="network-service-heading" className="flex flex-col gap-8">
        <h3 id="network-service-heading" className="text-base font-semibold">
          开放 API
        </h3>
        <SettingsCard
          searchId="open-api-enabled"
          title="启用开放 API"
          description="启动本机 HTTP 与 WebSocket 服务。"
        >
          <Switch
            aria-label="启用开放 API"
            checked={status?.config.enabled ?? false}
            disabled={!status || saving}
            onCheckedChange={(value) => void configure({ enabled: value })}
          />
        </SettingsCard>
        <SettingsCard
          searchId="open-api-docs"
          title="协议文档"
          description="在浏览器中查看接口说明。"
        >
          <div className="flex flex-col items-end gap-2">
            <Button
              variant="outline"
              size="sm"
              disabled={!status?.running || !status.addresses.length}
              onClick={() => void openDocs()}
            >
              <BookOpen aria-hidden="true" />
              打开文档
            </Button>
            {actionError && <ErrorNotice error={actionError} />}
          </div>
        </SettingsCard>
        <SettingsCard
          searchId="open-api-status"
          title="服务状态"
          description="当前运行状态、访问地址与连接数。"
        >
          <div className="flex w-full flex-col gap-3 lg:w-[32rem]">
            <div className="flex flex-wrap items-center gap-3">
              <span
                className={`inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 text-xs font-medium ${statusTone}`}
                aria-live="polite"
              >
                <Server className="size-3.5" aria-hidden="true" />
                {statusLabel}
              </span>
              <span className="text-xs text-muted-foreground">
                {status?.connections ?? 0} 个连接
              </span>
            </div>
            {status?.error && <ErrorNotice error={status.error} />}
            {loadingError && (
              <div className="flex items-center justify-between gap-3">
                <ErrorNotice error={loadingError} />
                <Button
                  variant="outline"
                  size="sm"
                  disabled={loading}
                  onClick={() => void loadStatus()}
                >
                  重试
                </Button>
              </div>
            )}
            {status?.running && status.addresses.length ? (
              <div className="flex flex-col gap-3" aria-label="开放 API 地址">
                {status.addresses.map((address) => {
                  const http = httpAddress(address)
                  const websocket = websocketAddress(address)
                  return (
                    <div key={address} className="rounded-md border border-border/70 p-3">
                      <div className="flex items-center gap-2">
                        <Globe2 className="size-4 text-muted-foreground" aria-hidden="true" />
                        <code className="min-w-0 flex-1 break-all text-xs">{http}</code>
                        <Button
                          variant="ghost"
                          size="icon-sm"
                          aria-label={`复制 HTTP 地址 ${http}`}
                          title={copied === `http:${http}` ? '已复制' : '复制 HTTP 地址'}
                          onClick={() => void copy(http, `http:${http}`)}
                        >
                          <Copy aria-hidden="true" />
                        </Button>
                      </div>
                      <div className="mt-2 flex items-center gap-2">
                        <Link2 className="size-4 text-muted-foreground" aria-hidden="true" />
                        <code className="min-w-0 flex-1 break-all text-xs">{websocket}</code>
                        <Button
                          variant="ghost"
                          size="icon-sm"
                          aria-label={`复制 WebSocket 地址 ${websocket}`}
                          title={copied === `ws:${websocket}` ? '已复制' : '复制 WebSocket 地址'}
                          onClick={() => void copy(websocket, `ws:${websocket}`)}
                        >
                          <Copy aria-hidden="true" />
                        </Button>
                      </div>
                    </div>
                  )
                })}
              </div>
            ) : (
              !status?.error && (
                <p className="text-xs text-muted-foreground">
                  {status?.config.enabled ? '服务启动后会在这里显示地址。' : '启用后显示访问地址。'}
                </p>
              )
            )}
          </div>
        </SettingsCard>
        <SettingsCard
          searchId="open-api-listen"
          title="监听范围"
          description="默认仅本机可连接；允许局域网后，同一网络中的设备也能访问。"
        >
          <div className="flex w-72 flex-col items-stretch gap-3">
            <div className="flex flex-col gap-1.5">
              <label htmlFor="open-api-port" className="text-xs font-medium">
                端口
              </label>
              <Input
                id="open-api-port"
                type="number"
                min={1}
                max={65535}
                step={1}
                value={port}
                disabled={!status || saving}
                aria-invalid={!!portError}
                aria-describedby={portError ? 'open-api-port-error' : undefined}
                onChange={(event) => setPort(event.currentTarget.value)}
              />
              {portError && (
                <p id="open-api-port-error" className="text-xs text-destructive">
                  {portError}
                </p>
              )}
            </div>
            <label className="flex items-center justify-between gap-4 text-sm">
              <span>允许局域网访问</span>
              <Switch
                aria-label="允许局域网访问"
                checked={lan}
                disabled={!status || saving}
                onCheckedChange={setLan}
              />
            </label>
            <Button
              className="self-end"
              size="sm"
              disabled={!status || saving || !!portError || !dirty}
              onClick={() => void configure({ port: portNumber, lan })}
            >
              {saving ? '保存中…' : '应用更改'}
            </Button>
            {saveError && <ErrorNotice error={saveError} />}
          </div>
        </SettingsCard>
      </section>
    </div>
  )
}
