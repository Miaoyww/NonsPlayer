import { Tabs, TabsList, TabsTab, TabsPanel } from '@/components/animate-ui/components/base/tabs'
import type { ReactNode } from 'react'
import { RefreshCw, Search, Upload, FolderOpen } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue
} from '@/components/ui/select'

export function ExtensionListHeader({
  title,
  description,
  icon,
  busy,
  onRefresh,
  children
}: {
  title: string
  description: string
  icon: ReactNode
  busy: boolean
  onRefresh: () => void
  children?: ReactNode
}) {
  return (
    <div className="flex flex-wrap items-start justify-between gap-4">
      <div>
        <h2 className="flex items-center gap-2 text-xl font-bold">
          {icon}
          {title}
        </h2>
        <p className="mt-2 text-sm text-muted-foreground">{description}</p>
      </div>
      <div className="flex flex-wrap items-center gap-2">
        {children}
        <Button
          variant="outline"
          size="sm"
          disabled={busy}
          aria-label={`刷新${title}`}
          onClick={onRefresh}
        >
          <RefreshCw aria-hidden="true" />
          刷新
        </Button>
      </div>
    </div>
  )
}

export function ExtensionTabs({
  noun,
  value,
  onValueChange,
  installation,
  children
}: {
  noun: string
  value: string
  onValueChange: (value: string) => void
  installation: ReactNode
  children: ReactNode
}) {
  return (
    <Tabs value={value} onValueChange={onValueChange} className="gap-4">
      <TabsList aria-label={`${noun}页面`}>
        <TabsTab value="installed">已安装</TabsTab>
        <TabsTab value="install">
          <Upload aria-hidden="true" />
          安装{noun}
        </TabsTab>
      </TabsList>
      <TabsPanel value="installed" className="space-y-4">
        {children}
      </TabsPanel>
      <TabsPanel value="install">{installation}</TabsPanel>
    </Tabs>
  )
}

export function ExtensionInstallPage({
  noun,
  available = false,
  busy = false,
  onZip,
  onDirectory,
  children
}: {
  noun: string
  available?: boolean
  busy?: boolean
  onZip?: () => void
  onDirectory?: () => void
  children?: ReactNode
}) {
  return (
    <div className="space-y-4">
      <section
        aria-label={`本地安装${noun}`}
        className="rounded-xl border border-dashed border-border p-4"
      >
        <h3 className="text-sm font-semibold">从本地安装</h3>
        <p className="mt-1 text-xs leading-normal text-muted-foreground">
          {available
            ? '选择 ZIP 文件或插件目录。安装前会显示权限范围并要求确认。'
            : '适配器本地安装暂未开放，当前仅支持随包提供的适配器。'}
        </p>
        <div className="mt-3 flex flex-wrap gap-2">
          <Button size="sm" disabled={!available || busy} onClick={onZip}>
            <Upload aria-hidden="true" />
            选择 ZIP
          </Button>
          <Button size="sm" variant="outline" disabled={!available || busy} onClick={onDirectory}>
            <FolderOpen aria-hidden="true" />
            选择目录
          </Button>
        </div>
      </section>
      {children}
    </div>
  )
}

export function ExtensionListToolbar({
  noun,
  query,
  onQueryChange,
  filter,
  onFilterChange,
  options,
  status,
  onStatusChange,
  statuses,
  count,
  total
}: {
  noun: string
  query: string
  onQueryChange: (value: string) => void
  filter?: string
  onFilterChange?: (value: string) => void
  options?: { value: string; label: string }[]
  status: string
  onStatusChange: (value: string) => void
  statuses: readonly (readonly [string, string])[]
  count: number
  total: number
}) {
  const label = options?.find((option) => option.value === filter)?.label ?? filter
  return (
    <div className="space-y-3">
      <div className="flex flex-wrap items-center gap-3">
        <div className="relative min-w-0 flex-1 basis-48">
          <Search
            aria-hidden="true"
            className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground"
          />
          <Input
            type="search"
            aria-label={`搜索${noun}`}
            placeholder="搜索名称、ID 或描述"
            value={query}
            onChange={(event) => onQueryChange(event.target.value)}
            className="pl-9"
          />
        </div>
        {options && onFilterChange && (
          <Select value={filter} onValueChange={(value) => onFilterChange(value ?? 'all')}>
            <SelectTrigger aria-label={`筛选${noun}能力`} className="w-40">
              <SelectValue>{label}</SelectValue>
            </SelectTrigger>
            <SelectContent>
              {options.map((option) => (
                <SelectItem key={option.value} value={option.value}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        )}
      </div>
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex flex-wrap gap-1" role="group" aria-label={`按状态筛选${noun}`}>
          {statuses.map(([value, text]) => (
            <Button
              key={value}
              variant={status === value ? 'secondary' : 'ghost'}
              size="sm"
              aria-pressed={status === value}
              onClick={() => onStatusChange(value)}
            >
              {text}
            </Button>
          ))}
        </div>
        <p role="status" className="text-xs tabular-nums text-muted-foreground">
          显示 {count} / {total} 个{noun}
        </p>
      </div>
    </div>
  )
}
