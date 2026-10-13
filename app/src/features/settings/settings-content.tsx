import { GeneralPage } from './pages/general'
import { PersonalizationPage } from './pages/personalization'
import { PlaybackPage } from './pages/playback'
import { LyricsPage } from './pages/lyrics'
import { LocalCachePage } from './pages/local-cache'
import { AdaptersPage } from './pages/adapters'
import { PluginsPage } from './pages/plugins'
import { ShortcutsPage } from './pages/shortcuts'
import { AboutPage } from './pages/about'
import type { Theme } from './use-theme'
import type { SettingsSection } from './settings-sections'
import { version } from '../../../package.json'

export function SettingsContent({
  section,
  theme,
  onThemeChange,
  pluginId
}: {
  section: SettingsSection
  theme: Theme
  onThemeChange: (theme: Theme) => void
  pluginId?: string
}) {
  switch (section) {
    case 'general':
      return <GeneralPage theme={theme} onThemeChange={onThemeChange} />
    case 'personalization':
      return <PersonalizationPage />
    case 'playback':
      return <PlaybackPage />
    case 'lyrics':
      return <LyricsPage />
    case 'local-cache':
      return <LocalCachePage />
    case 'adapters':
      return <AdaptersPage />
    case 'plugins':
      return <PluginsPage initialPluginId={pluginId} />
    case 'shortcuts':
      return <ShortcutsPage />
    case 'about':
      return <AboutPage version={version} />
  }
}
