import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import {
	ACHIEVEMENT_TAG_KEYS,
	normalizeAchievementTagKey,
	normalizeAchievementTags,
	computeRemainingCompletionistHours,
	formatRemainingTimeHours,
	RARITY_TIERS_7,
	getRarityTier7,
	getRarityTierIndex,
	getRarityCssVar,
	parseCssRgb,
	getContrastTextForBg,
	calibrateAccentForTheme
} from './achievementHelpers.js';

function relativeLuminance(rgb: [number, number, number]): number {
	const channels = rgb.map((channel) => {
		const srgb = channel / 255;
		return srgb <= 0.03928 ? srgb / 12.92 : Math.pow((srgb + 0.055) / 1.055, 2.4);
	});
	return 0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2];
}

function contrastRatio(hexA: string, hexB: string): number {
	const rgbA = parseCssRgb(hexA);
	const rgbB = parseCssRgb(hexB);
	assert.ok(rgbA && rgbB);
	const lA = relativeLuminance(rgbA);
	const lB = relativeLuminance(rgbB);
	return (Math.max(lA, lB) + 0.05) / (Math.min(lA, lB) + 0.05);
}

describe('achievementHelpers', () => {
	it('exposes the canonical achievement tag keys', () => {
		assert.deepEqual(ACHIEVEMENT_TAG_KEYS, ['missable', 'multi', 'ngplus', 'buggy']);
	});

	it('normalizes FR and EN marker tag aliases', () => {
		assert.equal(normalizeAchievementTagKey('manquable'), 'missable');
		assert.equal(normalizeAchievementTagKey('  Missable '), 'missable');
		assert.equal(normalizeAchievementTagKey('multi'), 'multi');
		assert.equal(normalizeAchievementTagKey('Multiplayer'), 'multi');
		assert.equal(normalizeAchievementTagKey('COOP'), 'multi');
		assert.equal(normalizeAchievementTagKey('ng+'), 'ngplus');
		assert.equal(normalizeAchievementTagKey('ngplus'), 'ngplus');
		assert.equal(normalizeAchievementTagKey('New Game+'), 'ngplus');
		assert.equal(normalizeAchievementTagKey('newgame+'), 'ngplus');
		assert.equal(normalizeAchievementTagKey('buggué'), 'buggy');
		assert.equal(normalizeAchievementTagKey('buggue'), 'buggy');
		assert.equal(normalizeAchievementTagKey('BUGGY'), 'buggy');
		assert.equal(normalizeAchievementTagKey('Glitched'), 'buggy');

		assert.equal(normalizeAchievementTagKey(''), null);
		assert.equal(normalizeAchievementTagKey('   '), null);
		assert.equal(normalizeAchievementTagKey('story'), null);
	});

	it('deduplicates and preserves order in normalizeAchievementTags', () => {
		assert.deepEqual(normalizeAchievementTags(null), []);
		assert.deepEqual(normalizeAchievementTags(undefined), []);
		assert.deepEqual(
			normalizeAchievementTags(['  MANQUABLE ', 'coop', 'missable', 'unknown', 'NG+', 'glitched']),
			['missable', 'multi', 'ngplus', 'buggy']
		);
	});

	it('computes remaining completionist hours safely and clamps at 0', () => {
		assert.equal(computeRemainingCompletionistHours(10, 7200), 8);
		assert.equal(computeRemainingCompletionistHours(5, 36000), 0);
		assert.equal(computeRemainingCompletionistHours(4.5, -120), 4.5);
		assert.equal(computeRemainingCompletionistHours(null, 3600), null);
		assert.equal(computeRemainingCompletionistHours(undefined, 3600), null);
		assert.equal(computeRemainingCompletionistHours(0, 3600), null);
		assert.equal(computeRemainingCompletionistHours(-2, 3600), null);
		assert.equal(computeRemainingCompletionistHours(Number.NaN, 3600), null);
	});

	it('formats remaining hours concisely', () => {
		assert.equal(formatRemainingTimeHours(0), '0h');
		assert.equal(formatRemainingTimeHours(-0.5), '0h');
		assert.equal(formatRemainingTimeHours(Number.NaN), '0h');
		assert.equal(formatRemainingTimeHours(0.25), '15min');
		assert.equal(formatRemainingTimeHours(0.75), '45min');
		assert.equal(formatRemainingTimeHours(0.999), '1h');
		assert.equal(formatRemainingTimeHours(3), '3h');
		assert.equal(formatRemainingTimeHours(2.0834), '2h05');
		assert.equal(formatRemainingTimeHours(2.25), '2h15');
	});

	it('classifies 7-tier rarities and maps indices and CSS vars', () => {
		assert.deepEqual(RARITY_TIERS_7, [
			'mythic',
			'legendary',
			'epic',
			'very-rare',
			'rare',
			'uncommon',
			'common'
		]);

		assert.equal(getRarityTier7(0), 'mythic');
		assert.equal(getRarityTier7(0.1), 'mythic');
		assert.equal(getRarityTier7('0.08%'), 'mythic');
		assert.equal(getRarityTier7('0,1'), 'mythic');
		assert.equal(getRarityTier7(0.11), 'legendary');
		assert.equal(getRarityTier7(1), 'legendary');
		assert.equal(getRarityTier7(1.01), 'epic');
		assert.equal(getRarityTier7(3), 'epic');
		assert.equal(getRarityTier7(3.1), 'very-rare');
		assert.equal(getRarityTier7(7), 'very-rare');
		assert.equal(getRarityTier7(7.1), 'rare');
		assert.equal(getRarityTier7(15), 'rare');
		assert.equal(getRarityTier7(15.1), 'uncommon');
		assert.equal(getRarityTier7(35), 'uncommon');
		assert.equal(getRarityTier7(35.1), 'common');
		assert.equal(getRarityTier7('100'), 'common');

		assert.equal(getRarityTier7(null), 'common');
		assert.equal(getRarityTier7(undefined), 'common');
		assert.equal(getRarityTier7(''), 'common');
		assert.equal(getRarityTier7('   '), 'common');
		assert.equal(getRarityTier7('not-a-number'), 'common');
		assert.equal(getRarityTier7(Number.NaN), 'common');
		assert.equal(getRarityTier7(Number.POSITIVE_INFINITY), 'common');
		assert.equal(getRarityTier7(-5), 'common');

		RARITY_TIERS_7.forEach((tier, index) => {
			assert.equal(getRarityTierIndex(tier), index);
			assert.equal(getRarityCssVar(tier), `var(--rarity-${tier})`);
		});
	});

	it('parses CSS hex and rgb/rgba strings cleanly', () => {
		assert.deepEqual(parseCssRgb('#abc'), [170, 187, 204]);
		assert.deepEqual(parseCssRgb(' #C8A96E '), [200, 169, 110]);
		assert.deepEqual(parseCssRgb('rgb(12, 34, 56)'), [12, 34, 56]);
		assert.deepEqual(parseCssRgb('rgba(300, -15, 127.6, 0.4)'), [255, 0, 128]);
		assert.deepEqual(parseCssRgb('rgb(10 20 30 / 50%)'), [10, 20, 30]);

		assert.equal(parseCssRgb(''), null);
		assert.equal(parseCssRgb('#12'), null);
		assert.equal(parseCssRgb('#gggggg'), null);
		assert.equal(parseCssRgb('rgb(1, 2)'), null);
		assert.equal(parseCssRgb('hsl(120, 50%, 50%)'), null);
		assert.equal(parseCssRgb('not-a-color'), null);
	});

	it('picks higher-contrast foreground text for backgrounds', () => {
		assert.equal(getContrastTextForBg('#ffffff'), '#111111');
		assert.equal(getContrastTextForBg('#c8a96e'), '#111111');
		assert.equal(getContrastTextForBg('rgb(240, 230, 140)'), '#111111');
		assert.equal(getContrastTextForBg('#000000'), '#ffffff');
		assert.equal(getContrastTextForBg('#1a1b20'), '#ffffff');
		assert.equal(getContrastTextForBg('#785612'), '#ffffff');
		assert.equal(getContrastTextForBg('invalid'), '#ffffff');
	});

	it('calibrates accent colors to WCAG AA in both dark and light themes', () => {
		assert.deepEqual(calibrateAccentForTheme('invalid', 'dark'), {
			accent: '#c8a96e',
			text: '#111111'
		});
		assert.deepEqual(calibrateAccentForTheme('invalid', 'light'), {
			accent: '#785612',
			text: '#ffffff'
		});

		// Compliant inputs preserve exact values
		assert.deepEqual(calibrateAccentForTheme('#c8a96e', 'dark'), {
			accent: '#c8a96e',
			text: '#111111'
		});
		assert.deepEqual(calibrateAccentForTheme('#785612', 'light'), {
			accent: '#785612',
			text: '#ffffff'
		});

		const samples = [
			'#c8a96e',
			'#785612',
			'#787878', // mid-luminance dead zone (~0.188)
			'#7c7c7c',
			'#1122aa',
			'#ffee00',
			'#ff0000',
			'rgb(40, 120, 180)',
			'rgba(250, 220, 180, 0.9)',
			'#000000',
			'#ffffff'
		];

		for (const sample of samples) {
			const darkRes = calibrateAccentForTheme(sample, 'dark');
			assert.match(darkRes.accent, /^#[0-9a-f]{6}$/);
			assert.equal(darkRes.text, getContrastTextForBg(darkRes.accent));
			assert.ok(
				contrastRatio(darkRes.accent, '#1a1b20') >= 4.6,
				`dark surface contrast failed for ${sample} -> ${darkRes.accent}`
			);
			assert.ok(
				contrastRatio(darkRes.accent, darkRes.text) >= 4.6,
				`dark text contrast failed for ${sample} -> ${darkRes.accent}`
			);

			const lightRes = calibrateAccentForTheme(sample, 'light');
			assert.match(lightRes.accent, /^#[0-9a-f]{6}$/);
			assert.equal(lightRes.text, getContrastTextForBg(lightRes.accent));
			assert.ok(
				contrastRatio(lightRes.accent, '#edf1f6') >= 4.6,
				`light surface contrast failed for ${sample} -> ${lightRes.accent}`
			);
			assert.ok(
				contrastRatio(lightRes.accent, lightRes.text) >= 4.6,
				`light text contrast failed for ${sample} -> ${lightRes.accent}`
			);
		}
	});
});
