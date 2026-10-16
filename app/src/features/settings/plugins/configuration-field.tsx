import { reportCaughtError } from '@/lib/diagnostics'
import { ErrorNotice } from '@/components/error-notice'
import { useEffect, useId, useRef, useState } from 'react'
import { open } from '@tauri-apps/plugin-dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Switch } from '@/components/ui/switch'
import { Slider } from '@/components/ui/slider'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue
} from '@/components/ui/select'
import { errorText } from '@/lib/player'
import type { ConfigField, DirectoryGrant } from '@/plugins/configuration-types'

const same = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b)
export function ConfigurationField({
  field,
  value,
  diagnostic,
  disabled,
  save,
  reset,
  grants = [],
  authorize
}: {
  field: ConfigField
  value: unknown
  diagnostic?: string
  disabled: boolean
  save: (key: string, value: unknown) => Promise<void>
  reset: (key: string) => Promise<void>
  grants?: DirectoryGrant[]
  authorize?: (key: string) => Promise<void>
}) {
  const id = useId()
  const [draft, setDraft] = useState(value)
  const [error, setError] = useState<string>()
  const [saving, setSaving] = useState(false)
  const dirty = useRef(false)
  const savingRef = useRef(false)
  useEffect(() => {
    if (!dirty.current) setDraft(value)
  }, [value])
  const editor = field.editor!
  function edit(next: unknown) {
    dirty.current = true
    setDraft(next)
  }
  async function commit(next: unknown = draft) {
    if (savingRef.current || (!diagnostic && same(next, value))) {
      if (same(next, value)) dirty.current = false
      return
    }
    savingRef.current = true
    setSaving(true)
    setError(undefined)
    try {
      await save(field.key, next)
      dirty.current = false
      setDraft(next)
    } catch (error) {
      reportCaughtError('app/src/features/settings/plugins/configuration-field.tsx', error)

      dirty.current = true
      setError(errorText(error))
    } finally {
      savingRef.current = false
      setSaving(false)
    }
  }
  const blocked = disabled || saving
  const aria = {
    'aria-labelledby': `${id}-title`,
    'aria-describedby': `${id}-description`,
    'aria-invalid': !!error
  }
  const onEnter = (event: React.KeyboardEvent) => {
    if (event.key === 'Enter' && !event.nativeEvent.isComposing) {
      event.preventDefault()
      void commit()
    }
  }
  let control
  if (editor.kind === 'authorizedDirectory') {
    const available = grants.filter((grant) => grant.writable)
    control = (
      <div className="flex w-full items-center gap-2">
        <Select
          value={String(draft || '')}
          onValueChange={(next) => {
            edit(next)
            void commit(next)
          }}
        >
          <SelectTrigger {...aria} disabled={blocked} className="min-w-0 flex-1">
            <SelectValue>
              {available.find((grant) => grant.id === draft)?.path ||
                (draft ? '目录授权已撤销，请重新选择' : '请选择保存目录')}
            </SelectValue>
          </SelectTrigger>
          <SelectContent>
            {available.map((grant) => (
              <SelectItem key={grant.id} value={grant.id}>
                {grant.path}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Button
          variant="outline"
          disabled={blocked || !authorize}
          onClick={() => {
            void authorize?.(field.key).catch((error) => {
              reportCaughtError(
                'app/src/features/settings/plugins/configuration-field.tsx:114',
                error
              )
              return setError(errorText(error))
            })
          }}
        >
          选择并授权
        </Button>
      </div>
    )
  } else if (editor.kind === 'switch')
    control = (
      <Switch
        {...aria}
        checked={!!draft}
        disabled={blocked}
        onCheckedChange={(next) => {
          edit(next)
          void commit(next)
        }}
      />
    )
  else if (editor.kind === 'slider')
    control = (
      <div className="flex w-full items-center gap-4">
        <Slider
          {...aria}
          disabled={blocked}
          min={editor.min ?? 0}
          max={editor.max ?? 100}
          step={editor.step ?? 1}
          value={Number(draft)}
          onValueChange={(v) => edit(v)}
          onValueCommitted={(v) => void commit(v)}
        />
        <span className="min-w-12 text-right text-sm tabular-nums">
          {String(draft)}
          {editor.unit}
        </span>
      </div>
    )
  else if (editor.kind === 'select' || editor.kind === 'multiselect') {
    const multiple = editor.kind === 'multiselect'
    const indexes = editor.options.flatMap((option, index) =>
      (
        multiple
          ? Array.isArray(draft) && draft.some((v) => same(v, option.value))
          : same(draft, option.value)
      )
        ? [String(index)]
        : []
    )
    control = (
      <Select
        multiple={multiple}
        value={multiple ? indexes : (indexes[0] ?? null)}
        onValueChange={(next) => {
          const selected = multiple
            ? (next as string[]).map((v) => editor.options[Number(v)].value)
            : editor.options[Number(next)].value
          edit(selected)
          void commit(selected)
        }}
      >
        <SelectTrigger {...aria} disabled={blocked} className="w-full">
          <SelectValue>
            {indexes.map((index) => editor.options[Number(index)].label).join('、') || '请选择'}
          </SelectValue>
        </SelectTrigger>
        <SelectContent>
          {editor.options.map((option, index) => (
            <SelectItem key={index} value={String(index)}>
              {option.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    )
  } else if (editor.kind === 'textarea')
    control = (
      <textarea
        {...aria}
        disabled={blocked}
        rows={3}
        value={String(draft ?? '')}
        onChange={(event) => edit(event.target.value)}
        onBlur={() => void commit()}
        className="w-full resize-y rounded-lg border border-input bg-input/30 p-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
      />
    )
  else {
    const number = editor.kind === 'number'
    control = (
      <div className="flex w-full items-center gap-2">
        <Input
          {...aria}
          disabled={blocked}
          type={number ? 'number' : editor.kind === 'color' ? 'color' : 'text'}
          min={editor.min ?? undefined}
          max={editor.max ?? undefined}
          step={editor.step ?? (field.schema.type === 'integer' ? 1 : 'any')}
          value={String(draft ?? '')}
          onChange={(event) => edit(event.target.value)}
          onBlur={() => {
            if (number && String(draft).trim() === '') {
              dirty.current = true
              setError('请输入数字')
              return
            }
            void commit(number ? Number(draft) : draft)
          }}
          onKeyDown={(event) => {
            if (number && event.key === 'Enter' && !event.nativeEvent.isComposing) {
              event.preventDefault()
              if (String(draft).trim() === '') setError('请输入数字')
              else void commit(Number(draft))
            } else onEnter(event)
          }}
          className={editor.kind === 'color' ? 'w-16 p-1' : undefined}
        />
        {editor.unit && editor.kind !== 'color' && (
          <span className="shrink-0 text-sm text-muted-foreground">{editor.unit}</span>
        )}
        {(editor.kind === 'file' || editor.kind === 'directory') && (
          <Button
            variant="outline"
            disabled={blocked}
            onClick={() => {
              void (async () => {
                try {
                  const path = await open({
                    directory: editor.kind === 'directory',
                    multiple: false,
                    title: field.title
                  })
                  if (typeof path === 'string') {
                    edit(path)
                    await commit(path)
                  }
                } catch (error) {
                  reportCaughtError(
                    'app/src/features/settings/plugins/configuration-field.tsx:250',
                    error
                  )

                  setError(errorText(error))
                }
              })()
            }}
          >
            选择
          </Button>
        )}
      </div>
    )
  }
  return (
    <section className="space-y-3 border-b border-border py-5 last:border-b-0">
      <div className="flex items-start justify-between gap-4">
        <div className="min-w-0">
          <h4 id={`${id}-title`} className="text-sm font-medium">
            {field.title}
          </h4>
          <p
            id={`${id}-description`}
            className="mt-1 whitespace-pre-line text-xs leading-5 text-muted-foreground"
          >
            {field.description}
          </p>
          {field.apply === 'reload' && (
            <p className="mt-1 text-xs text-muted-foreground">重载插件后生效</p>
          )}
        </div>
        <Button
          variant="ghost"
          size="sm"
          disabled={blocked}
          onClick={() => {
            setSaving(true)
            void reset(field.key)
              .then(() => {
                dirty.current = false
                setDraft(field.default)
                setError(undefined)
              })
              .catch((error) => {
                reportCaughtError(
                  'app/src/features/settings/plugins/configuration-field.tsx:285',
                  error
                )
                return setError(errorText(error))
              })
              .finally(() => setSaving(false))
          }}
        >
          恢复默认
        </Button>
      </div>
      <div className={editor.kind === 'switch' ? '' : 'max-w-md'}>{control}</div>
      {(error || diagnostic) && <ErrorNotice error={error ?? diagnostic} />}
      {error && (
        <Button
          size="sm"
          variant="outline"
          disabled={blocked}
          onClick={() => {
            if (editor.kind === 'number' && String(draft).trim() === '') {
              setError('请输入数字')
              return
            }
            void commit(editor.kind === 'number' ? Number(draft) : draft)
          }}
        >
          重试保存
        </Button>
      )}
      {saving && (
        <p role="status" className="text-xs text-muted-foreground">
          正在保存…
        </p>
      )}
    </section>
  )
}
