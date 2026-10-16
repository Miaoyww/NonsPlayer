import { ErrorNotice } from '@/components/error-notice'
import { TextSkeleton } from '@/components/music/loading'
import { useCallback, useEffect, useRef, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import { Folder, FolderCog, Plus, RefreshCw, Trash2 } from 'lucide-react'
import { nativeCall, errorText } from '@/lib/player'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger
} from '@/components/ui/dialog'

interface MusicFolder {
  path: string
  tracks: number
  scanning: boolean
  error?: string
}
export function FolderManager() {
  const [opened, setOpened] = useState(false)
  const [folders, setFolders] = useState<MusicFolder[]>([])
  const [loading, setLoading] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string>()
  const serial = useRef(0)
  const reload = useCallback(async () => {
    if (!isTauri()) return
    const generation = ++serial.current
    try {
      const value = await nativeCall<MusicFolder[]>('music_folders')
      if (generation === serial.current) setFolders(value)
    } catch (cause) {
      if (generation === serial.current) setError(errorText(cause))
    }
  }, [])
  useEffect(() => {
    if (!opened) return
    let disposed = false
    let stop: (() => void) | undefined
    setLoading(true)
    void reload().finally(() => {
      if (!disposed) setLoading(false)
    })
    if (isTauri())
      void listen('local-library-updated', () => {
        void reload()
      })
        .then((fn) => {
          if (disposed) fn()
          else stop = fn
        })
        .catch((cause) => setError(errorText(cause)))
    return () => {
      disposed = true
      serial.current++
      stop?.()
    }
  }, [opened, reload])
  async function mutate(action: () => Promise<unknown>) {
    if (busy) return
    setBusy(true)
    setError(undefined)
    try {
      await action()
      await reload()
    } catch (cause) {
      setError(errorText(cause))
    } finally {
      setBusy(false)
    }
  }
  return (
    <Dialog open={opened} onOpenChange={setOpened}>
      <DialogTrigger render={<Button variant="outline" />}>
        <FolderCog aria-hidden="true" />
        管理文件夹
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>管理音乐文件夹</DialogTitle>
          <DialogDescription>
            添加或移除本地音乐文件夹。已添加的文件夹会自动扫描，并同步文件变化。
          </DialogDescription>
        </DialogHeader>
        <div className="my-5 max-h-[40dvh] overflow-auto">
          {loading ? (
            <TextSkeleton />
          ) : folders.length ? (
            <ul className="flex flex-col gap-3">
              {folders.map((folder) => (
                <li
                  key={folder.path}
                  className="flex items-start gap-3 rounded-xl border border-border bg-muted/30 p-4"
                >
                  <Folder
                    className="mt-1 size-5 shrink-0 text-muted-foreground"
                    aria-hidden="true"
                  />
                  <div className="min-w-0 flex-1">
                    <p className="break-all text-sm font-medium">{folder.path}</p>
                    <p className="mt-1 text-xs text-muted-foreground">
                      {folder.scanning ? '正在扫描…' : `${folder.tracks} 首音乐`}
                    </p>
                    {folder.error && <ErrorNotice error={folder.error} />}
                  </div>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    disabled={busy}
                    aria-label={`移除文件夹 ${folder.path}`}
                    title="移除目录索引，保留原始文件"
                    onClick={() =>
                      void mutate(() => nativeCall('remove_music_folder', { path: folder.path }))
                    }
                  >
                    <Trash2 className="text-destructive" aria-hidden="true" />
                  </Button>
                </li>
              ))}
            </ul>
          ) : (
            <div className="flex min-h-44 flex-col items-center justify-center gap-4 rounded-xl border-2 border-dashed border-border text-muted-foreground">
              <Folder className="size-12 opacity-50" aria-hidden="true" />
              <p className="text-sm">暂未添加文件夹</p>
            </div>
          )}
        </div>
        {error && <ErrorNotice error={error} />}
        <div className="flex gap-2">
          <Button
            variant="outline"
            className="flex-1 rounded-full"
            disabled={busy || !isTauri()}
            onClick={() =>
              void mutate(async () => {
                const path = await open({
                  directory: true,
                  multiple: false,
                  title: '添加音乐文件夹'
                })
                if (typeof path === 'string') await nativeCall('add_music_folder', { path })
              })
            }
          >
            <Plus aria-hidden="true" />
            {busy ? '正在处理…' : '添加文件夹'}
          </Button>
          {folders.length > 0 && (
            <Button
              variant="ghost"
              disabled={busy || folders.some((f) => f.scanning)}
              onClick={() => void mutate(() => nativeCall('rescan_music_folders'))}
            >
              <RefreshCw aria-hidden="true" />
              重新扫描
            </Button>
          )}
        </div>
        <p className="mt-3 text-xs text-muted-foreground">
          移除文件夹只移除曲库索引，保留原始音乐文件。
        </p>
      </DialogContent>
    </Dialog>
  )
}
