import { reportCaughtError } from '@/lib/diagnostics'
import { useEffect, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { FolderOpen } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { ErrorNotice } from '@/components/error-notice'
import { SettingsCard } from '@/features/settings/settings-card'
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue
} from '@/components/ui/select'
import { errorText, nativeCall } from '@/lib/player'

const qualities = [
  { value: 'standard', label: '标准' },
  { value: 'higher', label: '较高' },
  { value: 'exhigh', label: '极高' },
  { value: 'lossless', label: '无损' },
  { value: 'hires', label: 'Hi-Res' }
]

const filenameFormats = [
  { value: 'title-artist', label: '歌曲名 - 歌手' },
  { value: 'title', label: '歌曲名' },
  { value: 'artist-title', label: '歌手 - 歌曲名' }
]

interface DownloadSettings {
  directory: string
  quality: string
  filenameFormat: string
}

export function DownloadPage() {
  const [settings, setSettings] = useState<DownloadSettings>()
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string>()
  useEffect(() => {
    if (!isTauri()) return
    let disposed = false
    void nativeCall<DownloadSettings>('download_settings')
      .then((value) => {
        if (!disposed) setSettings(value)
      })
      .catch((cause) => {
        reportCaughtError('app/src/features/settings/pages/download.tsx', cause)
        if (!disposed) setError(errorText(cause))
      })
    return () => {
      disposed = true
    }
  }, [])
  async function updateQuality(quality: string) {
    if (busy) return
    setBusy(true)
    setError(undefined)
    try {
      setSettings(await nativeCall<DownloadSettings>('set_download_quality', { quality }))
    } catch (cause) {
      reportCaughtError('app/src/features/settings/pages/download.tsx', cause)
      setError(errorText(cause))
    } finally {
      setBusy(false)
    }
  }
  async function updateFilenameFormat(format: string) {
    if (busy) return
    setBusy(true)
    setError(undefined)
    try {
      setSettings(await nativeCall<DownloadSettings>('set_download_filename_format', { format }))
    } catch (cause) {
      reportCaughtError('app/src/features/settings/pages/download.tsx', cause)
      setError(errorText(cause))
    } finally {
      setBusy(false)
    }
  }
  async function openDirectory() {
    if (busy) return
    setBusy(true)
    setError(undefined)
    try {
      await nativeCall('open_download_directory')
    } catch (cause) {
      reportCaughtError('app/src/features/settings/pages/download.tsx', cause)
      setError(errorText(cause))
    } finally {
      setBusy(false)
    }
  }
  return (
    <div className="flex flex-col gap-8">
      <div>
        <h2 className="text-xl font-bold">下载设置</h2>
        <p className="mt-2 text-sm text-muted-foreground">
          设置下载音质与文件名格式；下载文件保存在应用数据目录的 downloads
          文件夹，并自动加入本地音乐库。
        </p>
      </div>
      <SettingsCard
        searchId="download-directory"
        title="下载目录"
        description="固定为应用数据目录下的 downloads，下载完成后自动写入标签并索引。"
      >
        <div className="flex min-w-0 flex-wrap items-center gap-2">
          <code className="min-w-0 break-all rounded-lg bg-muted px-3 py-2 text-xs">
            {settings?.directory ?? '正在读取…'}
          </code>
          <Button
            variant="outline"
            size="sm"
            disabled={busy || !isTauri()}
            onClick={() => void openDirectory()}
          >
            <FolderOpen aria-hidden="true" />
            打开目录
          </Button>
        </div>
      </SettingsCard>
      <SettingsCard
        searchId="download-filename-format"
        title="文件名格式"
        description="选择下载文件的命名方式；重名时自动追加序号，不覆盖已有文件。"
      >
        <Select
          items={filenameFormats}
          value={settings?.filenameFormat ?? 'title-artist'}
          disabled={busy || !isTauri()}
          onValueChange={(format) => {
            if (format) void updateFilenameFormat(format)
          }}
        >
          <SelectTrigger className="w-56" aria-label="文件名格式">
            <SelectValue />
          </SelectTrigger>
          <SelectContent alignItemWithTrigger={false}>
            <SelectGroup>
              {filenameFormats.map(({ value, label }) => (
                <SelectItem key={value} value={value}>
                  {label}
                </SelectItem>
              ))}
            </SelectGroup>
          </SelectContent>
        </Select>
      </SettingsCard>
      <SettingsCard
        searchId="download-quality"
        title="下载音质"
        description="账号不支持所选音质时，自动使用可用的较低音质。"
      >
        <Select
          items={qualities}
          value={settings?.quality ?? 'lossless'}
          disabled={busy || !isTauri()}
          onValueChange={(quality) => {
            if (quality) void updateQuality(quality)
          }}
        >
          <SelectTrigger className="w-48" aria-label="下载音质">
            <SelectValue />
          </SelectTrigger>
          <SelectContent alignItemWithTrigger={false}>
            <SelectGroup>
              {qualities.map(({ value, label }) => (
                <SelectItem key={value} value={value}>
                  {label}
                </SelectItem>
              ))}
            </SelectGroup>
          </SelectContent>
        </Select>
      </SettingsCard>
      {error && <ErrorNotice error={error} />}
    </div>
  )
}
