import { ErrorNotice } from '@/components/error-notice'
import { useState, type FormEvent } from 'react'
import { Plus } from 'lucide-react'
import { musicClient } from '@/features/music/client'
import { currentSource } from '@/features/music/queries'
import { errorText } from '@/lib/player'
import {
  Dialog,
  DialogClose,
  DialogDescription,
  DialogContent,
  DialogTitle,
  DialogTrigger
} from '@/components/ui/dialog'
import { Button as ActionButton } from '@/components/ui/button'

export function CreatePlaylist({
  onCreated,
  source,
  iconOnly = false,
  disabled = false
}: {
  onCreated: () => void
  source?: string
  iconOnly?: boolean
  disabled?: boolean
}) {
  const [open, setOpen] = useState(false)
  const [name, setName] = useState('')
  const [privatePlaylist, setPrivatePlaylist] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string>()
  async function create(event: FormEvent) {
    event.preventDefault()
    if (busy || !name.trim()) return
    setBusy(true)
    setError(undefined)
    try {
      await musicClient.write(source ?? (await currentSource()), {
        operation: 'createPlaylist',
        name: name.trim(),
        private: privatePlaylist
      })
      setOpen(false)
      setName('')
      onCreated()
    } catch (cause) {
      setError(errorText(cause))
    } finally {
      setBusy(false)
    }
  }
  return (
    <Dialog
      open={open}
      onOpenChange={(value) => {
        if (!busy) {
          setOpen(value)
          setError(undefined)
        }
      }}
    >
      <DialogTrigger
        render={
          <ActionButton
            variant="ghost"
            size={iconOnly ? 'icon' : 'sm'}
            aria-label="新建歌单"
            disabled={disabled}
          />
        }
      >
        <Plus aria-hidden="true" />
        {!iconOnly && '新建歌单'}
      </DialogTrigger>
      <DialogContent className="max-w-sm" showCloseButton={!busy}>
        <DialogTitle>新建歌单</DialogTitle>
        <DialogDescription>把喜欢的音乐整理成一个歌单。</DialogDescription>
        <form onSubmit={(event) => void create(event)} className="mt-4 flex flex-col gap-4">
          <label className="flex flex-col gap-2 text-sm">
            歌单名称
            <input
              autoFocus
              className="music-input w-full"
              maxLength={40}
              value={name}
              onChange={(event) => setName(event.target.value)}
              required
              disabled={busy}
            />
          </label>
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={privatePlaylist}
              onChange={(event) => setPrivatePlaylist(event.target.checked)}
              disabled={busy}
            />
            设为私密歌单
          </label>
          {error && <ErrorNotice error={error} />}
          <div className="flex justify-end gap-2">
            <DialogClose render={<ActionButton variant="outline" disabled={busy} />}>
              取消
            </DialogClose>
            <ActionButton type="submit" disabled={busy || !name.trim()}>
              {busy ? '正在创建…' : '创建'}
            </ActionButton>
          </div>
        </form>
      </DialogContent>
    </Dialog>
  )
}
