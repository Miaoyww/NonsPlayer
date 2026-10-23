import { isAndroid } from '@/lib/platform'
import { getShortcutStatus, shortcutActions } from '@/features/playback/shortcuts/shortcuts'
import type { SettingsSection } from './settings-sections'

export interface SettingsSearchItem {
  id: string
  section: SettingsSection
  title: string
  path: string
  keywords?: string
}

type SearchGroup = {
  section: SettingsSection
  path: string
  items: [id: string, title: string, keywords?: string][]
}

const groups: SearchGroup[] = [
  {
    section: 'general',
    path: '常规设置 / 歌曲操作',
    items: [
      ['track-double-click-action', '双击播放行为', '歌曲 加入下一曲 立即播放'],
      ['track-double-click-scope', '双击播放范围', '歌曲 播放此队列 当前列表 仅播放这一首 单曲']
    ]
  },
  {
    section: 'general',
    path: '常规设置 / 窗口与启动',
    items: [
      ['close-behavior', '关闭主面板时', '最小化 托盘 退出'],
      ['autostart', '开机自启', '登录 启动']
    ]
  },
  {
    section: 'general',
    path: '常规设置 / 隐私与统计',
    items: [['telemetry', '匿名使用统计', '遥测 隐私']]
  },
  {
    section: 'personalization',
    path: '个性化设置 / 外观',
    items: [
      ['theme', '界面主题', '浅色 深色 跟随系统'],
      ['app-font', '应用字体', '字型'],
      ['interface-density', '界面密度', '紧凑式 标准式 留白 行高']
    ]
  },
  {
    section: 'personalization',
    path: '个性化设置 / 播放器外观',
    items: [['playback-bar', '播放栏', '折叠式 常驻式 关闭 播放器样式']]
  },
  {
    section: 'playback',
    path: '播放设置',
    items: [
      ['output-device', '输出设备', '扬声器 耳机 声卡 系统默认'],
      ['playback-engine', '播放引擎', 'WebAudio GStreamer 浏览器 audio'],
      ['quality-downgrade', '允许音质降级', '不可用'],
      ['playback-fades', '播放/暂停渐变', '淡入 淡出 过渡'],
      ['gapless', '无缝播放（Gapless）', '连续 衔接 预加载'],
      ['crossfade', '交叉淡入淡出', 'WebAudio 自动切歌 重叠 过渡时长']
    ]
  },
  {
    section: 'shortcuts',
    path: '快捷键设置 / 应用内快捷键',
    items: [
      ['shortcut-app-toggle', '播放/暂停音乐', 'Space 空格'],
      ['shortcut-app-search', '快捷搜索歌曲', 'Ctrl F Command 查找']
    ]
  },
  {
    section: 'shortcuts',
    path: '快捷键设置 / 全局快捷键',
    items: [['global-shortcuts', '启用全局快捷键', '后台 组合键']]
  },
  {
    section: 'lyrics',
    path: '歌词设置 / 歌词显示',
    items: [
      ['lyrics-font', '歌词字体', '翻译 发音'],
      ['lyrics-font-weight', '歌词字重', '常规 中等 粗体 自定义 粗细 400 500 700'],
      ['lyrics-auto-blur', '歌词自动模糊', '模糊 清晰 AMLL blur'],
      ['lyrics-background-speed', '背景流速', '流动 动态 静止 速度']
    ]
  },
  {
    section: 'lyrics',
    path: '歌词设置 / 来源',
    items: [
      ['lyrics-amll', 'AMLL DB', '歌词 逐字 TTML'],
      ['lyrics-qq', 'QQ 音乐', '歌词 匹配 网易云']
    ]
  },
  {
    section: 'local-cache',
    path: '本地与缓存 / 本地歌曲',
    items: [
      ['local-lyrics-source', '来源', '歌词 本地 在线 LRC 内嵌'],
      ['local-covers', '显示本地歌曲封面', '图片 内嵌'],
      ['artist-separators', '艺术家分隔符', '歌手 拆分 标签'],
      ['music-folders', '音乐文件夹', '目录 扫描 导入']
    ]
  },
  {
    section: 'local-cache',
    path: '本地与缓存 / 缓存配置',
    items: [
      ['local-cache-enabled', '启用本地缓存', '歌词 离线 TTML'],
      ['cache-limit', '缓存大小上限', '容量 MB 清理'],
      ['cache-directory', '缓存目录', '路径 更改 迁移'],
      ['cache-cleanup', '缓存占用与清理', '清空 删除 刷新 容量']
    ]
  },
  {
    section: 'about',
    path: '关于',
    items: [
      ['app-version', '版本号', '版本 Nons'],
      ['app-updates', '版本更新', '升级 下载 更新'],
      ['license', '开源许可', 'GPL 协议'],
      ['repository', 'GitHub 仓库', '源码 Issue 反馈']
    ]
  },
  {
    section: 'about',
    path: '关于 / 版本更新',
    items: [
      ['update-auto-check', '启动时自动检查', '自动更新'],
      ['update-check', '检查更新', '升级 下载']
    ]
  },
  {
    section: 'about',
    path: '关于 / 特别鸣谢',
    items: [
      ['thanks-qier222/YesPlayMusic', 'YesPlayMusic'],
      ['thanks-SPlayer-Dev/SPlayer', 'SPlayer'],
      ['thanks-Ferry-200/coriander_player', 'coriander_player'],
      ['thanks-amll-dev/applemusic-like-lyrics', 'Apple Music-like Lyrics'],
      ['thanks-SPlayer-Dev/ncm-api-rs', 'ncm-api-rs']
    ]
  }
]

const desktopOnly = new Set(['close-behavior', 'autostart', 'playback-bar'])

export function getSettingsSearchItems(): SettingsSearchItem[] {
  const items = groups
    .flatMap(({ section, path, items }) =>
      items.map(([id, title, keywords]) => ({ id, section, title, path, keywords }))
    )
    .filter(({ id }) => !isAndroid() || !desktopOnly.has(id))
  if (getShortcutStatus().settings.enabled) {
    items.push(
      ...shortcutActions.map(([action, title]) => ({
        id: `shortcut-global-${action}`,
        section: 'shortcuts' as const,
        title,
        path: '快捷键设置 / 全局快捷键',
        keywords: '后台 快捷键 组合键'
      }))
    )
  }
  if (import.meta.env.DEV) {
    items.push({
      id: 'devtools',
      section: 'about',
      title: '开发者工具',
      path: '关于',
      keywords: '调试 控制台'
    })
  }
  return items
}
