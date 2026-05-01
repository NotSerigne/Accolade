import { derived } from 'svelte/store';
import { settings } from './settings';

export const languages = ['fr', 'en', 'es', 'de', 'it'] as const;
export type LanguageCode = (typeof languages)[number];

const localeByLanguage: Record<LanguageCode, string> = {
    fr: 'fr-FR',
    en: 'en-US',
    es: 'es-ES',
    de: 'de-DE',
    it: 'it-IT'
};

const translations = {
    fr: {
        'lang.fr': 'Français',
        'lang.en': 'Anglais',
        'lang.es': 'Espagnol',
        'lang.de': 'Allemand',
        'lang.it': 'Italien',

        'sidebar.home': 'Accueil',
        'sidebar.stats': 'Statistiques',
        'sidebar.objectives': 'Objectifs',
        'sidebar.journal': 'Journal',
        'sidebar.setup': 'Setup',
        'sidebar.emptyGames': 'Aucun\njeu',
        'sidebar.user.disconnected': 'Non connecté',
        'sidebar.user.configureSteam': 'Configurez votre Steam ID',
        'sidebar.user.achievements': '{count} succès · 0 platines',
        'sidebar.watcher.active': 'Watcher actif',
        'sidebar.watcher.inactive': 'Watcher inactif',
        'sidebar.advancedSettings': 'Paramètres avancés',

        'topbar.searchPlaceholder': 'Rechercher un jeu ou un succès',
        'topbar.searchSuggestions': 'Suggestions de recherche',
        'topbar.noSuggestion': 'Aucune suggestion',
        'topbar.kind.game': 'Jeu',
        'topbar.kind.achievement': 'Succès',
        'topbar.sync': 'Actualiser',
        'topbar.syncing': 'Synchronisation...',
        'topbar.syncTitle': 'Synchroniser avec Steam',
        'topbar.scan': '● Scan',
        'topbar.library': 'Bibliothèque',
        'topbar.gameSubtitle': 'Jeu · AppID {id}',

        'main.recentActivity': 'Activité récente',
        'main.noRecentActivity': 'Aucune activité récente',
        'main.recentLibrary': 'Bibliothèque récente',

        'setup.title': 'Setup',
        'setup.subtitle': 'Préférences de langue, notifications et apparence',
        'setup.steam': 'Steam',
        'setup.steamId': 'Steam ID (64 bits)',
        'setup.steamApiKey': 'Clé API Steam',
        'setup.sgdbApiKey': 'Clé API SteamGridDB',
        'setup.link.steamId': 'Ouvrir la page de votre compte Steam',
        'setup.link.steamApiKey': 'Ouvrir la page Steam Web API Key',
        'setup.link.sgdbApiKey': 'Ouvrir la page API SteamGridDB',
        'setup.language': 'Langue',
        'setup.notifications': 'Notifications',
        'setup.position': 'Position HUD',
        'setup.sound': 'Son de notification',
        'setup.test': 'Tester la notification',
        'setup.appearance': 'Apparence',
        'setup.theme': 'Thème',
        'setup.theme.dark': 'Sombre',
        'setup.theme.light': 'Clair',
        'setup.theme.system': 'Système',
        'setup.accent': 'Couleur accent',
        'setup.save': 'Enregistrer',
        'setup.saving': 'Enregistrement...',
        'setup.saved': 'Préférences enregistrées',
        'setup.errorSave': "Échec de l'enregistrement: {details}",
        'setup.errorTest': 'Erreur lors du test: {details}',
        'setup.position.top-left': 'Haut gauche',
        'setup.position.top-center': 'Haut centre',
        'setup.position.top-right': 'Haut droite',
        'setup.position.bottom-left': 'Bas gauche',
        'setup.position.bottom-center': 'Bas centre',
        'setup.position.bottom-right': 'Bas droite',
        'setup.sound.none': 'Aucun son',

        'journal.title': 'Journal',
        'journal.subtitle': 'Historique de vos succès débloqués',
        'journal.allGames': 'Tous les jeux',
        'journal.countAchievements': '{count} succès',
        'journal.completed': 'Jeu complété à 100% !',
        'journal.empty': 'Aucun événement trouvé.',

        'objectives.title': 'Objectifs',
        'objectives.subtitle': 'Jeux les plus proches du 100%',
        'objectives.sort.progression': 'Progression',
        'objectives.sort.remaining': 'Succès restants',
        'objectives.easy': 'Succès faciles',
        'objectives.easySubtitle': 'Succès les plus faciles restants',
        'objectives.remaining': '{unlocked} / {total} — {remaining} restants',
        'objectives.details': 'Voir les détails',

        'statsPanel.stats': 'Stats',
        'statsPanel.unlocked': 'Succès débloqués',
        'statsPanel.outOf': 'sur {total} total',
        'statsPanel.avgProgress': 'Progression moy.',
        'statsPanel.avgRarity': 'Rareté moyenne',
        'statsPanel.rarityBreakdown': 'Répartition Rareté',
        'statsPanel.recentCompleted': 'Complétés récents'
    },
    en: {
        'lang.fr': 'French',
        'lang.en': 'English',
        'lang.es': 'Spanish',
        'lang.de': 'German',
        'lang.it': 'Italian',
        'sidebar.home': 'Home',
        'sidebar.stats': 'Stats',
        'sidebar.objectives': 'Objectives',
        'sidebar.journal': 'Journal',
        'sidebar.setup': 'Setup',
        'sidebar.emptyGames': 'No\ngame',
        'sidebar.user.disconnected': 'Not connected',
        'sidebar.user.configureSteam': 'Configure your Steam ID',
        'sidebar.user.achievements': '{count} achievements · 0 platinums',
        'sidebar.watcher.active': 'Watcher active',
        'sidebar.watcher.inactive': 'Watcher inactive',
        'sidebar.advancedSettings': 'Advanced settings',
        'topbar.searchPlaceholder': 'Search a game or an achievement',
        'topbar.searchSuggestions': 'Search suggestions',
        'topbar.noSuggestion': 'No suggestion',
        'topbar.kind.game': 'Game',
        'topbar.kind.achievement': 'Achievement',
        'topbar.sync': 'Refresh',
        'topbar.syncing': 'Syncing...',
        'topbar.syncTitle': 'Sync with Steam',
        'topbar.scan': '● Scan',
        'topbar.library': 'Library',
        'topbar.gameSubtitle': 'Game · AppID {id}',
        'main.recentActivity': 'Recent activity',
        'main.noRecentActivity': 'No recent activity',
        'main.recentLibrary': 'Recent library',
        'setup.title': 'Setup',
        'setup.subtitle': 'Language, notification and appearance preferences',
        'setup.steam': 'Steam',
        'setup.steamId': 'Steam ID (64-bit)',
        'setup.steamApiKey': 'Steam API key',
        'setup.sgdbApiKey': 'SteamGridDB API key',
        'setup.link.steamId': 'Open your Steam account page',
        'setup.link.steamApiKey': 'Open Steam Web API Key page',
        'setup.link.sgdbApiKey': 'Open SteamGridDB API page',
        'setup.language': 'Language',
        'setup.notifications': 'Notifications',
        'setup.position': 'HUD position',
        'setup.sound': 'Notification sound',
        'setup.test': 'Test notification',
        'setup.appearance': 'Appearance',
        'setup.theme': 'Theme',
        'setup.theme.dark': 'Dark',
        'setup.theme.light': 'Light',
        'setup.theme.system': 'System',
        'setup.accent': 'Accent color',
        'setup.save': 'Save',
        'setup.saving': 'Saving...',
        'setup.saved': 'Preferences saved',
        'setup.errorSave': 'Failed to save: {details}',
        'setup.errorTest': 'Test failed: {details}',
        'setup.position.top-left': 'Top left',
        'setup.position.top-center': 'Top center',
        'setup.position.top-right': 'Top right',
        'setup.position.bottom-left': 'Bottom left',
        'setup.position.bottom-center': 'Bottom center',
        'setup.position.bottom-right': 'Bottom right',
        'setup.sound.none': 'No sound',
        'journal.title': 'Journal',
        'journal.subtitle': 'History of your unlocked achievements',
        'journal.allGames': 'All games',
        'journal.countAchievements': '{count} achievements',
        'journal.completed': 'Game completed at 100%!',
        'journal.empty': 'No event found.',
        'objectives.title': 'Objectives',
        'objectives.subtitle': 'Games closest to 100%',
        'objectives.sort.progression': 'Progression',
        'objectives.sort.remaining': 'Remaining achievements',
        'objectives.easy': 'Easy achievements',
        'objectives.easySubtitle': 'Easiest achievements left',
        'objectives.remaining': '{unlocked} / {total} — {remaining} left',
        'objectives.details': 'View details',
        'statsPanel.stats': 'Stats',
        'statsPanel.unlocked': 'Unlocked achievements',
        'statsPanel.outOf': 'out of {total} total',
        'statsPanel.avgProgress': 'Avg. progression',
        'statsPanel.avgRarity': 'Avg. rarity',
        'statsPanel.rarityBreakdown': 'Rarity breakdown',
        'statsPanel.recentCompleted': 'Recently completed'
    },
    es: {
        'lang.fr': 'Francés', 'lang.en': 'Inglés', 'lang.es': 'Español', 'lang.de': 'Alemán', 'lang.it': 'Italiano',
        'sidebar.home': 'Inicio', 'sidebar.stats': 'Estadísticas', 'sidebar.objectives': 'Objetivos', 'sidebar.journal': 'Diario', 'sidebar.setup': 'Setup',
        'sidebar.emptyGames': 'Sin\njuego', 'sidebar.user.disconnected': 'No conectado', 'sidebar.user.configureSteam': 'Configura tu Steam ID',
        'sidebar.user.achievements': '{count} logros · 0 platinos', 'sidebar.watcher.active': 'Watcher activo', 'sidebar.watcher.inactive': 'Watcher inactivo',
        'sidebar.advancedSettings': 'Ajustes avanzados', 'topbar.searchPlaceholder': 'Buscar un juego o un logro', 'topbar.searchSuggestions': 'Sugerencias de búsqueda',
        'topbar.noSuggestion': 'Sin sugerencias', 'topbar.kind.game': 'Juego', 'topbar.kind.achievement': 'Logro', 'topbar.sync': 'Actualizar',
        'topbar.syncing': 'Sincronizando...', 'topbar.syncTitle': 'Sincronizar con Steam', 'topbar.scan': '● Escaneo', 'topbar.library': 'Biblioteca',
        'topbar.gameSubtitle': 'Juego · AppID {id}', 'main.recentActivity': 'Actividad reciente', 'main.noRecentActivity': 'Sin actividad reciente',
        'main.recentLibrary': 'Biblioteca reciente', 'setup.title': 'Setup', 'setup.subtitle': 'Preferencias de idioma, notificaciones y apariencia',
        'setup.steam': 'Steam', 'setup.steamId': 'Steam ID (64 bits)', 'setup.steamApiKey': 'Clave API de Steam', 'setup.sgdbApiKey': 'Clave API de SteamGridDB',
        'setup.link.steamId': 'Abrir la página de tu cuenta de Steam', 'setup.link.steamApiKey': 'Abrir la página Steam Web API Key', 'setup.link.sgdbApiKey': 'Abrir la página API de SteamGridDB',
        'setup.language': 'Idioma', 'setup.notifications': 'Notificaciones', 'setup.position': 'Posición HUD', 'setup.sound': 'Sonido de notificación',
        'setup.test': 'Probar notificación', 'setup.appearance': 'Apariencia', 'setup.theme': 'Tema', 'setup.theme.dark': 'Oscuro', 'setup.theme.light': 'Claro',
        'setup.theme.system': 'Sistema',
        'setup.accent': 'Color de acento', 'setup.save': 'Guardar', 'setup.saving': 'Guardando...', 'setup.saved': 'Preferencias guardadas',
        'setup.errorSave': 'Error al guardar: {details}', 'setup.errorTest': 'Error de prueba: {details}',
        'setup.position.top-left': 'Arriba izquierda', 'setup.position.top-center': 'Arriba centro', 'setup.position.top-right': 'Arriba derecha',
        'setup.position.bottom-left': 'Abajo izquierda', 'setup.position.bottom-center': 'Abajo centro', 'setup.position.bottom-right': 'Abajo derecha',
        'setup.sound.none': 'Sin sonido', 'journal.title': 'Diario', 'journal.subtitle': 'Historial de logros desbloqueados', 'journal.allGames': 'Todos los juegos',
        'journal.countAchievements': '{count} logros', 'journal.completed': '¡Juego completado al 100%!', 'journal.empty': 'No se encontraron eventos.',
        'objectives.title': 'Objetivos', 'objectives.subtitle': 'Juegos más cerca del 100%', 'objectives.sort.progression': 'Progresión',
        'objectives.sort.remaining': 'Logros restantes', 'objectives.easy': 'Logros fáciles', 'objectives.easySubtitle': 'Logros más fáciles restantes',
        'objectives.remaining': '{unlocked} / {total} — {remaining} restantes', 'objectives.details': 'Ver detalles', 'statsPanel.stats': 'Stats',
        'statsPanel.unlocked': 'Logros desbloqueados', 'statsPanel.outOf': 'de {total} total', 'statsPanel.avgProgress': 'Progreso prom.',
        'statsPanel.avgRarity': 'Rareza prom.', 'statsPanel.rarityBreakdown': 'Distribución de rareza', 'statsPanel.recentCompleted': 'Completados recientes'
    },
    de: {
        'lang.fr': 'Französisch', 'lang.en': 'Englisch', 'lang.es': 'Spanisch', 'lang.de': 'Deutsch', 'lang.it': 'Italienisch',
        'sidebar.home': 'Start', 'sidebar.stats': 'Statistiken', 'sidebar.objectives': 'Ziele', 'sidebar.journal': 'Journal', 'sidebar.setup': 'Setup',
        'sidebar.emptyGames': 'Kein\nSpiel', 'sidebar.user.disconnected': 'Nicht verbunden', 'sidebar.user.configureSteam': 'Steam-ID konfigurieren',
        'sidebar.user.achievements': '{count} Erfolge · 0 Platin', 'sidebar.watcher.active': 'Watcher aktiv', 'sidebar.watcher.inactive': 'Watcher inaktiv',
        'sidebar.advancedSettings': 'Erweiterte Einstellungen', 'topbar.searchPlaceholder': 'Ein Spiel oder Erfolg suchen',
        'topbar.searchSuggestions': 'Suchvorschläge', 'topbar.noSuggestion': 'Keine Vorschläge', 'topbar.kind.game': 'Spiel',
        'topbar.kind.achievement': 'Erfolg', 'topbar.sync': 'Aktualisieren', 'topbar.syncing': 'Synchronisiere...', 'topbar.syncTitle': 'Mit Steam synchronisieren',
        'topbar.scan': '● Scan', 'topbar.library': 'Bibliothek', 'topbar.gameSubtitle': 'Spiel · AppID {id}', 'main.recentActivity': 'Letzte Aktivität',
        'main.noRecentActivity': 'Keine letzte Aktivität', 'main.recentLibrary': 'Letzte Bibliothek', 'setup.title': 'Setup',
        'setup.steam': 'Steam', 'setup.steamId': 'Steam-ID (64-Bit)', 'setup.steamApiKey': 'Steam API-Schlüssel', 'setup.sgdbApiKey': 'SteamGridDB API-Schlüssel',
        'setup.link.steamId': 'Steam-Kontoseite öffnen', 'setup.link.steamApiKey': 'Steam Web API Key-Seite öffnen', 'setup.link.sgdbApiKey': 'SteamGridDB API-Seite öffnen',
        'setup.subtitle': 'Sprache, Benachrichtigungen und Darstellung', 'setup.language': 'Sprache', 'setup.notifications': 'Benachrichtigungen',
        'setup.position': 'HUD-Position', 'setup.sound': 'Benachrichtigungston', 'setup.test': 'Benachrichtigung testen', 'setup.appearance': 'Darstellung',
        'setup.theme': 'Theme', 'setup.theme.dark': 'Dunkel', 'setup.theme.light': 'Hell', 'setup.accent': 'Akzentfarbe', 'setup.save': 'Speichern',
        'setup.theme.system': 'System',
        'setup.saving': 'Speichern...', 'setup.saved': 'Einstellungen gespeichert', 'setup.errorSave': 'Speichern fehlgeschlagen: {details}',
        'setup.errorTest': 'Test fehlgeschlagen: {details}', 'setup.position.top-left': 'Oben links', 'setup.position.top-center': 'Oben mitte',
        'setup.position.top-right': 'Oben rechts', 'setup.position.bottom-left': 'Unten links', 'setup.position.bottom-center': 'Unten mitte',
        'setup.position.bottom-right': 'Unten rechts', 'setup.sound.none': 'Kein Ton', 'journal.title': 'Journal',
        'journal.subtitle': 'Verlauf deiner freigeschalteten Erfolge', 'journal.allGames': 'Alle Spiele', 'journal.countAchievements': '{count} Erfolge',
        'journal.completed': 'Spiel zu 100% abgeschlossen!', 'journal.empty': 'Keine Ereignisse gefunden.', 'objectives.title': 'Ziele',
        'objectives.subtitle': 'Spiele am nächsten zu 100%', 'objectives.sort.progression': 'Fortschritt', 'objectives.sort.remaining': 'Verbleibende Erfolge',
        'objectives.easy': 'Einfache Erfolge', 'objectives.easySubtitle': 'Einfachste verbleibende Erfolge',
        'objectives.remaining': '{unlocked} / {total} — {remaining} übrig', 'objectives.details': 'Details anzeigen', 'statsPanel.stats': 'Stats',
        'statsPanel.unlocked': 'Freigeschaltete Erfolge', 'statsPanel.outOf': 'von {total} gesamt', 'statsPanel.avgProgress': 'Ø Fortschritt',
        'statsPanel.avgRarity': 'Ø Seltenheit', 'statsPanel.rarityBreakdown': 'Seltenheitsverteilung', 'statsPanel.recentCompleted': 'Zuletzt abgeschlossen'
    },
    it: {
        'lang.fr': 'Francese', 'lang.en': 'Inglese', 'lang.es': 'Spagnolo', 'lang.de': 'Tedesco', 'lang.it': 'Italiano',
        'sidebar.home': 'Home', 'sidebar.stats': 'Statistiche', 'sidebar.objectives': 'Obiettivi', 'sidebar.journal': 'Diario', 'sidebar.setup': 'Setup',
        'sidebar.emptyGames': 'Nessun\ngioco', 'sidebar.user.disconnected': 'Non connesso', 'sidebar.user.configureSteam': 'Configura il tuo Steam ID',
        'sidebar.user.achievements': '{count} obiettivi · 0 platini', 'sidebar.watcher.active': 'Watcher attivo', 'sidebar.watcher.inactive': 'Watcher inattivo',
        'sidebar.advancedSettings': 'Impostazioni avanzate', 'topbar.searchPlaceholder': 'Cerca un gioco o un obiettivo',
        'topbar.searchSuggestions': 'Suggerimenti di ricerca', 'topbar.noSuggestion': 'Nessun suggerimento', 'topbar.kind.game': 'Gioco',
        'topbar.kind.achievement': 'Obiettivo', 'topbar.sync': 'Aggiorna', 'topbar.syncing': 'Sincronizzazione...', 'topbar.syncTitle': 'Sincronizza con Steam',
        'topbar.scan': '● Scan', 'topbar.library': 'Libreria', 'topbar.gameSubtitle': 'Gioco · AppID {id}', 'main.recentActivity': 'Attività recente',
        'main.noRecentActivity': 'Nessuna attività recente', 'main.recentLibrary': 'Libreria recente', 'setup.title': 'Setup',
        'setup.steam': 'Steam', 'setup.steamId': 'Steam ID (64 bit)', 'setup.steamApiKey': 'Chiave API Steam', 'setup.sgdbApiKey': 'Chiave API SteamGridDB',
        'setup.link.steamId': 'Apri la pagina del tuo account Steam', 'setup.link.steamApiKey': 'Apri la pagina Steam Web API Key', 'setup.link.sgdbApiKey': 'Apri la pagina API SteamGridDB',
        'setup.subtitle': 'Preferenze lingua, notifiche e aspetto', 'setup.language': 'Lingua', 'setup.notifications': 'Notifiche',
        'setup.position': 'Posizione HUD', 'setup.sound': 'Suono notifica', 'setup.test': 'Testa notifica', 'setup.appearance': 'Aspetto',
        'setup.theme': 'Tema', 'setup.theme.dark': 'Scuro', 'setup.theme.light': 'Chiaro', 'setup.accent': 'Colore accento', 'setup.save': 'Salva',
        'setup.theme.system': 'Sistema',
        'setup.saving': 'Salvataggio...', 'setup.saved': 'Preferenze salvate', 'setup.errorSave': 'Salvataggio fallito: {details}',
        'setup.errorTest': 'Test fallito: {details}', 'setup.position.top-left': 'Alto sinistra', 'setup.position.top-center': 'Alto centro',
        'setup.position.top-right': 'Alto destra', 'setup.position.bottom-left': 'Basso sinistra', 'setup.position.bottom-center': 'Basso centro',
        'setup.position.bottom-right': 'Basso destra', 'setup.sound.none': 'Nessun suono', 'journal.title': 'Diario',
        'journal.subtitle': 'Cronologia dei tuoi obiettivi sbloccati', 'journal.allGames': 'Tutti i giochi', 'journal.countAchievements': '{count} obiettivi',
        'journal.completed': 'Gioco completato al 100%!', 'journal.empty': 'Nessun evento trovato.', 'objectives.title': 'Obiettivi',
        'objectives.subtitle': 'Giochi più vicini al 100%', 'objectives.sort.progression': 'Progressione', 'objectives.sort.remaining': 'Obiettivi rimanenti',
        'objectives.easy': 'Obiettivi facili', 'objectives.easySubtitle': 'Obiettivi più facili rimasti',
        'objectives.remaining': '{unlocked} / {total} — {remaining} rimanenti', 'objectives.details': 'Vedi dettagli', 'statsPanel.stats': 'Stats',
        'statsPanel.unlocked': 'Obiettivi sbloccati', 'statsPanel.outOf': 'su {total} totali', 'statsPanel.avgProgress': 'Progresso medio',
        'statsPanel.avgRarity': 'Rarità media', 'statsPanel.rarityBreakdown': 'Distribuzione rarità', 'statsPanel.recentCompleted': 'Completati recenti'
    }
} as const;

type TranslationKey = keyof typeof translations.fr;
type TranslationParams = Record<string, string | number>;

function normalizeLanguage(input: string): LanguageCode {
    return (languages as readonly string[]).includes(input) ? (input as LanguageCode) : 'fr';
}

function interpolate(template: string, params?: TranslationParams): string {
    if (!params) return template;
    return template.replace(/\{(\w+)\}/g, (_, key: string) => String(params[key] ?? `{${key}}`));
}

export function translate(language: LanguageCode, key: TranslationKey, params?: TranslationParams): string {
    const localized = translations[language][key] ?? translations.fr[key];
    return interpolate(localized, params);
}

export const languageOptions = [
    { value: 'fr', label: 'Français' },
    { value: 'en', label: 'English' },
    { value: 'es', label: 'Español' },
    { value: 'de', label: 'Deutsch' },
    { value: 'it', label: 'Italiano' }
] as const;

export const i18n = derived(settings, ($settings) => {
    const language = normalizeLanguage($settings.language);
    const locale = localeByLanguage[language];

    return {
        language,
        locale,
        t: (key: TranslationKey, params?: TranslationParams) => translate(language, key, params)
    };
});

const rarityByLanguage: Record<LanguageCode, string[]> = {
    fr: ['Mythique', 'Légendaire', 'Épique', 'Très rare', 'Rare', 'Peu commun', 'Commun'],
    en: ['Mythic', 'Legendary', 'Epic', 'Very rare', 'Rare', 'Uncommon', 'Common'],
    es: ['Mítico', 'Legendario', 'Épico', 'Muy raro', 'Raro', 'Poco común', 'Común'],
    de: ['Mythisch', 'Legendär', 'Episch', 'Sehr selten', 'Selten', 'Ungewöhnlich', 'Gewöhnlich'],
    it: ['Mitico', 'Leggendario', 'Epico', 'Molto raro', 'Raro', 'Non comune', 'Comune']
};

export function rarityLabelByIndex(language: LanguageCode, index: number): string {
    const list = rarityByLanguage[language] ?? rarityByLanguage.fr;
    return list[index] ?? list[list.length - 1];
}
