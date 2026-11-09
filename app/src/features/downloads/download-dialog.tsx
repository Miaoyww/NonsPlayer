import { errorText, nativeCall } from '@/lib/player'
import { reportCaughtError } from '@/lib/diagnostics'
import { TrackIdentity } from '@/components/music/track-identity'
import { Button } from '@/components/ui/button'
import { Progress } from '@/components/ui/progress'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import {
  Dialog,
  DialogDescription,
  DialogTitle
} from '@/components/animate-ui/components/base/dialog'
import { WorkspaceDialogShell } from '@/components/workspace-dialog'
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
import { useEffect, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
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

function TaskList({
  tasks,
  empty,
  busy,
  run
}: {
  tasks: DownloadTask[]
  empty: string
  busy: boolean
  run: (key: number | string, action: () => Promise<unknown>) => void
}) {
  if (!tasks.length) {
    return <p className="py-12 text-center text-sm text-muted-foreground">{empty}</p>
  }
  return (
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
                <Progress value={percent(task)} aria-label={`${task.track.title} 下载进度`} />
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
                  disabled={busy}
                  aria-label={`在文件夹中显示 ${task.track.title}`}
                  title="在文件夹中显示"
                  onClick={() => void run(task.id, () => downloadsReveal(task.id))}
                >
                  <FolderOpen aria-hidden="true" />
                </Button>
              )}
              {(task.state === 'running' || task.state === 'waiting') && (
                <Button
                  variant="ghost"
                  size="icon-sm"
                  disabled={busy}
                  aria-label={`取消下载 ${task.track.title}`}
                  title="取消下载"
                  onClick={() => void run(task.id, () => downloadsCancel(task.id))}
                >
                  <X aria-hidden="true" />
                </Button>
              )}
              {(task.state === 'failed' || task.state === 'cancelled') && (
                <Button
                  variant="ghost"
                  size="icon-sm"
                  disabled={busy}
                  aria-label={`重试下载 ${task.track.title}`}
                  title="重试下载"
                  onClick={() => void run(task.id, () => downloadsRetry(task.id))}
                >
                  <RotateCw aria-hidden="true" />
                </Button>
              )}
              {['completed', 'failed', 'cancelled'].includes(task.state) && (
                <Button
                  variant="ghost"
                  size="icon-sm"
                  disabled={busy}
                  aria-label={`移除下载记录 ${task.track.title}`}
                  title="移除记录"
                  onClick={() => void run(task.id, () => downloadsRemove(task.id))}
                >
                  <Trash2 aria-hidden="true" />
                </Button>
              )}
            </div>
          </div>
          {task.error && <p className="mt-1.5 text-xs text-destructive">{task.error}</p>}
          {task.warning && (
            <p className="mt-1.5 text-xs text-amber-600 dark:text-amber-400">{task.warning}</p>
          )}
        </article>
      ))}
    </div>
  )
}

const downloadsReveal = (id: number) => nativeCall('downloads_reveal', { id })
const downloadsCancel = (id: number) => nativeCall('downloads_cancel', { id })
const downloadsRetry = (id: number) => nativeCall('downloads_retry', { id })
const downloadsRemove = (id: number) => nativeCall('downloads_remove', { id })

export function DownloadDialog() {
  const downloads = useDownloads()
  const [busy, setBusy] = useState<number | string>()
  const [error, setError] = useState<string>()
  const [directory, setDirectory] = useState<string>()
  const tasks = sorted(downloads.tasks)
  const activeTasks = tasks.filter((task) => task.state !== 'completed')
  const completedTasks = tasks.filter((task) => task.state === 'completed')
  const finished = tasks.some((task) => ['completed', 'failed', 'cancelled'].includes(task.state))

  useEffect(() => {
    if (!downloads.open || !isTauri()) return
    let disposed = false
    void nativeCall<{ directory: string }>('download_settings')
      .then((settings) => {
        if (!disposed) setDirectory(settings.directory)
      })
      .catch((cause) => {
        reportCaughtError('app/src/features/downloads/download-dialog.tsx', cause)
        if (!disposed) setError(errorText(cause))
      })
    return () => {
      disposed = true
    }
  }, [downloads.open])

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
    <Dialog open={downloads.open} onOpenChange={(open) => downloads.setOpen(open)}>
      <WorkspaceDialogShell title="下载管理">
        <div className="flex h-full min-h-0 flex-col p-5 pt-14 sm:p-10">
          <header className="shrink-0 pr-10">
            <DialogTitle className="text-xl font-bold">下载管理</DialogTitle>
            <DialogDescription className="mt-2">
              下载完成后自动写入标签，并加入本地音乐库；关闭弹窗后下载继续，可从标题栏重新打开。
            </DialogDescription>
          </header>
          {error && <p className="mt-3 shrink-0 text-sm text-destructive">{error}</p>}
          <Tabs defaultValue="active" className="mt-5 min-h-0 flex-1 gap-3">
            <div className="flex min-w-0 items-center gap-4">
              <TabsList aria-label="下载分类">
                <TabsTrigger value="active">
                  下载中
                  <span className="text-xs tabular-nums text-muted-foreground">
                    {activeTasks.length}
                  </span>
                </TabsTrigger>
                <TabsTrigger value="completed">
                  已下载
                  <span className="text-xs tabular-nums text-muted-foreground">
                    {completedTasks.length}
                  </span>
                </TabsTrigger>
              </TabsList>
              <div className="ml-auto flex min-w-0 items-center justify-end gap-1.5">
                <span
                  className="min-w-0 max-w-96 truncate text-xs text-muted-foreground"
                  title={directory}
                >
                  当前下载位置：{directory ?? '正在读取…'}
                </span>
                <Button
                  variant="ghost"
                  size="icon-sm"
                  disabled={busy !== undefined || !isTauri()}
                  aria-label="打开下载目录"
                  title="打开下载目录"
                  onClick={() => void run('directory', () => nativeCall('open_download_directory'))}
                >
                  <FolderOpen aria-hidden="true" />
                </Button>
              </div>
            </div>
            <TabsContent value="active" className="min-h-0 overflow-auto pr-1">
              <TaskList
                tasks={activeTasks}
                empty="暂无下载任务，右键在线歌曲选择“下载”即可开始。"
                busy={busy !== undefined}
                run={run}
              />
            </TabsContent>
            <TabsContent value="completed" className="min-h-0 overflow-auto pr-1">
              <TaskList
                tasks={completedTasks}
                empty="还没有已下载的歌曲。"
                busy={busy !== undefined}
                run={run}
              />
            </TabsContent>
          </Tabs>
          <div className="mt-4 flex shrink-0 justify-end gap-2">
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
          </div>
        </div>
      </WorkspaceDialogShell>
    </Dialog>
  )
}
