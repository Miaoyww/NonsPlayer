import type { ReactNode } from 'react'
import { RefreshCw, Search, Upload } from 'lucide-react'
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

export function ExtensionInstallButton({
  label,
  disabled,
  available = true,
  expanded,
  onClick
}: {
  label: string
  disabled?: boolean
  available?: boolean
  expanded?: boolean
  onClick?: () => void
}) {
  return (
    <Button
      variant="outline"
      size="sm"
      disabled={disabled || !available}
      aria-expanded={available ? expanded : undefined}
      title={available ? label : `${label}暂未开放，当前仅支持随包提供的适配器。`}
      onClick={onClick}
    >
      <Upload aria-hidden="true" />
      {label}
      {!available && <span className="text-xs text-muted-foreground">暂未开放</span>}
    </Button>
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
  filter: string
  onFilterChange: (value: string) => void
  options: { value: string; label: string }[]
  status: string
  onStatusChange: (value: string) => void
  statuses: readonly (readonly [string, string])[]
  count: number
  total: number
}) {
  const label = options.find((option) => option.value === filter)?.label ?? filter
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
