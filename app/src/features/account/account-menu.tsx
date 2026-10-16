import { reportCaughtError } from '@/lib/diagnostics'
import { ErrorNotice } from '@/components/error-notice'
import { hasPersonalPages } from '@/features/music/source-capabilities'
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
  const { adapters, loading: accountLoading, error, refresh } = useAccountAdapters()
  const { preferences, loading: sourceLoading, error: sourceError } = useMusicSources()
  const [open, setOpen] = useState(false)
  const [busy, setBusy] = useState(false)
  const [actionError, setActionError] = useState('')
  const personalAdapters = adapters.filter((item) => hasPersonalPages(item.descriptor))
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
      reportCaughtError('app/src/features/account/account-menu.tsx', cause)

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
        <span className="max-w-48 truncate">
          {adapter?.current?.displayName ??
            (accountLoading || sourceLoading
              ? '正在恢复…'
              : error || sourceError
                ? '账号暂不可用'
                : '未登录')}
        </span>
      </Popover.Trigger>
      <Popover.Portal>
        <Popover.Positioner side="bottom" align="end" sideOffset={8} className="z-50">
          <Popover.Popup className="max-h-[70vh] w-96 max-w-[calc(100vw-2rem)] overflow-y-auto rounded-xl border border-border bg-popover p-4 text-popover-foreground shadow-lg outline-none">
            <Popover.Title className="text-sm font-semibold">音乐账号</Popover.Title>
            <Popover.Description className="mt-1 text-xs text-muted-foreground">
              音乐库与发现：{name}
              {adapter?.enabled ? '' : ' · 不可用'}
            </Popover.Description>
            {(actionError || error || sourceError) && (
              <ErrorNotice error={actionError || error || sourceError} />
            )}
            <div className="mt-4">
              <SavedAccounts
                open={open}
                showSeparator={false}
                sources={personalAdapters.map((item) => ({
                  source: item.descriptor.source,
                  name: item.descriptor.displayName,
                  current: item.descriptor.source === current ? item.current?.reference : undefined,
                  enabled: item.enabled && !busy
                }))}
                onChanged={() => void refresh()}
              />
              <div className="mt-2 flex flex-wrap gap-2">
                <ActionButton
                  variant="ghost"
                  size="sm"
                  disabled={busy || !personalAdapters.some((item) => item.enabled)}
                  onClick={() => {
                    setOpen(false)
                    navigate(
                      'accounts',
                      adapter && hasPersonalPages(adapter.descriptor) ? current : undefined
                    )
                  }}
                >
                  <UserPlus aria-hidden="true" />
                  {adapter?.current ? '添加账号' : '登录账号'}
                </ActionButton>
                {adapter?.current && hasPersonalPages(adapter.descriptor) && (
                  <ActionButton
                    variant="ghost"
                    size="sm"
                    disabled={busy || !adapter.enabled}
                    onClick={() =>
                      void act(async () => {
                        const result = await musicClient.account(adapter.descriptor.source, {
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
            </div>
          </Popover.Popup>
        </Popover.Positioner>
      </Popover.Portal>
    </Popover.Root>
  )
}
