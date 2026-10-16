import { useMemo, useState, useSyncExternalStore } from 'react'
import {
  reportCaughtError,
  Button,
  DownloadedSongList,
  Icon,
  Input,
  Progress,
  SongIdentity,
  Tabs,
  TabsList,
  TabsTab,
  TabsPanel,
  TabsPanels
} from '@app/plugin-sdk'
import { qualityLabels } from './protocol'
import {
  configure,
  deleteTask,
  openDirectory,
  pauseTask,
  resumeTask,
  retryTask,
  snapshot,
  subscribe
} from './runtime'
export { activate, downloadSong } from './runtime'

const labels = {
  waiting: '等待下载',
  setup: '需要设置',
  resolving: '解析资源',
  running: '下载中',
  paused: '已暂停',
  saving: '保存中',
  completed: '已完成',
  failed: '失败',
  cancelled: '已取消'
}
export function DownloadsPage() {
  const view = useSyncExternalStore(subscribe, snapshot)
  const [tab, setTab] = useState('active')
  const [query, setQuery] = useState('')
  const [message, setMessage] = useState('')
  const [busy, setBusy] = useState<number | string>()
  const filtered = useMemo(() => {
    const search = query.trim().toLocaleLowerCase()
    return view.tasks.filter((task) =>
      `${task.song.title} ${task.song.artist} ${task.song.album}`
        .toLocaleLowerCase()
        .includes(search)
    )
  }, [view.tasks, query])
  const completed = useMemo(
    () =>
      filtered
        .filter((task) => task.state === 'completed' && task.path && task.options?.directory)
        .sort((a, b) => (b.completedAt || 0) - (a.completedAt || 0))
        .map((task) => ({
          song: task.song,
          root: task.options!.directory,
          path: task.path!,
          completedAt: task.completedAt,
          size: task.bytes
        })),
    [filtered]
  )
  const active = filtered.filter((task) => task.state !== 'completed')
  async function run(id: number | string, work: () => Promise<unknown>) {
    if (busy !== undefined) return
    setBusy(id)
    setMessage('')
    try {
      await work()
    } catch (error) {
      reportCaughtError('netease-download.action', error)
      setMessage(error instanceof Error ? error.message : String(error))
    } finally {
      setBusy(undefined)
    }
  }
  return (
    <div className="space-y-6">
      <h1 className="library-heading">下载管理</h1>
      <Tabs value={tab} onValueChange={setTab} className="gap-6">
        <div className="flex flex-wrap items-center justify-between gap-4">
          <TabsList className="music-tabs" aria-label="下载分类">
            <TabsTab value="completed">已下载单曲</TabsTab>
            <TabsTab value="active">正在下载</TabsTab>
          </TabsList>
          <div className="flex flex-wrap items-center gap-2">
            <div className="relative w-56">
              <span className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground [&_svg]:size-4">
                <Icon name="search" />
              </span>
              <Input
                type="search"
                className="pl-9"
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                placeholder="搜索歌名、歌手或专辑"
                aria-label="搜索下载音乐"
              />
            </div>
            <Button
              variant="outline"
              disabled={busy !== undefined}
              onClick={() => void run('directory', openDirectory)}
            >
              <Icon name="folder" />
              打开目录
            </Button>
            <Button variant="ghost" size="icon" aria-label="下载设置" onClick={configure}>
              <Icon name="settings" />
            </Button>
          </div>
        </div>
        {(message || view.message) && (
          <p role="status" className="text-sm text-muted-foreground">
            {message || view.message}
          </p>
        )}
        <TabsPanels>
          <TabsPanel value="completed">
            {completed.length ? (
              <DownloadedSongList songs={completed} />
            ) : (
              <p className="py-12 text-center text-sm text-muted-foreground">
                {query ? '没有匹配的已下载单曲' : '暂无已下载单曲'}
              </p>
            )}
          </TabsPanel>
          <TabsPanel value="active">
            {!active.length && (
              <p className="py-12 text-center text-sm text-muted-foreground">
                {query ? '没有匹配的下载任务' : '暂无下载任务，右键网易云歌曲选择“下载”即可添加。'}
              </p>
            )}
            <div className="space-y-1">
              {active.map((task, index) => (
                <article
                  key={task.id}
                  className="grid grid-cols-[2rem_minmax(0,1fr)] items-center gap-4 rounded-xl px-3 py-4 hover:bg-muted/40 lg:grid-cols-[2rem_minmax(0,1fr)_15rem_5rem_5rem]"
                  aria-label={`${task.song.title} 下载任务`}
                >
                  <span className="text-center text-sm tabular-nums text-muted-foreground">
                    {String(index + 1).padStart(2, '0')}
                  </span>
                  <SongIdentity song={task.song} />
                  <div className="col-start-2 space-y-2 lg:col-start-auto">
                    <Progress
                      className="w-full"
                      aria-label={`${task.song.title} 下载进度`}
                      value={task.total ? task.bytes : null}
                      max={task.total || undefined}
                    />
                    <p className="text-xs tabular-nums text-muted-foreground">
                      {(task.bytes / 1048576).toFixed(1)} /{' '}
                      {task.total ? (task.total / 1048576).toFixed(1) : '—'} MiB
                      {task.quality ? ` · ${qualityLabels[task.quality]}` : ''}
                    </p>
                  </div>
                  <p className="col-start-2 text-sm text-muted-foreground lg:col-start-auto">
                    {labels[task.state]}
                  </p>
                  <div className="col-start-2 flex items-center gap-1 lg:col-start-auto">
                    <Button
                      variant="ghost"
                      size="icon-sm"
                      disabled={busy !== undefined || task.state === 'saving'}
                      aria-label={`${task.state === 'paused' ? '继续下载' : ['failed', 'cancelled', 'setup'].includes(task.state) ? '重试下载' : '暂停下载'} ${task.song.title}`}
                      onClick={() =>
                        void run(task.id, () =>
                          task.state === 'paused'
                            ? resumeTask(task.id)
                            : ['failed', 'cancelled', 'setup'].includes(task.state)
                              ? retryTask(task.id)
                              : pauseTask(task.id)
                        )
                      }
                    >
                      <Icon
                        name={
                          ['paused', 'failed', 'cancelled', 'setup'].includes(task.state)
                            ? 'play'
                            : 'pause'
                        }
                      />
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon-sm"
                      disabled={busy !== undefined || task.state === 'saving'}
                      aria-label={`删除下载任务 ${task.song.title}`}
                      onClick={() => void run(task.id, () => deleteTask(task.id))}
                    >
                      <Icon name="delete" />
                    </Button>
                  </div>
                  {task.error && (
                    <p className="col-span-full pl-12 text-sm text-destructive" role="alert">
                      {task.error}
                    </p>
                  )}
                </article>
              ))}
            </div>
          </TabsPanel>
        </TabsPanels>
      </Tabs>
    </div>
  )
}
