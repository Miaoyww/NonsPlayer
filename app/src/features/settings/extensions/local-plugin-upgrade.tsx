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

const permissionLabels: Record<string, string> = {
  websocket: 'WebSocket 长连接：可访问任意地址',
  'account:credentials': '读取当前平台账号凭据',
  'process:spawn': '启动任意程序并管理其输入输出',
  'music:write': '更改当前账号的收藏、歌单和推荐',
  'files:selected': '访问用户选择的外部目录'
}

function visiblePermissions(permissions: readonly string[]) {
  return permissions.filter(
    (permission) => permission !== 'http:request' && permission !== 'http:transfer'
  )
}

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
            {visiblePermissions(packageInfo?.permissions ?? []).map((permission) => (
              <li key={permission}>{permissionLabels[permission] ?? permission}</li>
            ))}
            {!!packageInfo?.fileRoots?.length && (
              <li>文件范围：{packageInfo.fileRoots.join('、')}</li>
            )}
          </ul>
          <p className="text-destructive">
            前端代码与播放器共享界面环境。任意网络地址、文件、账号凭据或程序执行权限可能允许访问或发送个人数据。请仅升级你信任的插件。
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
