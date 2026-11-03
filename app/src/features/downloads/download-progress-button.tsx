import { Download } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { isWindows } from '@/lib/platform'
import { useDownloads } from './download-provider'

export function DownloadProgressButton() {
  const downloads = useDownloads()
  if (!isWindows() || !downloads.tasks.length) return null
  const active = downloads.tasks.filter((task) =>
    ['waiting', 'running'].includes(task.state)
  ).length
  return (
    <Button
      variant="ghost"
      size="icon-sm"
      aria-label={active ? `下载进度，${active} 个任务进行中` : '下载进度'}
      title={active ? `${active} 个下载任务进行中` : '下载进度'}
      onClick={() => downloads.setOpen(true)}
    >
      <Download aria-hidden="true" />
    </Button>
  )
}
