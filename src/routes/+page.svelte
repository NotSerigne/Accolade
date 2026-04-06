<script>
    import {invoke} from "@tauri-apps/api/core";
    import {onMount} from "svelte";
    import {listen} from "@tauri-apps/api/event";
    import AchievementNotif from '$lib/AchievementNotif.svelte';

    let achievements = $state([]);
    onMount(() => {
        loadAchievements();
        listen("achievement-unlocked", (e) => {
            achievements = achievements.concat(e.payload);
        })
    });
    async function loadAchievements() {
        const game = {
            name: "Elden Ring",
            steam_id: 1245620,
            game_icon: "icon.png",
            achievements_total: 42,
            achievements: [],
            path_buf: "C:/Users/Serigne/AppData/Roaming/Goldberg",
            emulator: "Goldberg"
        };
        achievements = await invoke("get_achievements", {game: game});
    }

</script>

<h1>Welcome to SvelteKit</h1>
{#each achievements as achievement}
    <AchievementNotif achievement={achievement} />
{/each}
