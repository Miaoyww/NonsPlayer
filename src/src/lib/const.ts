export const NONSPLAYER_NAME = "NonsPlayer";
export const NONSPLAYER_TAGLINE = "多音源 | 高性能";
export const NONSPLAYER_DESCRIPTION = "跨平台、多音源的音乐播放器";

/** 可用的音乐平台适配器 */
export interface MusicAdapter {
	id: string;
	name: string;
	description: string;
	icon: string; // hugeicons icon name
	color: string; // tailwind color class
}

export const AVAILABLE_ADAPTERS: MusicAdapter[] = [
	{
		id: "netease",
		name: "网易云音乐",
		description: "连接网易云音乐账号，同步你的歌单和收藏",
		icon: "headphones",
		color: "text-red-500"
	},
	{
		id: "qqmusic",
		name: "QQ音乐",
		description: "接入QQ音乐海量曲库，畅听全网好歌",
		icon: "music-note",
		color: "text-emerald-500"
	},
	{
		id: "local",
		name: "本地音乐",
		description: "播放你设备上的本地音乐文件",
		icon: "folder",
		color: "text-amber-500"
	}
];

/** 主题选项 */
export interface ThemeOption {
	id: "light" | "dark" | "system";
	name: string;
	description: string;
	icon: string;
}

export const THEME_OPTIONS: ThemeOption[] = [
	{
		id: "light",
		name: "浅色",
		description: "明亮的界面风格",
		icon: "sun"
	},
	{
		id: "dark",
		name: "深色",
		description: "护眼的暗色模式",
		icon: "moon"
	},
	{
		id: "system",
		name: "跟随系统",
		description: "自动切换浅色/深色",
		icon: "settings"
	}
];
