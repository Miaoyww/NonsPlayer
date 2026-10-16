import { useEffect, useRef, useState } from 'react'
import {
  Button,
  MusicPageHeader,
  Progress,
  usePluginFiles,
  reportCaughtError
} from '@app/plugin-sdk'

interface Item {
  root: string
  path: string
  name: string
  state: 'waiting' | 'converting' | 'done' | 'error'
  output?: string
  error?: string
}
export function ConverterPage() {
  const files = usePluginFiles()
  const [items, setItems] = useState<Item[]>([])
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string>()
  const running = useRef(false)
  const cancelled = useRef(false)
  const mounted = useRef(true)
  useEffect(() => {
    mounted.current = true
    return () => {
      mounted.current = false
      cancelled.current = true
    }
  }, [])
  async function select() {
    if (running.current) return
    running.current = true
    setBusy(true)
    setError(undefined)
    try {
      const selected = await files.pickAudio(['ncm'])
      if (!mounted.current) return
      const ncm = selected.filter((file) => /\.ncm$/i.test(file.path))
      if (ncm.length !== selected.length) setError('已跳过非 NCM 文件。')
      setItems((previous) => {
        const existing = new Set(previous.map((item) => `${item.root}:${item.path}`))
        const added = ncm.filter((item) => !existing.has(`${item.root}:${item.path}`))
        return [...previous, ...added.map((item): Item => ({ ...item, state: 'waiting' }))].slice(
          0,
          128
        )
      })
    } catch (cause) {
      reportCaughtError('ncm-converter.select', cause)
      if (mounted.current) setError(String(cause))
    } finally {
      running.current = false
      if (mounted.current) setBusy(false)
    }
  }
  async function convert() {
    if (running.current) return
    running.current = true
    cancelled.current = false
    setBusy(true)
    setError(undefined)
    const update = (item: Item, patch: Partial<Item>) => {
      if (mounted.current)
        setItems((previous) =>
          previous.map((entry) =>
            entry.root === item.root && entry.path === item.path ? { ...entry, ...patch } : entry
          )
        )
    }
    try {
      for (const item of items.filter((entry) => entry.state !== 'done')) {
        if (cancelled.current || !mounted.current) break
        update(item, { state: 'converting', error: undefined })
        try {
          const result = await files.decodeAudio(item.root, item.path)
          update(item, { state: 'done', output: result.path })
        } catch (cause) {
          reportCaughtError('ncm-converter.convert', cause)
          update(item, { state: 'error', error: String(cause) })
        }
      }
    } finally {
      running.current = false
      if (mounted.current) setBusy(false)
    }
  }
  const done = items.filter((item) => item.state === 'done').length
  const failed = items.filter((item) => item.state === 'error').length
  const active = items.some((item) => item.state === 'converting')
  return (
    <div className="flex flex-col gap-6" aria-busy={busy}>
      <MusicPageHeader title="NCM 格式转换" />
      <div className="rounded-2xl border border-border bg-muted/30 p-6">
        <p className="text-base font-medium">让音乐回到通用格式</p>
        <p className="mt-2 text-sm leading-6 text-muted-foreground">
          选择一个或多个 NCM 文件，自动恢复为其原始 MP3 或 FLAC
          音频。保存到原文件相同目录，保留歌曲信息和内嵌封面；原文件保留，同名结果不会覆盖。
        </p>
        <div className="mt-5 flex flex-wrap gap-3">
          <Button onClick={() => void select()} disabled={busy}>
            选择 NCM 文件
          </Button>
          <Button
            variant="secondary"
            onClick={() => void convert()}
            disabled={busy || !items.some((item) => item.state !== 'done')}
          >
            开始转换
          </Button>
          {active && (
            <Button
              variant="outline"
              onClick={() => {
                cancelled.current = true
              }}
            >
              完成当前文件后停止
            </Button>
          )}
          {!!items.length && (
            <Button
              variant="ghost"
              disabled={busy}
              onClick={() => {
                setItems([])
                setError(undefined)
              }}
            >
              清空列表
            </Button>
          )}
        </div>
      </div>
      {error && (
        <p role="alert" className="text-sm text-destructive">
          {error}
        </p>
      )}
      {!!items.length && (
        <>
          <div className="space-y-3">
            <p role="status" aria-live="polite" className="text-sm text-muted-foreground">
              共 {items.length} 个文件 · 已完成 {done} 个{failed ? ` · 失败 ${failed} 个` : ''}
              {active ? ' · 正在转换' : ''}
            </p>
            <Progress value={done + failed} max={items.length} aria-label="批量转换进度" />
          </div>
          <ul className="divide-y divide-border rounded-2xl border border-border">
            {items.map((item) => (
              <li
                key={`${item.root}:${item.path}`}
                className="flex flex-wrap items-start justify-between gap-3 p-4"
              >
                <div className="min-w-0 flex-1">
                  <p className="break-all font-medium">{item.name}</p>
                  <p className="mt-1 break-all text-xs leading-5 text-muted-foreground">
                    {item.path}
                  </p>
                  {item.output && (
                    <p className="mt-2 break-all text-sm text-muted-foreground">
                      已保存：{item.output}
                    </p>
                  )}
                  {item.error && (
                    <p className="mt-2 break-all text-sm text-destructive">{item.error}</p>
                  )}
                </div>
                <span className="text-sm text-muted-foreground">
                  {
                    {
                      waiting: '等待转换',
                      converting: '正在转换…',
                      done: '转换完成',
                      error: '转换失败'
                    }[item.state]
                  }
                </span>
              </li>
            ))}
          </ul>
        </>
      )}
      <p className="text-sm leading-6 text-muted-foreground">
        本地音乐也可直接导入 NCM 并播放，无需先转换。此工具恢复原始编码，不进行 MP3 与 FLAC
        之间的转码。单文件上限 256MiB，每批最多 128 个。
      </p>
    </div>
  )
}
