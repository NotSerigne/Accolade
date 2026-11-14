// src/lib/stores/settings.ts
import { emit } from '@tauri-apps/api/event';
import { Store } from '@tauri-apps/plugin-store';
import { writable } from 'svelte/store';

export type NotifStyle = 'accolade' | 'ps5' | 'xbox360' | 'steamdeck' | 'retro';

export interface NotifAchievement {
	name?: string;
	desc?: string;
	icon?: string;
	rarity?: string;
	is_platinum?: boolean;
	completionpercentage?: number;
	unlocked?: number;
	total?: number;
	unlocked_time?: number;
	is_progress?: boolean;
	progress_current?: number;
	progress_max?: number;
	progress_milestone?: number;
}

export const NOTIF_STYLES: readonly NotifStyle[] = [
	'accolade',
	'ps5',
	'xbox360',
	'steamdeck',
	'retro'
];

export function normalizeNotifStyle(value: unknown): NotifStyle {
	if (typeof value === 'string' && (NOTIF_STYLES as readonly string[]).includes(value)) {
		return value as NotifStyle;
	}
	return 'accolade';
}

export interface AppSettings {
	steamId: string;
	steamApiKey: string;
	raUsername: string;
	raApiKey: string;
	steamGridDbApiKey: string;
	searchPaths: string[];
	language: 'fr' | 'en' | 'es' | 'de' | 'it';
	setupCompleted: boolean;
	windowPosition:
		| 'top-left'
		| 'top-center'
		| 'top-right'
		| 'bottom-left'
		| 'bottom-center'
		| 'bottom-right';
	notif_style: NotifStyle;
	notificationSound: string;
	progressSound: string;
	platinumSound: string;
	notificationVolume: number;
	theme: 'dark' | 'light' | 'system';
	accentColor: string;
	dynamicTheme: boolean;
	launchOnStartup: boolean;
	startMinimized: boolean;
	minimizeToTray: boolean;
	screenshotShortcut: string;
	hudShortcut: string;
	discordRpc: boolean;
	replayBuffer: boolean;
}

export const DEFAULTS: AppSettings = {
	steamId: '',
	steamApiKey: '',
	raUsername: '',
	raApiKey: '',
	steamGridDbApiKey: '',
	searchPaths: [],
	language: 'fr',
	setupCompleted: false,
	windowPosition: 'bottom-right',
	notif_style: 'accolade',
	notificationSound: 'Steam.mp3',
	progressSound: 'Steamdeck.mp3',
	platinumSound: 'PS5 Platinum.mp3',
	notificationVolume: 0.7,
	theme: 'system',
	accentColor: '#c8a96e',
	dynamicTheme: true,
	launchOnStartup: false,
	startMinimized: false,
	minimizeToTray: false,
	screenshotShortcut: 'F12',
	hudShortcut: 'Alt+A',
	discordRpc: true,
	replayBuffer: false
};

const THEME_CACHE_KEY = 'accolade:theme:v1';

type ThemeCache = Pick<AppSettings, 'theme' | 'accentColor'>;

function readThemeCache(): ThemeCache | null {
	if (typeof localStorage === 'undefined') return null;

	try {
		const raw = localStorage.getItem(THEME_CACHE_KEY);
		if (!raw) return null;

		const parsed = JSON.parse(raw) as Partial<ThemeCache>;
		const theme =
			parsed.theme === 'light' || parsed.theme === 'dark' || parsed.theme === 'system'
				? parsed.theme
				: null;
		const accentColor = typeof parsed.accentColor === 'string' ? parsed.accentColor : null;

		if (!theme || !accentColor) return null;
		return { theme, accentColor };
	} catch {
		return null;
	}
}

function writeThemeCache(s: AppSettings): void {
	if (typeof localStorage === 'undefined') return;

	try {
		localStorage.setItem(
			THEME_CACHE_KEY,
			JSON.stringify({ theme: s.theme, accentColor: s.accentColor })
		);
	} catch {
		// empty
	}
}

const INITIAL_SETTINGS: AppSettings = { ...DEFAULTS, ...(readThemeCache() ?? {}) };

export const settings = writable<AppSettings>(INITIAL_SETTINGS);

let store: Store | null = null;
let removeSystemThemeListener: (() => void) | null = null;

async function getStore(): Promise<Store> {
	if (!store) {
		store = await Store.load('settings.json');
	}
	return store;
}

function normalizeSound(value: unknown, fallback: string): string {
	return typeof value === 'string' && value.length > 0 ? value : fallback;
}

export async function loadSettings(): Promise<void> {
	const cached = readThemeCache();
	if (cached) {
		const quick = { ...DEFAULTS, ...cached };
		settings.set(quick);
		applyTheme(quick);
	}

	const s = await getStore();
	const saved: Partial<AppSettings> = {};
	const keys = Object.keys(DEFAULTS) as (keyof AppSettings)[];
	const entries = await Promise.all(
		keys.map(async (key) => {
			const val = await s.get<AppSettings[typeof key]>(key);
			return [key, val] as const;
		})
	);

	for (const [key, val] of entries) {
		if (val !== null && val !== undefined) {
			(saved as Record<string, unknown>)[key] = val;
		}
	}

	const merged: AppSettings = {
		...DEFAULTS,
		...saved,
		notif_style: normalizeNotifStyle(saved.notif_style),
		notificationSound: normalizeSound(saved.notificationSound, DEFAULTS.notificationSound),
		progressSound: normalizeSound(saved.progressSound, DEFAULTS.progressSound),
		platinumSound: normalizeSound(saved.platinumSound, DEFAULTS.platinumSound)
	};
	settings.set(merged);
	applyTheme(merged);
	writeThemeCache(merged);
}

export async function saveSettings(next: AppSettings): Promise<void> {
	const normalized: AppSettings = {
		...next,
		notif_style: normalizeNotifStyle(next.notif_style),
		notificationSound: normalizeSound(next.notificationSound, DEFAULTS.notificationSound),
		progressSound: normalizeSound(next.progressSound, DEFAULTS.progressSound),
		platinumSound: normalizeSound(next.platinumSound, DEFAULTS.platinumSound)
	};
	const s = await getStore();
	await Promise.all(Object.entries(normalized).map(([key, val]) => s.set(key, val)));
	await s.save();
	settings.set(normalized);
	applyTheme(normalized);
	writeThemeCache(normalized);
	if (typeof window !== 'undefined') {
		try {
			await emit('settings-updated');
		} catch {
			// Ignore when running outside Tauri webview context
		}
	}
}

export function applyTheme(s: AppSettings): void {
	if (typeof document === 'undefined') return;

	const root = document.documentElement;
	const mediaQuery =
		typeof window !== 'undefined' ? window.matchMedia('(prefers-color-scheme: dark)') : null;
	const prefersDark = mediaQuery ? mediaQuery.matches : true;
	const resolvedTheme = s.theme === 'system' ? (prefersDark ? 'dark' : 'light') : s.theme;

	root.setAttribute('data-theme', resolvedTheme);
	root.style.setProperty('--accent-default', s.accentColor);
	root.style.setProperty('--accent', s.accentColor);
	root.style.colorScheme = resolvedTheme;

	if (removeSystemThemeListener) {
		removeSystemThemeListener();
		removeSystemThemeListener = null;
	}

	if (!mediaQuery || s.theme !== 'system') return;

	const onChange = () => {
		const nextTheme = mediaQuery.matches ? 'dark' : 'light';
		root.setAttribute('data-theme', nextTheme);
		root.style.colorScheme = nextTheme;
		root.style.setProperty('--accent', s.accentColor);
	};
	mediaQuery.addEventListener('change', onChange);
	removeSystemThemeListener = () => mediaQuery.removeEventListener('change', onChange);
}
