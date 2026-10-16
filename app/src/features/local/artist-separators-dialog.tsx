import { ErrorNotice } from '@/components/error-notice'
import { useRef, useState } from 'react'
import { Plus, Trash2 } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger
} from '@/components/ui/dialog'
import { useLocalPreferences } from '@/features/local/use-local-preferences'
import { errorText } from '@/lib/player'

export function ArtistSeparatorsDialog({
  disabled,
  onSaved
}: {
  disabled: boolean
  onSaved: () => void
}) {
  const { options, setOptions } = useLocalPreferences()
  const [opened, setOpened] = useState(false)
  const [items, setItems] = useState<{ id: number; value: string }[]>([])
  const nextId = useRef(0)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string>()
  const separators = [...new Set(items.map((item) => item.value.trim()).filter(Boolean))]
  const changed = JSON.stringify(separators) !== JSON.stringify(options.artistSeparators)

  async function save() {
    if (saving || !changed) return
    if (separators.some((value) => new TextEncoder().encode(value).length > 32)) {
      setError('每个分隔符最多 32 字节，请缩短后重试。')
      return
    }
    setSaving(true)
    setError(undefined)
    try {
      await setOptions({ ...options, artistSeparators: separators })
      setOpened(false)
      onSaved()
    } catch (cause) {
      setError(errorText(cause))
    } finally {
      setSaving(false)
    }
  }

  return (
    <Dialog
      open={opened}
      onOpenChange={(open, details) => {
        if (saving) {
          details.cancel()
          return
        }
        if (open) {
          setItems(options.artistSeparators.map((value) => ({ id: nextId.current++, value })))
          setError(undefined)
        }
        setOpened(open)
      }}
    >
      <DialogTrigger render={<Button variant="outline" disabled={disabled} />}>
        设置分隔符
      </DialogTrigger>
      <DialogContent showCloseButton={!saving}>
        <DialogHeader>
          <DialogTitle>艺术家分隔符</DialogTitle>
          <DialogDescription>
            每项填写一个分隔符。全部移除后视为一位艺术家。保存后重新整理艺术家，不修改原始标签。
          </DialogDescription>
        </DialogHeader>
        <form
          className="flex min-h-0 flex-col gap-4"
          onSubmit={(event) => {
            event.preventDefault()
            void save()
          }}
        >
          <div className="max-h-[40dvh] space-y-3 overflow-y-auto">
            {items.length ? (
              items.map((item, index) => (
                <div key={item.id} className="flex items-center gap-2">
                  <label htmlFor={`artist-separator-${item.id}`} className="sr-only">
                    分隔符 {index + 1}
                  </label>
                  <Input
                    id={`artist-separator-${item.id}`}
                    value={item.value}
                    disabled={saving}
                    maxLength={32}
                    placeholder="输入分隔符"
                    onChange={(event) => {
                      setItems((current) =>
                        current.map((row) =>
                          row.id === item.id ? { ...row, value: event.target.value } : row
                        )
                      )
                      setError(undefined)
                    }}
                  />
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    disabled={saving}
                    aria-label={`删除分隔符 ${index + 1}`}
                    onClick={() =>
                      setItems((current) => current.filter((row) => row.id !== item.id))
                    }
                  >
                    <Trash2 aria-hidden="true" />
                  </Button>
                </div>
              ))
            ) : (
              <p className="py-4 text-center text-sm text-muted-foreground">
                未设置分隔符，艺术家名称将保持完整。
              </p>
            )}
          </div>
          <Button
            type="button"
            variant="outline"
            disabled={saving || items.length >= 16}
            onClick={() => {
              const item = { id: nextId.current++, value: '' }
              setItems((current) => [...current, item])
            }}
          >
            <Plus aria-hidden="true" />
            添加分隔符
          </Button>
          {error && <ErrorNotice error={error} />}
          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              disabled={saving}
              onClick={() => setOpened(false)}
            >
              取消
            </Button>
            <Button type="submit" disabled={saving || !changed}>
              {saving ? '正在保存…' : '保存'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
