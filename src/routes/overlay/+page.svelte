<script lang="ts">
    import { onMount } from 'svelte';
    import { listen } from '@tauri-apps/api/event';
    import { getCurrentWindow, currentMonitor } from '@tauri-apps/api/window';
    import { LogicalPosition } from '@tauri-apps/api/dpi';
    import { Store } from '@tauri-apps/plugin-store';
    import AchievementNotif from '$lib/AchievementNotif.svelte';

    type WindowPosition = 'top-left' | 'top-center' | 'top-right' | 'bottom-left' | 'bottom-center' | 'bottom-right';

    interface Achievement {
        name?: string;
        desc?: string;
        icon?: string;
        rarity?: string;
        completionpercentage?: number;
        unlocked?: number;
        total?: number;
        is_platinum?: boolean;
    }

    const NOTIF_WIDTH  = 460;
    const NOTIF_HEIGHT = 260;
    const MARGIN       = 24;
    const DEFAULT_POSITION: WindowPosition = 'top-right';
    const DEFAULT_SOUND = 'Steam.mp3';

    let settingsStorePromise: Promise<Store> | null = null;

    async function getSettingsStore(): Promise<Store> {
        if (!settingsStorePromise) {
            settingsStorePromise = Store.load('settings.json');
        }
        return settingsStorePromise;
    }

    async function getOverlayPreferences(): Promise<{ windowPosition: WindowPosition; notificationSound: string }> {
        try {
            const store = await getSettingsStore();
            const [windowPosition, notificationSound] = await Promise.all([
                store.get<WindowPosition>('windowPosition'),
                store.get<string>('notificationSound'),
            ]);
            return {
                windowPosition: windowPosition ?? DEFAULT_POSITION,
                notificationSound: notificationSound ?? DEFAULT_SOUND,
            };
        } catch {
            settingsStorePromise = null;
            return {
                windowPosition: DEFAULT_POSITION,
                notificationSound: DEFAULT_SOUND,
            };
        }
    }

    function playNotificationSound(filename: string): void {
        if (!filename || filename === 'none') return;
        const audio = new Audio(`/sounds/${encodeURIComponent(filename)}`);
        audio.volume = 0.7;
        audio.play().catch(() => {});
    }

    async function positionOverlay(pos: WindowPosition): Promise<WindowPosition> {
        const win = getCurrentWindow();
        const monitor = await currentMonitor();
        if (!monitor) return DEFAULT_POSITION;

        const sw = monitor.size.width  / monitor.scaleFactor;
        const sh = monitor.size.height / monitor.scaleFactor;
        const ox = monitor.position.x  / monitor.scaleFactor;
        const oy = monitor.position.y  / monitor.scaleFactor;

        let x: number;
        let y: number;

        switch (pos) {
            case 'top-left':
                x = ox + MARGIN;
                y = oy + MARGIN;
                break;
            case 'top-center':
                x = ox + (sw - NOTIF_WIDTH) / 2;
                y = oy + MARGIN;
                break;
            case 'top-right':
                x = ox + sw - NOTIF_WIDTH - MARGIN;
                y = oy + MARGIN;
                break;
            case 'bottom-left':
                x = ox + MARGIN;
                y = oy + sh - NOTIF_HEIGHT - MARGIN;
                break;
            case 'bottom-center':
                x = ox + (sw - NOTIF_WIDTH) / 2;
                y = oy + sh - NOTIF_HEIGHT - MARGIN;
                break;
            case 'bottom-right':
                x = ox + sw - NOTIF_WIDTH - MARGIN;
                y = oy + sh - NOTIF_HEIGHT - MARGIN;
                break;
            default:
                x = ox + sw - NOTIF_WIDTH - MARGIN;
                y = oy + MARGIN;
        }

        await win.setPosition(new LogicalPosition(Math.round(x), Math.round(y)));
        return pos;
    }

    let achievement = $state<Achievement | null>(null);
    let visible     = $state(false);
    let position    = $state<WindowPosition>('top-right');

    onMount(() => {
        const win = getCurrentWindow();
        let timer: ReturnType<typeof setTimeout> | null = null;
        let unlisten: (() => void) | undefined;

        listen('achievement-notif', async ({ payload }: any) => {
            if (timer) clearTimeout(timer);

            // Cacher + réinitialiser pendant qu'on repositionne
            visible     = false;
            achievement = null;

            const { windowPosition, notificationSound } = await getOverlayPreferences();
            const resolvedPosition = await positionOverlay(windowPosition);

            playNotificationSound(notificationSound);

            // Tout dans le même frame : position connue, élément créé, animation démarre
            requestAnimationFrame(() => {
                position    = resolvedPosition;
                achievement = payload as Achievement;
                visible     = true;
                win.show();

                timer = setTimeout(() => {
                    visible = false;
                    setTimeout(() => win.hide(), 600);
                }, 5000);
            });
        }).then((cleanup) => {
            unlisten = cleanup;
        });

        return () => {
            if (timer) clearTimeout(timer);
            unlisten?.();
        };
    });
</script>

<AchievementNotif {achievement} {visible} {position} />
