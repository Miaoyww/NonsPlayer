import { reportCaughtError } from '@/lib/diagnostics'
import { ErrorNotice } from '@/components/error-notice'
import { TextSkeleton } from '@/components/music/loading'
import { useEffect, useMemo, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { Button } from '@/components/ui/button'
import {
  Combobox,
  ComboboxContent,
  ComboboxEmpty,
  ComboboxInput,
  ComboboxItem,
  ComboboxList
} from '@/components/ui/combobox'
import { fontStack } from '@/features/settings/font-settings'
import { errorText, nativeCall } from '@/lib/player'

type FontOption = { value: string; label: string }
const previewFonts = [
  'Arial',
  'Georgia',
  'Microsoft YaHei',
  'Segoe UI',
  'Times New Roman',
  'Verdana'
]

export function FontPicker({
  value,
  onChange,
  label,
  defaultLabel
}: {
  value: string
  onChange: (family: string) => void
  label: string
  defaultLabel: string
}) {
  const [families, setFamilies] = useState<string[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string>()
  const [refresh, setRefresh] = useState(0)
  useEffect(() => {
    let disposed = false
    setLoading(true)
    setError(undefined)
    const load = isTauri()
      ? nativeCall<string[]>('system_fonts', refresh ? { refresh: true } : undefined)
      : Promise.resolve(previewFonts)
    void load
      .then((fonts) => {
        if (!disposed) setFamilies(fonts)
      })
      .catch((cause) => {
        reportCaughtError('app/src/features/settings/font-picker.tsx', cause)

        if (!disposed) setError(errorText(cause))
      })
      .finally(() => {
        if (!disposed) setLoading(false)
      })
    return () => {
      disposed = true
    }
  }, [refresh])
  const items = useMemo<FontOption[]>(
    () => [
      { value: '', label: defaultLabel },
      ...(value && !families.includes(value)
        ? [{ value, label: loading || error ? value : `${value}（未安装）` }]
        : []),
      ...families.map((family) => ({ value: family, label: family }))
    ],
    [families, value, defaultLabel, loading, error]
  )
  return (
    <div className="flex w-64 max-w-full flex-col gap-2">
      <Combobox
        items={items}
        value={items.find((item) => item.value === value) ?? items[0]}
        isItemEqualToValue={(a, b) => a.value === b.value}
        onValueChange={(item) => {
          if (item) onChange(item.value)
        }}
      >
        <ComboboxInput aria-label={label} placeholder="搜索字体…" className="w-full" />
        <ComboboxContent>
          <ComboboxEmpty>{loading ? '正在读取字体…' : '没有匹配的字体'}</ComboboxEmpty>
          <ComboboxList>
            {(item: FontOption) => (
              <ComboboxItem key={item.value} value={item}>
                <span
                  className="truncate"
                  style={item.value ? { fontFamily: fontStack(item.value) } : undefined}
                >
                  {item.label}
                </span>
              </ComboboxItem>
            )}
          </ComboboxList>
        </ComboboxContent>
      </Combobox>
      {loading && <TextSkeleton />}
      {error && (
        <div className="text-xs">
          <ErrorNotice error={error} />
          <Button size="sm" variant="ghost" onClick={() => setRefresh((n) => n + 1)}>
            重新读取
          </Button>
        </div>
      )}
      {!isTauri() && (
        <p className="text-xs text-muted-foreground">
          浏览器预览使用常见字体；桌面应用显示已安装字体。
        </p>
      )}
    </div>
  )
}
