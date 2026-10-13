import { ChevronRight } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { AccountAvatar } from '@/features/account/account-avatar'
import { useAccountAdapters } from '@/features/account/use-account-adapters'
import { useMusicSources } from '@/features/music/use-music-sources'
import { settingsSections } from './settings-sections'

export function SettingsHome({
  onSelect
}: {
  onSelect: (view: 'accounts' | 'settings', query?: string) => void
}) {
  const { preferences } = useMusicSources()
  const { adapters } = useAccountAdapters()
  const account = adapters.find((item) => item.descriptor.source === preferences?.current)?.current
  const card = ({ id, label, icon: Icon }: (typeof settingsSections)[number]) => (
    <Button
      key={id}
      variant="secondary"
      className="mobile-settings-category"
      onClick={() => onSelect('settings', id)}
    >
      <Icon aria-hidden="true" />
      <span className="flex-1 text-left">{label}</span>
      <ChevronRight aria-hidden="true" />
    </Button>
  )
  return (
    <div className="settings-home-content">
      <div className="settings-home-scroll">
        <Button
          variant="secondary"
          className="mobile-settings-account"
          onClick={() => onSelect('accounts')}
        >
          <AccountAvatar avatar={account?.avatar} className="size-10" />
          <span className="min-w-0 flex-1 truncate text-left">
            {account?.displayName ?? '音乐账号'}
          </span>
          <ChevronRight aria-hidden="true" />
        </Button>
        <div role="separator" className="my-6 border-t border-border" />
        <nav aria-label="设置分类" className="mobile-settings-categories">
          {settingsSections
            .filter((item) => item.id !== 'plugins' && item.id !== 'about')
            .map(card)}
        </nav>
      </div>
      <nav aria-label="插件与关于" className="settings-home-footer">
        {['about', 'plugins'].map((id) => card(settingsSections.find((item) => item.id === id)!))}
      </nav>
    </div>
  )
}
