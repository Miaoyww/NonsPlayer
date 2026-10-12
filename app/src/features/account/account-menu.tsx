import { useState } from 'react'
import { Popover } from '@base-ui/react/popover'
import { LogOut, UserPlus } from 'lucide-react'
import { ActionButton } from '@/components/music/action-button'
import { musicClient } from '@/features/music/client'
import { useMusicSources } from '@/features/music/use-music-sources'
import { sourceLabel } from '@/features/music/source-label'
import { useMusicNavigation } from '@/features/workspace/music-navigation'
import { errorText } from '@/lib/player'
import { AccountAvatar } from './account-avatar'
import { SavedAccounts } from './saved-accounts'
import { useAccountAdapters } from './use-account-adapters'

export function AccountMenu() {
  const { navigate } = useMusicNavigation()
  const { adapters, error, refresh } = useAccountAdapters()
  const { preferences, error: sourceError } = useMusicSources()
  const [open, setOpen] = useState(false)
  const [busy, setBusy] = useState(false)
  const [actionError, setActionError] = useState('')
  const current = preferences?.current
  const adapter = adapters.find((item) => item.descriptor.source === current)
  const name = adapter?.descriptor.displayName ?? (current ? sourceLabel(current) : '音乐来源')
  async function act(operation: () => Promise<unknown>) {
    if (busy) return
    setBusy(true)
    setActionError('')
    try {
      await operation()
      await refresh()
    } catch (cause) {
      setActionError(errorText(cause))
    } finally {
      setBusy(false)
    }
  }
  return (
    <Popover.Root
      open={open}
      onOpenChange={(value) => {
        setOpen(value)
        if (value) void refresh()
      }}
    >
      <Popover.Trigger
        render={<ActionButton variant="ghost" size="sm" className="gap-2" aria-label="音乐账号" />}
      >
        <AccountAvatar avatar={adapter?.current?.avatar} />
        <span className="max-w-48 truncate">{adapter?.current?.displayName ?? '未登录'}</span>
      </Popover.Trigger>
      <Popover.Portal>
        <Popover.Positioner side="bottom" align="end" sideOffset={8} className="z-50">
          <Popover.Popup className="max-h-[70vh] w-96 max-w-[calc(100vw-2rem)] overflow-y-auto rounded-xl border border-border bg-popover p-4 text-popover-foreground shadow-lg outline-none">
            <Popover.Title className="text-sm font-semibold">音乐账号</Popover.Title>
            <Popover.Description className="mt-1 text-xs text-muted-foreground">
              当前来源：{name}
              {adapter?.enabled ? '' : ' · 不可用'}
            </Popover.Description>
            {(actionError || error || sourceError) && (
              <p role="alert" className="mt-3 text-sm text-destructive">
                {actionError || error || sourceError}
              </p>
            )}
            {adapters.map((item) => {
              const { source, displayName } = item.descriptor
              return (
                <section key={source} className="mt-4" aria-label={`${displayName}账号`}>
                  <SavedAccounts
                    open={open}
                    showSeparator={false}
                    source={source}
                    sourceName={displayName}
                    current={item.current?.reference}
                    enabled={item.enabled && !busy}
                    onChanged={() => void refresh()}
                  />
                  <div className="mt-2 flex flex-wrap gap-2">
                    <ActionButton
                      variant="ghost"
                      size="sm"
                      disabled={busy || !item.enabled}
                      onClick={() => {
                        setOpen(false)
                        navigate('accounts', source)
                      }}
                    >
                      <UserPlus aria-hidden="true" />
                      {item.current ? '添加账号' : '登录账号'}
                    </ActionButton>
                    {item.current && (
                      <ActionButton
                        variant="ghost"
                        size="sm"
                        disabled={busy || !item.enabled}
                        onClick={() =>
                          void act(async () => {
                            const result = await musicClient.account(source, {
                              operation: 'logout'
                            })
                            if (result.type !== 'logout' || !result.data.localCleared)
                              throw new Error('退出当前账号失败，请重试。')
                          })
                        }
                      >
                        <LogOut aria-hidden="true" />
                        退出账号
                      </ActionButton>
                    )}
                  </div>
                </section>
              )
            })}
          </Popover.Popup>
        </Popover.Positioner>
      </Popover.Portal>
    </Popover.Root>
  )
}
