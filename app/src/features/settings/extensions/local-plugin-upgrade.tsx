import { useEffect, useState } from 'react'
import { Button } from '@/components/ui/button'
import { ErrorNotice } from '@/components/error-notice'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle
} from '@/components/ui/dialog'
import type { PluginManifest } from '@/plugins/types'

export function LocalPluginUpgrade({
  packageInfo,
  installedVersion,
  error,
  busy,
  onClose,
  onConfirm
}: {
  packageInfo?: PluginManifest
  installedVersion?: string
  error?: string
  busy: boolean
  onClose: () => void
  onConfirm: () => void
}) {
  const [trusted, setTrusted] = useState(false)
  useEffect(() => setTrusted(false), [packageInfo])
  return (
    <Dialog open={!!packageInfo} onOpenChange={(open) => !open && !busy && onClose()}>
      <DialogContent showCloseButton={!busy} className="max-h-[85dvh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>升级插件</DialogTitle>
          <DialogDescription>
            {packageInfo?.name}：{installedVersion} → {packageInfo?.version}。保留配置和数据。
          </DialogDescription>
        </DialogHeader>
        <ErrorNotice error={error} />
        <div className="space-y-3 text-sm">
          <p>请确认新版本的权限范围：</p>
          <ul className="list-disc space-y-1 pl-5 text-muted-foreground">
            {packageInfo?.permissions.map((permission) => (
              <li key={permission}>{permission}</li>
            ))}
            {!!packageInfo?.httpHosts?.length && (
              <li>网络域名：{packageInfo.httpHosts.join('、')}</li>
            )}
            {!!packageInfo?.fileRoots?.length && (
              <li>文件范围：{packageInfo.fileRoots.join('、')}</li>
            )}
          </ul>
          <p className="text-destructive">
            前端代码与播放器共享界面环境。任意域名、文件、账号凭据或程序执行权限可能允许访问或发送个人数据。请仅升级你信任的插件。
          </p>
          <label className="flex items-start gap-2">
            <input
              type="checkbox"
              aria-label="同意升级插件权限"
              checked={trusted}
              disabled={busy}
              onChange={(event) => setTrusted(event.target.checked)}
              className="mt-0.5 size-4 accent-primary"
            />
            我已阅读并同意新版本的权限，信任此插件。
          </label>
        </div>
        <DialogFooter>
          <Button variant="outline" disabled={busy} onClick={onClose}>
            取消
          </Button>
          <Button disabled={!trusted || busy} onClick={onConfirm}>
            {busy ? '正在升级…' : '确认升级'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
