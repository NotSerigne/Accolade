<script lang="ts">
	import type { Game } from '$lib/stores/Games.js';
	import type { SteamUser } from '$lib/stores/user.js';
	import { calculateAdvancedStats } from '$lib/stores/selectedGame.js';

	interface ProfileData {
		user?: SteamUser;
		games: Game[];
		stats?: {
			rarestAchievement: { name: string; pct: string } | null;
			maxAchievementsInDay: number;
			maxAchievementsInWeek: number;
			totalRemaining: number;
			totalUnlocked: number;
		};
	}

	let profile1 = $state<ProfileData | null>(null);
	let profile2 = $state<ProfileData | null>(null);

	async function parseFile(file: File): Promise<ProfileData | null> {
		if (!file.name.toLowerCase().endsWith('.json')) return null;
		return new Promise((resolve) => {
			const reader = new FileReader();
			reader.onload = (e) => {
				try {
					const data = JSON.parse(e.target?.result as string);
					let profile: ProfileData;

					if (Array.isArray(data)) {
						profile = { games: data };
					} else if (data.games) {
						profile = data;
					} else {
						return resolve(null);
					}

					// Calculer les stats si elles manquent
					if (!profile.stats) {
						profile.stats = calculateAdvancedStats(profile.games);
					}

					resolve(profile);
				} catch {
					resolve(null);
				}
			};
			reader.readAsText(file);
		});
	}

	async function handleFiles(files: FileList | File[]) {
		const validFiles = Array.from(files).filter((f) => f.name.toLowerCase().endsWith('.json'));
		if (validFiles.length === 0) return;

		for (const file of validFiles) {
			const data = await parseFile(file);
			if (data) {
				if (!profile1) profile1 = data;
				else if (!profile2) profile2 = data;
				else {
					profile1 = profile2;
					profile2 = data;
				}
			}
		}
	}

	const statsSummary = $derived.by(() => {
		if (!profile1 || !profile2) return null;

		const g1 = profile1.games;
		const g2 = profile2.games;

		const ids2 = new Set(g2.map((g) => g.id));
		const common = g1.filter((g) => ids2.has(g.id));

		const getCompPct = (games: Game[]) => {
			const total = games.reduce((acc, g) => acc + (g.achievements_total || 0), 0);
			const unlocked = games.reduce(
				(acc, g) => acc + (g.achievements?.filter((a) => a.unlocked).length || 0),
				0
			);
			return total > 0 ? Math.round((unlocked / total) * 100) : 0;
		};

		return {
			comp1: getCompPct(g1),
			comp2: getCompPct(g2),
			commonCount: common.length
		};
	});

	function getGameStats(p: ProfileData, gameId: string) {
		const g = p.games.find((x) => x.id === gameId);
		if (!g) return { pct: 0, unlocked: 0, total: 0 };
		const unlocked = g.achievements?.filter((a) => a.unlocked).length || 0;
		const total = g.achievements_total || g.achievements?.length || 0;
		return {
			pct: Math.round((unlocked / (total || 1)) * 100),
			unlocked,
			total
		};
	}

	const commonGames = $derived(
		profile1 && profile2
			? profile1.games.filter((g) => profile2!.games.some((x) => x.id === g.id))
			: []
	);

	function reset() {
		profile1 = null;
		profile2 = null;
	}
</script>

<div class="compare-page">
	<header class="page-header">
		<h1>Comparer les Profils</h1>
		<p class="subtitle">Importez vos fichiers JSON pour comparer les bibliothèques</p>

		<div class="header-actions">
			<label class="upload-btn">
				<svg
					width="18"
					height="18"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
					><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" /><polyline
						points="17 8 12 3 7 8"
					/><line x1="12" y1="3" x2="12" y2="15" /></svg
				>
				Sélectionner des profils (.json)
				<input
					type="file"
					accept=".json"
					multiple
					onchange={(e) => {
						const input = e.target as HTMLInputElement;
						if (input.files) handleFiles(input.files);
					}}
				/>
			</label>
		</div>
	</header>

	<div class="profiles-overview">
		<div class="profile-slot" class:active={profile1}>
			{#if profile1}
				<div class="user-info-card">
					<img
						src={profile1.user?.avatarfull ||
							'https://avatars.steamstatic.com/fef49e7fa7e1997310d705b2a6158ff8dc1cdfeb_full.jpg'}
						alt="Avatar"
						class="user-avatar"
					/>
					<div class="user-meta">
						<span class="user-name">{profile1.user?.personaname || 'Profil 1'}</span>
						<span class="user-games">{profile1.games.length} jeux</span>
					</div>
				</div>
			{:else}
				<div class="placeholder-card">
					<div class="icon">📁</div>
					<span>Profil 1 non chargé</span>
				</div>
			{/if}
		</div>

		<div class="vs-badge">VS</div>

		<div class="profile-slot" class:active={profile2}>
			{#if profile2}
				<div class="user-info-card">
					<img
						src={profile2.user?.avatarfull ||
							'https://avatars.steamstatic.com/fef49e7fa7e1997310d705b2a6158ff8dc1cdfeb_full.jpg'}
						alt="Avatar"
						class="user-avatar"
					/>
					<div class="user-meta">
						<span class="user-name">{profile2.user?.personaname || 'Profil 2'}</span>
						<span class="user-games">{profile2.games.length} jeux</span>
					</div>
				</div>
			{:else}
				<div class="placeholder-card">
					<div class="icon">📁</div>
					<span>Profil 2 non chargé</span>
				</div>
			{/if}
		</div>
	</div>

	{#if profile1 && profile2}
		{@const s1 = profile1.stats}
		{@const s2 = profile2.stats}

		<div class="stats-ribbon">
			<div class="ribbon-item">
				<div class="val">{statsSummary?.comp1}% vs {statsSummary?.comp2}%</div>
				<div class="lbl">Complétion Moyenne</div>
			</div>
			<div class="ribbon-item">
				<div class="val">{statsSummary?.commonCount}</div>
				<div class="lbl">Jeux Communs</div>
			</div>
			<button class="reset-btn" onclick={reset}>Réinitialiser</button>
		</div>

		<section class="fun-stats">
			<h2>Duel Statistique</h2>
			<div class="fun-stats-grid">
				<div class="fun-card">
					<div class="fun-label">Total succès débloqués</div>
					<div class="fun-values">
						<span class="v1">{s1?.totalUnlocked ?? '?'}</span>
						<span class="v-vs">vs</span>
						<span class="v2">{s2?.totalUnlocked ?? '?'}</span>
					</div>
				</div>
				<div class="fun-card">
					<div class="fun-label">Record en un jour</div>
					<div class="fun-values">
						<span class="v1">{s1?.maxAchievementsInDay ?? '?'}</span>
						<span class="v-vs">vs</span>
						<span class="v2">{s2?.maxAchievementsInDay ?? '?'}</span>
					</div>
				</div>
				<div class="fun-card">
					<div class="fun-label">Record en une semaine</div>
					<div class="fun-values">
						<span class="v1">{s1?.maxAchievementsInWeek ?? '?'}</span>
						<span class="v-vs">vs</span>
						<span class="v2">{s2?.maxAchievementsInWeek ?? '?'}</span>
					</div>
				</div>
				<div class="fun-card">
					<div class="fun-label">Succès restants</div>
					<div class="fun-values">
						<span class="v1">{s1?.totalRemaining ?? '?'}</span>
						<span class="v-vs">vs</span>
						<span class="v2">{s2?.totalRemaining ?? '?'}</span>
					</div>
				</div>
				<div class="fun-card rarest">
					<div class="fun-label">Le plus rare débloqué</div>
					<div class="fun-desc">
						<div class="d1">
							{s1?.rarestAchievement
								? `${s1.rarestAchievement.name} (${s1.rarestAchievement.pct}%)`
								: 'N/A'}
						</div>
						<div class="d-vs">contre</div>
						<div class="d2">
							{s2?.rarestAchievement
								? `${s2.rarestAchievement.name} (${s2.rarestAchievement.pct}%)`
								: 'N/A'}
						</div>
					</div>
				</div>
			</div>
		</section>

		{#if commonGames.length > 0}
			<section class="comparison-details">
				<h2>Comparaison des jeux communs</h2>
				<div class="compare-grid">
					{#each commonGames as game (game.id)}
						{@const g1 = getGameStats(profile1!, game.id)}
						{@const g2 = getGameStats(profile2!, game.id)}
						<div class="compare-card">
							<div class="game-header-mini">
								<img src={game.game_icon} alt="" class="mini-icon" />
								<span class="mini-title">{game.name}</span>
							</div>
							<div class="compare-bars">
								<div class="bar-row">
									<div class="bar-fill-track">
										<div class="bar-fill" style:width="{g1.pct}%"></div>
									</div>
									<div class="bar-info">
										<span class="bar-count">{g1.unlocked}/{g1.total}</span>
										<span class="bar-pct">{g1.pct}%</span>
									</div>
								</div>
								<div class="bar-row">
									<div class="bar-fill-track">
										<div class="bar-fill alt" style:width="{g2.pct}%"></div>
									</div>
									<div class="bar-info">
										<span class="bar-count">{g2.unlocked}/{g2.total}</span>
										<span class="bar-pct">{g2.pct}%</span>
									</div>
								</div>
							</div>
						</div>
					{/each}
				</div>
			</section>
		{:else}
			<div class="empty-state">
				<p>Aucun jeu en commun détecté entre ces deux profils.</p>
			</div>
		{/if}
	{:else}
		<div class="empty-state">
			<svg
				width="64"
				height="64"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="1.5"
				><circle cx="12" cy="12" r="10" /><path
					d="M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20"
				/><path d="M2 12h20" /></svg
			>
			<p>Veuillez importer deux fichiers d'export pour commencer la comparaison.</p>
		</div>
	{/if}
</div>

<style>
	.compare-page {
		height: 100%;
		display: flex;
		flex-direction: column;
		padding: 40px;
		overflow-y: auto;
		color: var(--text-primary);
		transition: background 0.3s;
		position: relative;
	}

	/* Subtile Floating Scrollbar */
	.compare-page::-webkit-scrollbar {
		width: 14px;
	}

	.compare-page::-webkit-scrollbar-track {
		background: transparent;
	}

	.compare-page::-webkit-scrollbar-thumb {
		background: rgba(255, 255, 255, 0.1);
		border: 4px solid transparent;
		background-clip: padding-box;
		border-radius: 10px;
		transition: background 0.2s;
	}

	.compare-page::-webkit-scrollbar-thumb:hover {
		background: rgba(255, 255, 255, 0.2);
	}

	.page-header {
		text-align: center;
		margin-bottom: 50px;
	}
	h1 {
		font-size: 2.2rem;
		margin-bottom: 8px;
	}
	.subtitle {
		color: var(--text-secondary);
		opacity: 0.8;
		margin-bottom: 24px;
	}

	.header-actions {
		display: flex;
		justify-content: center;
		margin-top: 10px;
	}

	.upload-btn {
		display: flex;
		align-items: center;
		gap: 10px;
		background: var(--accent);
		color: var(--accent-text);
		padding: 12px 24px;
		border-radius: 12px;
		font-weight: 700;
		cursor: pointer;
		transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
		box-shadow: 0 4px 15px color-mix(in srgb, var(--accent) 30%, transparent);
	}

	.upload-btn:hover {
		transform: translateY(-2px);
		filter: brightness(1.1);
		box-shadow: 0 6px 20px color-mix(in srgb, var(--accent) 40%, transparent);
	}

	.upload-btn input {
		display: none;
	}

	.profiles-overview {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 40px;
		margin-bottom: 50px;
	}

	.profile-slot {
		width: 340px;
		height: 100px;
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		border-radius: 16px;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: all 0.3s;
	}

	.profile-slot.active {
		border-color: var(--accent);
		background: var(--surface-2);
	}

	.user-info-card {
		display: flex;
		align-items: center;
		gap: 20px;
		width: 100%;
		padding: 0 20px;
	}

	.user-avatar {
		width: 56px;
		height: 56px;
		border-radius: 12px;
		border: 2px solid var(--accent);
	}
	.user-meta {
		display: flex;
		flex-direction: column;
	}
	.user-name {
		font-weight: 700;
		font-size: 1.1rem;
	}
	.user-games {
		font-size: 0.85rem;
		color: var(--text-secondary);
	}

	.placeholder-card {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 8px;
		opacity: 0.4;
	}
	.placeholder-card .icon {
		font-size: 1.5rem;
	}

	.vs-badge {
		font-size: 1.5rem;
		font-weight: 900;
		color: var(--accent);
		opacity: 0.3;
	}

	.stats-ribbon {
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		border-radius: 12px;
		padding: 20px 40px;
		display: flex;
		align-items: center;
		justify-content: space-around;
		margin-bottom: 50px;
	}

	.ribbon-item {
		text-align: center;
	}
	.ribbon-item .val {
		font-size: 1.4rem;
		font-weight: 800;
		color: var(--accent);
	}
	.ribbon-item .lbl {
		font-size: 0.75rem;
		text-transform: uppercase;
		letter-spacing: 1px;
		color: var(--text-secondary);
	}

	.reset-btn {
		padding: 8px 16px;
		border-radius: 8px;
		border: 1px solid var(--border-soft);
		background: transparent;
		color: var(--text-secondary);
		cursor: pointer;
		font-size: 0.9rem;
	}
	.reset-btn:hover {
		background: var(--surface-hover);
		color: var(--text-primary);
	}

	.fun-stats {
		margin-bottom: 50px;
	}
	.fun-stats h2 {
		margin-bottom: 20px;
		font-size: 1.3rem;
	}

	.fun-stats-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
		gap: 20px;
	}

	.fun-card {
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		border-radius: 14px;
		padding: 20px;
		text-align: center;
		display: flex;
		flex-direction: column;
		justify-content: center;
	}

	.fun-label {
		font-size: 0.75rem;
		text-transform: uppercase;
		color: var(--text-secondary);
		margin-bottom: 15px;
		letter-spacing: 0.5px;
	}
	.fun-values {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 20px;
		font-size: 2rem;
		font-weight: 800;
	}
	.v-vs {
		font-size: 1rem;
		opacity: 0.3;
	}
	.v1 {
		color: var(--accent);
	}
	.v2 {
		color: color-mix(in srgb, var(--accent) 60%, white);
	}

	.fun-desc {
		display: flex;
		flex-direction: column;
		gap: 5px;
		font-size: 0.9rem;
	}
	.d1 {
		color: var(--accent);
		font-weight: 700;
	}
	.d2 {
		color: color-mix(in srgb, var(--accent) 60%, white);
		font-weight: 700;
	}
	.d-vs {
		font-size: 0.7rem;
		opacity: 0.4;
		text-transform: uppercase;
	}

	.comparison-details h2 {
		margin-bottom: 25px;
		font-size: 1.3rem;
	}

	.compare-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(350px, 1fr));
		gap: 20px;
		padding-bottom: 40px;
	}

	.compare-card {
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		border-radius: 14px;
		padding: 16px;
	}

	.game-header-mini {
		display: flex;
		align-items: center;
		gap: 12px;
		margin-bottom: 16px;
	}
	.mini-icon {
		width: 32px;
		height: 32px;
		border-radius: 6px;
	}
	.mini-title {
		font-weight: 600;
		font-size: 0.95rem;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.compare-bars {
		display: flex;
		flex-direction: column;
		gap: 10px;
	}
	.bar-row {
		display: flex;
		align-items: center;
		gap: 12px;
	}
	.bar-fill-track {
		flex: 1;
		height: 6px;
		background: var(--surface-2);
		border-radius: 3px;
		overflow: hidden;
	}
	.bar-fill {
		height: 100%;
		background: var(--accent);
		border-radius: 3px;
	}
	.bar-fill.alt {
		background: color-mix(in srgb, var(--accent) 60%, white);
	}
	.bar-info {
		display: flex;
		align-items: center;
		gap: 10px;
		min-width: 85px;
		justify-content: flex-end;
	}
	.bar-count {
		font-size: 0.7rem;
		color: var(--text-muted);
		font-family: monospace;
	}
	.bar-pct {
		font-size: 0.75rem;
		font-family: monospace;
		font-weight: 700;
		width: 35px;
		text-align: right;
	}

	.empty-state {
		text-align: center;
		padding: 60px 0;
		color: var(--text-muted);
		opacity: 0.8;
	}
	.empty-state svg {
		margin-bottom: 16px;
		opacity: 0.2;
	}
</style>
