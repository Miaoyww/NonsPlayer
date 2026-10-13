import { MusicPage } from '@/components/music/music-page'
import { useMusicNavigation } from '@/features/workspace/music-navigation'
import { SettingsContent } from './settings-content'
import { settingsSections } from './settings-sections'
import { useTheme } from './use-theme'
import { SettingsHome } from './settings-home'
import { isAndroid } from '@/lib/platform'

export default function SettingsPage() {
  const { page, navigate } = useMusicNavigation()
  const [theme, setTheme] = useTheme()
  const [sectionId, ...pluginParts] = page.query.split('/')
  const section = settingsSections.find(
    (item) => item.id === sectionId && !(isAndroid() && item.id === 'shortcuts')
  )
  return (
    <MusicPage
      className={`mobile-settings-page ${section ? 'mobile-settings-detail' : 'mobile-settings-home'}`}
      aria-label={section?.label ?? '设置'}
    >
      {section ? (
        <SettingsContent
          section={section.id}
          theme={theme}
          onThemeChange={setTheme}
          pluginId={pluginParts.join('/') || undefined}
        />
      ) : (
        <SettingsHome onSelect={(view, query) => navigate(view, query)} />
      )}
    </MusicPage>
  )
}
