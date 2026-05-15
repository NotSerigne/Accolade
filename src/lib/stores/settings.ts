// src/lib/stores/settings.ts
import { Store } from '@tauri-apps/plugin-store';
import { writable } from 'svelte/store';

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
	notificationSound: string;
	notificationVolume: number;
	theme: 'dark' | 'light' | 'system';
	accentColor: string;
	launchOnStartup: boolean;
	startMinimized: boolean;
	minimizeToTray: boolean;
}

const DEFAULTS: AppSettings = {
	steamId: '',
	steamApiKey: '',
	raUsername: '',
	raApiKey: '',
	steamGridDbApiKey: '',
	searchPaths: [],
	language: 'fr',
	setupCompleted: false,
	windowPosition: 'bottom-right',
	notificationSound: 'Steam.mp3',
	notificationVolume: 0.7,
	theme: 'system',
	accentColor: '#c8a96e',
	launchOnStartup: false,
	startMinimized: false,
	minimizeToTray: false
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

	const merged = { ...DEFAULTS, ...saved };
	settings.set(merged);
	applyTheme(merged);
	writeThemeCache(merged);
}

export async function saveSettings(next: AppSettings): Promise<void> {
	const s = await getStore();
	await Promise.all(Object.entries(next).map(([key, val]) => s.set(key, val)));
	await s.save();
	settings.set(next);
	applyTheme(next);
	writeThemeCache(next);
}

export function applyTheme(s: AppSettings): void {
	if (typeof document === 'undefined') return;

	const root = document.documentElement;
	const mediaQuery =
		typeof window !== 'undefined' ? window.matchMedia('(prefers-color-scheme: dark)') : null;
	const prefersDark = mediaQuery ? mediaQuery.matches : true;
	const resolvedTheme = s.theme === 'system' ? (prefersDark ? 'dark' : 'light') : s.theme;

	root.setAttribute('data-theme', resolvedTheme);
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
