import { ContextMenu } from '@base-ui/react/context-menu'
import { Copy, Save } from 'lucide-react'
import { useRef, useState, type ReactElement } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { useSongActions } from '@/components/music/song-actions'
import { coverOriginal, type CoverInput } from '@/features/music/cover-image'
import { nativeCall } from '@/lib/player'

export function CoverContextMenu({
  cover,
  title,
  children
}: {
  cover?: CoverInput
  title: string
  children: ReactElement
}) {
  const actions = useSongActions()
  const running = useRef(false)
  const [busy, setBusy] = useState(false)
  const url = coverOriginal(cover)
  const disabled = !url || !isTauri() || busy
  function run(action: 'copy' | 'save') {
    if (disabled || running.current) return
    running.current = true
    setBusy(true)
    actions.run(
      async () => {
        try {
          return await nativeCall<boolean>('cover_export', { url, title, action })
        } finally {
          running.current = false
          setBusy(false)
        }
      },
      (done) => (done ? (action === 'copy' ? '已复制封面原图。' : '封面已保存。') : undefined)
    )
  }
  return (
    <ContextMenu.Root>
      <ContextMenu.Trigger render={children} tabIndex={0} aria-label="封面菜单" />
      <ContextMenu.Portal>
        <ContextMenu.Positioner className="z-[70]" sideOffset={4}>
          <ContextMenu.Popup className="song-context-menu" aria-label="封面菜单">
            <ContextMenu.Item disabled={disabled} onClick={() => run('copy')}>
              <Copy aria-hidden="true" />
              复制原图
            </ContextMenu.Item>
            <ContextMenu.Item disabled={disabled} onClick={() => run('save')}>
              <Save aria-hidden="true" />
              另存为…
            </ContextMenu.Item>
          </ContextMenu.Popup>
        </ContextMenu.Positioner>
      </ContextMenu.Portal>
    </ContextMenu.Root>
  )
}
