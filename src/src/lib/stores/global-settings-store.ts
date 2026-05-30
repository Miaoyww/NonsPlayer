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
	/** 启用 AMLL TTML 歌词库 */
	enableAmllDb: boolean;
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
}

const STORAGE_KEY = 'nonsplayer_settings';

const DEFAULTS: GlobalSettings = {
	welcomeCompleted: false,
	selectedAdapters: [],
	theme: 'system',
	localMusicFolders: [],
	language: 'zh-cn',
	localLyricFirst: true,
	enableAmllDb: true,
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
