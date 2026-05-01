// src/lib/stores/user.ts
import { writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { settings } from './settings';
import { get } from 'svelte/store';

export interface SteamUser {
    steamid: string;
    personaname: string;
    avatarfull: string;
}

export const steamUser = writable<SteamUser | null>(null);

export async function refreshSteamUser() {
    const s = get(settings);
    if (!s.steamId || !s.steamApiKey) {
        steamUser.set(null);
        return;
    }

    try {
        const user = await invoke<SteamUser | null>('get_steam_user', {
            apiKey: s.steamApiKey,
            steamId: s.steamId
        });
        steamUser.set(user);
    } catch (err) {
        console.error('Failed to fetch steam user:', err);
        steamUser.set(null);
    }
}