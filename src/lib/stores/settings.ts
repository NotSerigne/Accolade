// src/lib/stores/settings.ts
import { Store } from '@tauri-apps/plugin-store';
import { writable } from 'svelte/store';

export interface AppSettings {
    steamApiKey: string;
    searchPaths: string[];
    windowPosition: 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right' | 'center';
    theme: 'dark' | 'light';
    accentColor: string;
}

const DEFAULTS: AppSettings = {
    steamApiKey: '',
    searchPaths: [],
    windowPosition: 'bottom-right',
    theme: 'dark',
    accentColor: '#c8a96e',
};

const THEME_CACHE_KEY = 'accolade:theme:v1';

type ThemeCache = Pick<AppSettings, 'theme' | 'accentColor'>;

function readThemeCache(): ThemeCache | null {
    if (typeof localStorage === 'undefined') return null;

    try {
        const raw = localStorage.getItem(THEME_CACHE_KEY);
        if (!raw) return null;

        const parsed = JSON.parse(raw) as Partial<ThemeCache>;
        const theme = parsed.theme === 'light' ? 'light' : parsed.theme === 'dark' ? 'dark' : null;
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
        localStorage.setItem(THEME_CACHE_KEY, JSON.stringify({ theme: s.theme, accentColor: s.accentColor }));
    } catch {
        // Ignore local cache write failures.
    }
}

const INITIAL_SETTINGS: AppSettings = { ...DEFAULTS, ...(readThemeCache() ?? {}) };

// Store Svelte réactif
export const settings = writable<AppSettings>(INITIAL_SETTINGS);

let store: Store | null = null;

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

    for (const key of Object.keys(DEFAULTS) as (keyof AppSettings)[]) {
        const val = await s.get<AppSettings[typeof key]>(key);
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
    for (const [key, val] of Object.entries(next)) {
        await s.set(key, val);
    }
    await s.save();
    settings.set(next);
    applyTheme(next);
    writeThemeCache(next);
}

// Applique thème + couleur accent sur :root
export function applyTheme(s: AppSettings): void {
    if (typeof document === 'undefined') return;
    const root = document.documentElement;
    root.setAttribute('data-theme', s.theme);
    root.style.setProperty('--accent', s.accentColor);
}