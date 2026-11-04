import { ActionButton } from '@/components/music/action-button'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Popover } from '@base-ui/react/popover'
import { Minus, Plus, TimerReset } from 'lucide-react'
import { useEffect, useState } from 'react'
import { clampLocalMvOffset, localMvOffsetStepMs, maxLocalMvOffsetMs } from './sync'

function seconds(offsetMs: number) {
  return String(offsetMs / 1000)
}

export function MvAlignmentPopover({
  offsetMs,
  onChange
}: {
  offsetMs: number
  onChange: (value: number) => void
}) {
  const [open, setOpen] = useState(false)
  const [draft, setDraft] = useState(() => seconds(offsetMs))

  useEffect(() => {
    setDraft(seconds(offsetMs))
  }, [offsetMs])

  const apply = (value: number) => {
    const next = clampLocalMvOffset(value)
    setDraft(seconds(next))
    onChange(next)
  }
  const draftMs = () => {
    const value = draft.trim()
    if (!value) return offsetMs
    const parsed = Number(value)
    return Number.isFinite(parsed) ? Math.round(parsed * 1000) : offsetMs
  }
  const commit = () => {
    const value = draftMs()
    setDraft(seconds(value))
    onChange(clampLocalMvOffset(value))
  }

  return (
    <Popover.Root
      open={open}
      onOpenChange={(value) => {
        if (!value) commit()
        setOpen(value)
      }}
    >
      <Popover.Trigger
        render={
          <ActionButton
            variant="ghost"
            size="icon"
            className="fullscreen-mode-button"
            aria-label="调整 MV 开头偏移"
            title="调整 MV 开头偏移"
          />
        }
      >
        <TimerReset aria-hidden="true" />
      </Popover.Trigger>
      <Popover.Portal>
        <Popover.Positioner
          side="bottom"
          align="end"
          sideOffset={8}
          collisionPadding={16}
          className="z-[70]"
        >
          <Popover.Popup className="mv-alignment-popover" aria-label="MV 开头偏移">
            <Popover.Title className="mv-alignment-title">MV 开头对齐</Popover.Title>
            <div className="mv-alignment-controls">
              <Button
                type="button"
                variant="outline"
                size="icon"
                aria-label="MV 提前 100 毫秒"
                title="提前 100 毫秒"
                onClick={() => apply(draftMs() - localMvOffsetStepMs)}
              >
                <Minus aria-hidden="true" />
              </Button>
              <Button
                type="button"
                variant="outline"
                size="icon"
                aria-label="MV 延后 100 毫秒"
                title="延后 100 毫秒"
                onClick={() => apply(draftMs() + localMvOffsetStepMs)}
              >
                <Plus aria-hidden="true" />
              </Button>
              <Input
                className="mv-alignment-input"
                type="number"
                inputMode="decimal"
                min={-maxLocalMvOffsetMs / 1000}
                max={maxLocalMvOffsetMs / 1000}
                step="0.1"
                value={draft}
                aria-label="MV 进入偏移（秒）"
                onChange={(event) => {
                  const value = event.currentTarget.value
                  setDraft(value)
                  const parsed = Number(value)
                  if (value.trim() && Number.isFinite(parsed))
                    onChange(clampLocalMvOffset(parsed * 1000))
                }}
                onBlur={commit}
                onKeyDown={(event) => {
                  if (event.key === 'Enter') event.currentTarget.blur()
                }}
              />
              <span className="text-sm text-muted-foreground">秒</span>
            </div>
            <p className="mv-alignment-hint">正数让 MV 延后进入，负数提前；范围 ±10 分钟。</p>
          </Popover.Popup>
        </Popover.Positioner>
      </Popover.Portal>
    </Popover.Root>
  )
}
