<script lang="ts">
    import { onMount } from 'svelte';
    import { listen } from '@tauri-apps/api/event';
    import { getCurrentWindow } from '@tauri-apps/api/window';
    import AchievementNotif from '$lib/AchievementNotif.svelte';

    interface Achievement {
        name?: string;
        desc?: string;
        icon?: string;
        rarity?: string;
        completionpercentage?: number;
        unlocked?: number;
        total?: number;
    }

    let achievement = $state<Achievement | null>(null);
    let visible = $state(false);

    onMount(() => {
        console.log('[OVERLAY] onMount fired');
        const win = getCurrentWindow();
        let timer: ReturnType<typeof setTimeout> | null = null;

        listen('achievement-notif', ({ payload }: any) => {
            console.log('[OVERLAY] event reçu', payload);
            if (timer) clearTimeout(timer);
            achievement = payload as Achievement;
            visible = false;

            requestAnimationFrame(() => {
                visible = true;
                win.show();

                timer = setTimeout(() => {
                    visible = false;
                    setTimeout(() => win.hide(), 600);
                }, 5000);
            });
        });
    });
</script>

<AchievementNotif {achievement} {visible} />