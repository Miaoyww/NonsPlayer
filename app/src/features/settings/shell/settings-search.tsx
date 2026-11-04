import { useMemo, useState, useSyncExternalStore } from 'react'
import { Search } from 'lucide-react'
import {
  Combobox,
  ComboboxContent,
  ComboboxEmpty,
  ComboboxInput,
  ComboboxItem,
  ComboboxList
} from '@/components/ui/combobox'
import { InputGroupAddon } from '@/components/ui/input-group'
import { getShortcutStatus, subscribeShortcuts } from '@/features/playback/shortcuts/shortcuts'
import { getSettingsSearchItems, type SettingsSearchItem } from './settings-search-index'

const normalize = (value: string) => value.normalize('NFKC').toLocaleLowerCase().trim()

export function SettingsSearch({ onSelect }: { onSelect: (item: SettingsSearchItem) => void }) {
  const [query, setQuery] = useState('')
  const [open, setOpen] = useState(false)
  const { settings } = useSyncExternalStore(subscribeShortcuts, getShortcutStatus)
  const items = useMemo(getSettingsSearchItems, [settings.enabled])
  return (
    <Combobox
      items={items}
      value={null}
      inputValue={query}
      open={open && !!query.trim()}
      autoHighlight
      itemToStringLabel={(item: SettingsSearchItem) => item.title}
      isItemEqualToValue={(a, b) => a.id === b.id}
      filter={(item, value) => {
        const text = normalize(`${item.title} ${item.path} ${item.keywords ?? ''}`)
        return normalize(value)
          .split(/\s+/)
          .every((word) => text.includes(word))
      }}
      onInputValueChange={(value, details) => {
        if (details.reason === 'item-press') return
        setQuery(value)
      }}
      onOpenChange={setOpen}
      onValueChange={(item) => {
        if (!item) return
        setQuery('')
        setOpen(false)
        onSelect(item)
      }}
    >
      <ComboboxInput
        aria-label="搜索设置"
        placeholder="搜索设置"
        showTrigger={false}
        autoComplete="off"
        className="h-8 w-full rounded-lg bg-background/70 [&_input]:text-xs"
      >
        <InputGroupAddon align="inline-start">
          <Search aria-hidden="true" className="size-3.5" />
        </InputGroupAddon>
      </ComboboxInput>
      <ComboboxContent className="min-w-64 rounded-lg shadow-lg" sideOffset={5}>
        <ComboboxEmpty>没有匹配的设置</ComboboxEmpty>
        <ComboboxList>
          {(item: SettingsSearchItem) => (
            <ComboboxItem key={item.id} value={item} className="rounded-md py-2 pl-3 pr-3">
              <span className="flex min-w-0 flex-col gap-1">
                <span className="text-xs text-muted-foreground">{item.path}</span>
                <span className="text-sm">{item.title}</span>
              </span>
            </ComboboxItem>
          )}
        </ComboboxList>
      </ComboboxContent>
    </Combobox>
  )
}
