import { writable } from 'svelte/store';

export interface GameFilters {
	status: 'all' | 'todo' | 'in-progress' | 'completed';
	favorite: 'all' | 'yes' | 'no';
	collection: string; // 'all' or tag name
	sortBy: 'name' | 'date' | 'completion' | 'exe_name';
	sortOrder: 'asc' | 'desc';
	searchQuery: string;
	displayMode: 'grid' | 'list' | 'timeline';
}

const DEFAULT_FILTERS: GameFilters = {
	status: 'all',
	favorite: 'all',
	collection: 'all',
	sortBy: 'name',
	sortOrder: 'asc',
	searchQuery: '',
	displayMode: 'grid'
};

function createPersistentFilters() {
	const storageKey = 'accolade_game_filters';
	let initialValue = DEFAULT_FILTERS;

	if (typeof localStorage !== 'undefined') {
		const saved = localStorage.getItem(storageKey);
		if (saved) {
			try {
				initialValue = { ...DEFAULT_FILTERS, ...JSON.parse(saved) };
			} catch (e) {
				console.error('Failed to parse saved filters', e);
			}
		}
	}

	const { subscribe, set, update } = writable<GameFilters>(initialValue);

	return {
		subscribe,
		set: (value: GameFilters) => {
			if (typeof localStorage !== 'undefined') {
				localStorage.setItem(storageKey, JSON.stringify(value));
			}
			set(value);
		},
		update: (fn: (v: GameFilters) => GameFilters) => {
			update((v) => {
				const next = fn(v);
				if (typeof localStorage !== 'undefined') {
					localStorage.setItem(storageKey, JSON.stringify(next));
				}
				return next;
			});
		},
		reset: () => {
			if (typeof localStorage !== 'undefined') {
				localStorage.removeItem(storageKey);
			}
			set(DEFAULT_FILTERS);
		}
	};
}

export const gameFilters = createPersistentFilters();
