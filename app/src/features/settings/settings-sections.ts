import {
  Blocks,
  FolderCog,
  Info,
  Keyboard,
  LayoutGrid,
  Mic2,
  Settings,
  SlidersHorizontal,
  Volume2
} from 'lucide-react'

export const settingsSections = [
  { id: 'general', label: '常规设置', description: '调整应用行为与匿名使用统计。', icon: Settings },
  {
    id: 'personalization',
    label: '个性化设置',
    description: '调整主题、字体、播放栏与界面密度。',
    icon: SlidersHorizontal
  },
  { id: 'playback', label: '播放设置', description: '调整音质与播放行为。', icon: Volume2 },
  { id: 'shortcuts', label: '快捷键设置', description: '配置播放快捷键。', icon: Keyboard },
  { id: 'lyrics', label: '歌词设置', description: '调整歌词显示与来源。', icon: Mic2 },
  { id: 'local-cache', label: '本地与缓存', description: '管理本地音乐与缓存。', icon: FolderCog },
  { id: 'adapters', label: '适配器设置', description: '管理音乐来源。', icon: Blocks },
  { id: 'plugins', label: '插件', description: '管理功能插件。', icon: LayoutGrid },
  { id: 'about', label: '关于', description: '了解 Nons。', icon: Info }
] as const
export type SettingsSection = (typeof settingsSections)[number]['id']
