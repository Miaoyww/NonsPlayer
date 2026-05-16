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
}

const STORAGE_KEY = 'nonsplayer_settings';

const DEFAULTS: GlobalSettings = {
	welcomeCompleted: false,
	selectedAdapters: [],
	theme: 'system',
	localMusicFolders: [],
	language: 'zh-cn'
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
