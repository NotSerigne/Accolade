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
        const games = await invoke("get_all_games");
        console.log("Games found:", games);  // 👈 Vérifie si des jeux sont trouvés

        for (let game of games) {
            const gameAchievements = await invoke("get_achievements", {game});
            console.log(`Achievements for ${game.name}:`, gameAchievements);  // 👈 Vérifie les achievements
            achievements = achievements.concat(gameAchievements);
        }
    }

</script>

<h1>Welcome to SvelteKit</h1>
{#each achievements as achievement}
    <AchievementNotif achievement={achievement} />
{/each}
