<script>
    import { scale } from 'svelte/transition';
    let { achievement } = $props();
</script>

<div class="notification bg-gray-900 rounded-xl w-96" transition:scale>
    <div class="ColoredBody">
        <div class="text-center text-sm text-gray-500">Achievement Unlocked!</div>
    </div>
    <div class="Achievement">
        <div class="Left">
            <img src="{achievement.icon}" alt="Icon"/>
        </div>
        <div class="Right">
            <div class="AchTitle">{achievement.name}</div>
            <div class="AchDesc">{achievement.desc}</div>
            <div class="AchRarity">{achievement.rarity}</div>
            <div class="AchCompletionPercentage">{achievement.completionpercentage}</div>
        </div>
    </div>
</div>