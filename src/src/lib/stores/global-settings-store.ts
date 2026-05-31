import { writable } from 'svelte/store';
import type { MusicAdapter } from '$lib/const';

export interface GlobalSettings {

	/** 是否已完成/跳过欢迎向导 */
	welcomeCompleted: boolean;
	/** 已选择的适配器 ID 列表 */
	selectedAdapters: string[];
	/** 主题偏好 */
	theme: 'light' | 'dark' | 'system';
	/** 本地音乐文件夹路径列表 */
	localMusicFolders: string[];
	/** 界面语言 */
	language: 'zh-cn' | 'en';
	/** 本地歌词优先 (true) vs 在线优先 (false) */
	localLyricFirst: boolean;
	/** 显示歌词翻译行 */
	showLyricTran: boolean;
	/** 显示歌词罗马音行 */
	showLyricRoma: boolean;

	// ── 全屏播放器设置 ──

	/** 播放器封面类型 */
	playerType: 'fullscreen' | 'cover' | 'record';
	/** 播放器背景类型 */
	playerBackgroundType: 'color' | 'blur' | 'animation';
	/** 左右分栏比例（左侧百分比） */
	playerStyleRatio: number;
	/** 背景动画帧率 */
	playerBackgroundFps: number;
	/** 背景动画流动速度 */
	playerBackgroundFlowSpeed: number;
	/** 背景渲染缩放比例 */
	playerBackgroundRenderScale: number;
	/** 暂停时暂停背景动画 */
	playerBackgroundPause: boolean;
	/** 启用低频脉动效果 */
	playerBackgroundLowFreqVolume: boolean;
	/** 自动隐藏播放器控件 */
	autoHidePlayerMeta: boolean;
	/** 显示逐字歌词 */
	showWordLyrics: boolean;
	/** 显示歌词翻译（即时歌词） */
	showTran: boolean;
	/** 歌词混合模式 */
	lyricsBlendMode: string;

	// ── AMLL 歌词显示设置 ──

	/** 逐字渐变宽度 (0~1) */
	lyricWordFadeWidth: number;
	/** 为非焦点行启用模糊效果 */
	lyricEnableBlur: boolean;
	/** 使用物理弹簧替代 CSS transition */
	lyricEnableSpring: boolean;
	/** 已过行缩放效果 */
	lyricEnableScale: boolean;
	/** 隐藏已播放行 */
	lyricHidePassedLines: boolean;
	/** 歌词对齐锚点 */
	lyricAlignAnchor: 'top' | 'bottom' | 'center';
	/** 歌词对齐位置 (0~1) */
	lyricAlignPosition: number;
	/** 背景动画静态模式（暂停时固定流动） */
	playerBackgroundStaticMode: boolean;
}

const STORAGE_KEY = 'nonsplayer_settings';

const DEFAULTS: GlobalSettings = {
	welcomeCompleted: false,
	selectedAdapters: [],
	theme: 'system',
	localMusicFolders: [],
	language: 'zh-cn',
	localLyricFirst: true,
	showLyricTran: true,
	showLyricRoma: true,
	playerType: 'cover',
	playerBackgroundType: 'blur',
	playerStyleRatio: 50,
	playerBackgroundFps: 30,
	playerBackgroundFlowSpeed: 4,
	playerBackgroundRenderScale: 0.5,
	playerBackgroundPause: false,
	playerBackgroundLowFreqVolume: false,
	autoHidePlayerMeta: true,
	showWordLyrics: true,
	showTran: true,
	lyricsBlendMode: 'normal',
	lyricWordFadeWidth: 0.5,
	lyricEnableBlur: true,
	lyricEnableSpring: true,
	lyricEnableScale: true,
	lyricHidePassedLines: false,
	lyricAlignAnchor: 'center',
	lyricAlignPosition: 0.35,
	playerBackgroundStaticMode: false,
};

function loadSettings(): GlobalSettings {
	if (typeof localStorage === 'undefined') return { ...DEFAULTS };
	try {
		const raw = localStorage.getItem(STORAGE_KEY);
		return raw ? { ...DEFAULTS, ...JSON.parse(raw) } : { ...DEFAULTS };
	} catch {
		return { ...DEFAULTS };
	}
}

function createGlobalSettings() {
	const { subscribe, set, update } = writable<GlobalSettings>(loadSettings());

	function persist(value: GlobalSettings) {
		if (typeof localStorage !== 'undefined') {
			localStorage.setItem(STORAGE_KEY, JSON.stringify(value));
		}
	}

	return {
		subscribe,
		patch(partial: Partial<GlobalSettings>) {
			update((s) => {
				const next = { ...s, ...partial };
				persist(next);
				return next;
			});
		},
		/** 标记欢迎向导已完成 */
		completeWelcome() {
			update((s) => {
				const next = { ...s, welcomeCompleted: true };
				persist(next);
				return next;
			});
		},
		reset() {
			set({ ...DEFAULTS });
			if (typeof localStorage !== 'undefined') {
				localStorage.removeItem(STORAGE_KEY);
			}
		}
	};
}

export const globalSettings = createGlobalSettings();
