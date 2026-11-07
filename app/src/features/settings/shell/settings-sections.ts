import {
  Blocks,
  Download,
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
  {
    id: 'general',
    label: '常规设置',
    description: '调整窗口与启动、引导和隐私统计。',
    icon: Settings
  },
  {
    id: 'personalization',
    label: '个性化设置',
    description: '调整主题、字体、播放栏与界面密度。',
    icon: SlidersHorizontal
  },
  {
    id: 'playback',
    label: '播放设置',
    description: '调整播放控制、歌曲操作与输出设备。',
    icon: Volume2
  },
  {
    id: 'download',
    label: '下载设置',
    description: '设置下载音质、MV 保存与保存目录。',
    icon: Download
  },
  { id: 'shortcuts', label: '快捷键设置', description: '配置播放快捷键。', icon: Keyboard },
  { id: 'lyrics', label: '歌词设置', description: '调整歌词显示与来源。', icon: Mic2 },
  {
    id: 'local-cache',
    label: '本地与缓存',
    description: '管理本地音乐、缓存与日志。',
    icon: FolderCog
  },
  { id: 'adapters', label: '适配器设置', description: '管理音乐来源。', icon: Blocks },
  { id: 'plugins', label: '插件', description: '管理功能插件。', icon: LayoutGrid },
  { id: 'about', label: '关于', description: '了解 Nons。', icon: Info }
] as const
export type SettingsSection = (typeof settingsSections)[number]['id']
