import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import {
	ACHIEVEMENT_TAG_KEYS,
	normalizeAchievementTagKey,
	normalizeAchievementTags,
	computeRemainingCompletionistHours,
	formatRemainingTimeHours
} from './achievementHelpers.js';

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
});
