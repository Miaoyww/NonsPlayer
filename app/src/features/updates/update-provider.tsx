import { createContext, useContext, useEffect, useRef, useState, type ReactNode } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { Download, RefreshCw } from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle
} from '@/components/ui/dialog'
import { ErrorNotice } from '@/components/error-notice'
import { SettingsCard } from '@/features/settings/settings-card'
import { errorText, nativeCall } from '@/lib/player'
import { isAndroid } from '@/lib/platform'

interface UpdateStatus {
  phase: 'idle' | 'checking' | 'current' | 'available' | 'downloading' | 'ready' | 'installing'
  version: string | null
  notes: string | null
  source: 'r2' | 'github'
  portable: boolean
  supported: boolean
  downloaded: number
  total: number | null
  error: string | null
}
const UpdateContext = createContext<{
  status?: UpdateStatus
  busy: boolean
  check: () => void
} | null>(null)

export function UpdateProvider({ children }: { children: ReactNode }) {
  const [status, setStatus] = useState<UpdateStatus>()
  const [open, setOpen] = useState(false)
  const [error, setError] = useState<string>()
  const [working, setWorking] = useState(false)
  const operation = useRef(false)
  const announced = useRef<string | null>(null)
  useEffect(() => {
    if (!isTauri() || isAndroid()) return
    let disposed = false
    let revision = 0
    const subscription = listen<UpdateStatus>('app-update', ({ payload }) => {
      revision++
      if (!disposed) setStatus(payload)
    })
    void subscription
      .then(async () => {
        const current = revision
        const snapshot = await nativeCall<UpdateStatus>('update_status')
        if (!disposed && current === revision) setStatus(snapshot)
      })
      .catch((cause) => {
        if (!disposed) setError(errorText(cause))
      })
    return () => {
      disposed = true
      void subscription.then((unlisten) => unlisten()).catch(() => {})
    }
  }, [])
  useEffect(() => {
    if (status?.version && status.version !== announced.current) {
      announced.current = status.version
      setOpen(true)
    }
  }, [status?.version])

  async function run(command: string) {
    if (operation.current) return
    operation.current = true
    setWorking(true)
    setError(undefined)
    try {
      await nativeCall(command)
    } catch (cause) {
      setError(errorText(cause))
    } finally {
      operation.current = false
      setWorking(false)
    }
  }
  const busy = working || ['checking', 'downloading', 'installing'].includes(status?.phase ?? '')
  const check = () => {
    setOpen(true)
    void run('update_check')
  }
  const progress = status?.total
    ? Math.min(100, Math.floor((status.downloaded / status.total) * 100))
    : undefined
  const message =
    status?.phase === 'checking'
      ? '正在检查新版本…'
      : status?.phase === 'current'
        ? '当前已是最新版本。'
        : status?.phase === 'downloading'
          ? `正在下载更新${progress === undefined ? '…' : ` ${progress}%`}`
          : status?.phase === 'ready'
            ? '更新已下载并通过签名验证。重启安装会中断当前播放。'
            : status?.phase === 'installing'
              ? '正在启动安装程序…'
              : status?.version
                ? status.portable
                  ? '便携版请下载新版压缩包，退出应用后解压，并保留原有 data 文件夹。'
                  : '下载完成后，可选择重启并安装更新。'
                : '启动后自动检查，也可以手动检查新版本。'

  return (
    <UpdateContext.Provider value={{ status, busy, check }}>
      {children}
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {status?.version ? `发现新版本 v${status.version}` : '版本更新'}
            </DialogTitle>
            <DialogDescription>{message}</DialogDescription>
          </DialogHeader>
          {status?.phase === 'downloading' && (
            <progress className="h-2 w-full" aria-label="更新下载进度" value={progress} max={100} />
          )}
          {status?.notes && (
            <div className="max-h-64 overflow-y-auto whitespace-pre-wrap break-words text-sm text-muted-foreground">
              {status.notes}
            </div>
          )}
          <p className="sr-only" role="status">
            {message}
          </p>
          {(error || status?.error) && <ErrorNotice error={error || status?.error || ''} />}
          <DialogFooter>
            <Button variant="outline" onClick={() => setOpen(false)}>
              稍后
            </Button>
            {status?.phase === 'ready' ? (
              <Button disabled={busy} onClick={() => void run('update_install')}>
                重启并安装
              </Button>
            ) : status?.version ? (
              <Button
                disabled={busy}
                onClick={() =>
                  void run(status.portable ? 'update_open_download' : 'update_download')
                }
              >
                <Download aria-hidden="true" />
                {status.portable
                  ? '下载便携版'
                  : status.phase === 'downloading'
                    ? '正在下载…'
                    : '下载更新'}
              </Button>
            ) : (
              <Button disabled={busy || !status?.supported} onClick={check}>
                <RefreshCw aria-hidden="true" />
                检查更新
              </Button>
            )}
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </UpdateContext.Provider>
  )
}

export function UpdateCard() {
  const updates = useContext(UpdateContext)
  const status = updates?.status
  const supported = isTauri() && !isAndroid() && status?.supported
  return (
    <SettingsCard
      title="版本更新"
      description={
        supported
          ? '启动后自动检查新版本，优先使用下载镜像，失败时切换至 GitHub。'
          : '自动更新支持 Windows x64，其他平台请从 GitHub 下载。'
      }
    >
      <Button
        variant="outline"
        size="sm"
        disabled={!supported || updates?.busy}
        onClick={updates?.check}
      >
        <RefreshCw aria-hidden="true" />
        {status?.phase === 'ready' ? '更新已就绪' : updates?.busy ? '正在处理…' : '检查更新'}
      </Button>
    </SettingsCard>
  )
}
