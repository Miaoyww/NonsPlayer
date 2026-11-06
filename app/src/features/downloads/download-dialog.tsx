import { errorText } from '@/lib/player'
import { reportCaughtError } from '@/lib/diagnostics'
import { TrackIdentity } from '@/components/music/track-identity'
import { Button } from '@/components/ui/button'
import { Progress } from '@/components/ui/progress'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle
} from '@/components/ui/dialog'
import {
  CheckCircle2,
  CircleAlert,
  FolderOpen,
  LoaderCircle,
  RotateCw,
  Trash2,
  X
} from 'lucide-react'
import { cn } from 'cn'
import { useState } from 'react'
import { useDownloads, type DownloadState, type DownloadTask } from './download-provider'

const labels: Record<DownloadState, string> = {
  waiting: '等待下载',
  running: '下载中',
  completed: '已完成',
  failed: '下载失败',
  cancelled: '已取消'
}

const qualityLabels: Record<string, string> = {
  standard: '标准',
  higher: '较高',
  exhigh: '极高',
  lossless: '无损',
  hires: 'Hi-Res'
}

function bytesText(bytes: number) {
  return `${(bytes / 1048576).toFixed(1)} MiB`
}

function percent(task: DownloadTask) {
  if (task.state === 'completed') return 100
  if (!task.total) return null
  return Math.min(100, Math.floor((task.bytes / task.total) * 100))
}

function sorted(tasks: DownloadTask[]) {
  const rank: Record<DownloadState, number> = {
    running: 0,
    waiting: 1,
    failed: 2,
    cancelled: 3,
    completed: 4
  }
  return [...tasks].sort((a, b) => rank[a.state] - rank[b.state] || b.id - a.id)
}

export function DownloadDialog() {
  const downloads = useDownloads()
  const [busy, setBusy] = useState<number | string>()
  const [error, setError] = useState<string>()
  const tasks = sorted(downloads.tasks)
  const finished = tasks.some((task) => ['completed', 'failed', 'cancelled'].includes(task.state))
  async function run(key: number | string, action: () => Promise<unknown>) {
    if (busy !== undefined) return
    setBusy(key)
    setError(undefined)
    try {
      await action()
    } catch (cause) {
      reportCaughtError('app/src/features/downloads/download-dialog.tsx', cause)
      setError(errorText(cause))
    } finally {
      setBusy(undefined)
    }
  }
  return (
    <Dialog open={downloads.open} onOpenChange={downloads.setOpen}>
      <DialogContent className="flex max-h-[80dvh] flex-col overflow-hidden sm:max-w-2xl">
        <DialogHeader className="shrink-0 pr-8">
          <DialogTitle>下载进度</DialogTitle>
          <DialogDescription>
            下载完成后自动写入标签，并加入本地音乐库；关闭弹窗后下载继续，可从标题栏重新打开。
          </DialogDescription>
        </DialogHeader>
        {error && <p className="text-sm text-destructive">{error}</p>}
        <div className="-mx-2 min-h-0 flex-1 overflow-auto px-2">
          {tasks.length ? (
            <div className="flex flex-col gap-2">
              {tasks.map((task) => (
                <article
                  key={task.id}
                  className={cn(
                    'download-task-card rounded-2xl border border-border/70 bg-muted/20',
                    task.state === 'completed' ? 'p-3' : 'p-4'
                  )}
                >
                  <div className="flex min-w-0 items-center gap-3">
                    <div className="min-w-0 flex-1">
                      <TrackIdentity track={task.track} />
                    </div>
                    {task.state === 'running' || task.state === 'waiting' ? (
                      <div className="flex min-w-0 flex-1 flex-col gap-1.5">
                        <Progress
                          value={percent(task)}
                          aria-label={`${task.track.title} 下载进度`}
                        />
                        <p className="flex justify-between text-xs tabular-nums text-muted-foreground">
                          <span>
                            {bytesText(task.bytes)}
                            {task.total ? ` / ${bytesText(task.total)}` : ''}
                          </span>
                          <span>
                            {task.actualQuality
                              ? (qualityLabels[task.actualQuality] ?? task.actualQuality)
                              : (qualityLabels[task.quality] ?? task.quality)}
                          </span>
                        </p>
                      </div>
                    ) : null}
                    <span className="ml-auto flex shrink-0 items-center gap-1.5 text-xs text-muted-foreground">
                      {task.state === 'running' || task.state === 'waiting' ? (
                        <LoaderCircle className="size-3.5 motion-safe:animate-spin" aria-hidden />
                      ) : task.state === 'completed' ? (
                        <CheckCircle2 className="size-3.5 text-[var(--music-accent)]" aria-hidden />
                      ) : (
                        <CircleAlert className="size-3.5 text-destructive" aria-hidden />
                      )}
                      {labels[task.state]}
                    </span>
                    <div className="flex shrink-0 items-center gap-0.5">
                      {task.state === 'completed' && (
                        <Button
                          variant="ghost"
                          size="icon-sm"
                          disabled={busy !== undefined}
                          aria-label={`在文件夹中显示 ${task.track.title}`}
                          title="在文件夹中显示"
                          onClick={() => void run(task.id, () => downloads.reveal(task.id))}
                        >
                          <FolderOpen aria-hidden="true" />
                        </Button>
                      )}
                      {(task.state === 'running' || task.state === 'waiting') && (
                        <Button
                          variant="ghost"
                          size="icon-sm"
                          disabled={busy !== undefined}
                          aria-label={`取消下载 ${task.track.title}`}
                          title="取消下载"
                          onClick={() => void run(task.id, () => downloads.cancel(task.id))}
                        >
                          <X aria-hidden="true" />
                        </Button>
                      )}
                      {(task.state === 'failed' || task.state === 'cancelled') && (
                        <Button
                          variant="ghost"
                          size="icon-sm"
                          disabled={busy !== undefined}
                          aria-label={`重试下载 ${task.track.title}`}
                          title="重试下载"
                          onClick={() => void run(task.id, () => downloads.retry(task.id))}
                        >
                          <RotateCw aria-hidden="true" />
                        </Button>
                      )}
                      {['completed', 'failed', 'cancelled'].includes(task.state) && (
                        <Button
                          variant="ghost"
                          size="icon-sm"
                          disabled={busy !== undefined}
                          aria-label={`移除下载记录 ${task.track.title}`}
                          title="移除记录"
                          onClick={() => void run(task.id, () => downloads.remove(task.id))}
                        >
                          <Trash2 aria-hidden="true" />
                        </Button>
                      )}
                    </div>
                  </div>
                  {task.error && <p className="mt-1.5 text-xs text-destructive">{task.error}</p>}
                  {task.warning && (
                    <p className="mt-1.5 text-xs text-amber-600 dark:text-amber-400">
                      {task.warning}
                    </p>
                  )}
                </article>
              ))}
            </div>
          ) : (
            <p className="py-12 text-center text-sm text-muted-foreground">
              暂无下载任务，右键在线歌曲选择“下载”即可开始。
            </p>
          )}
        </div>
        <DialogFooter className="shrink-0">
          {finished && (
            <Button
              variant="outline"
              disabled={busy !== undefined}
              onClick={() => void run('clear', () => downloads.clearFinished())}
            >
              清除已结束
            </Button>
          )}
          <Button variant="outline" onClick={() => downloads.setOpen(false)}>
            关闭
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
