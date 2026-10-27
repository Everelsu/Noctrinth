/**
 * Noctrinth's own: what the quick search finds the settings tabs by.
 *
 * A tab is found by its name in the language the app is in, and also by the
 * words below, in English and Russian whichever language is on: someone used to
 * the English names who switched the app to Russian still types "proxy", and
 * the other way round. The words name what is inside each tab, not only the
 * tab, so "шрифт" or "java args" lead to the right one.
 *
 * Keyed by the tab name's message id, which is how the settings dialog opens a
 * tab, so upstream's tab list needs no ids of its own.
 */
import type { Component } from 'vue'

export interface SettingsTabEntry {
	/** The tab name's message id. */
	id: string
	name: { id: string; defaultMessage: string }
	category: { id: string; defaultMessage: string }
	icon: Component
}

const KEYWORDS: Record<string, string[]> = {
	'app.settings.tabs.appearance': [
		'appearance theme dark light oled accent colour color wheel tint font typeface minecraft font background wallpaper picture gif video blur',
		'оформление внешний вид тема тёмная светлая акцент цвет круг оттенок шрифт фон обои картинка гифка видео размытие',
	],
	'app.settings.tabs.features': [
		'features worlds tab files tab screenshots tab sidebar skin selector quick instances play page',
		'возможности функции вкладка миры файлы скриншоты боковая панель выбор скинов быстрые сборки страница играть',
	],
	'app.settings.tabs.behavior': [
		'behavior behaviour startup minimize refocus home library right sidebar confirmations warnings',
		'поведение запуск свернуть сворачивать фокус главная библиотека правая панель подтверждения предупреждения',
	],
	'app.settings.tabs.language': [
		'language locale translation translate google deepl api key description',
		'язык локализация перевод переводчик гугл deepl ключ описание',
	],
	'app.settings.tabs.changelog': [
		'changelog changes release notes what is new versions',
		'список изменений изменения чейнджлог что нового версии',
	],
	'settings.feature-flags.title': [
		'feature flags developer experimental splash cube',
		'флаги разработчик экспериментальные заставка куб',
	],
	'settings.profile.navigation-title': [
		'profile modrinth account avatar username bio',
		'профиль аккаунт modrinth аватар ник описание',
	],
	'settings.social.title': [
		'social friends friend requests block blocked invites',
		'общение друзья заявки блокировка заблокированные приглашения',
	],
	'app.settings.tabs.privacy': [
		'privacy telemetry analytics ads discord rich presence',
		'приватность конфиденциальность телеметрия аналитика реклама дискорд статус',
	],
	'app.settings.tabs.java-and-memory': [
		'java jre jdk memory ram allocation arguments jvm args environment variables graphics adapter gpu download provider azul temurin mojang mirror',
		'джава ява память оперативка озу выделение аргументы переменные окружения видеокарта графический адаптер скачивание источник зеркало',
	],
	'app.settings.tabs.window': [
		'window fullscreen width height resolution size',
		'окно полноэкранный полный экран ширина высота разрешение размер',
	],
	'app.settings.tabs.launch-hooks': [
		'launch hooks pre-launch wrapper post-exit command script',
		'хуки запуска перед запуском обёртка после выхода команда скрипт',
	],
	'app.settings.tabs.syncing': [
		'syncing sync resource packs data packs game options options.txt servers command history hotbars skins every player recover instances',
		'синхронизация ресурспаки датапаки настройки игры серверы история команд хотбары скины игроков восстановить сборки',
	],
	'app.settings.tabs.resources': [
		'resources storage disk space cache unused downloads verify repair app directory folder proxy network vpn',
		'ресурсы хранилище место на диске кэш неиспользуемые загрузки проверить восстановить папка приложения прокси сеть впн',
	],
	'app.settings.tabs.troubleshooting': [
		'troubleshooting debug info export logs report ad clear cache database backups',
		'устранение неполадок отладка экспорт логи пожаловаться на рекламу очистить кэш база данных резервные копии',
	],
	'app.settings.tabs.curseforge': [
		'curseforge api key restricted files updates release type install links share code other launchers',
		'курсфордж ключ api запрещённые файлы обновления тип релиза ссылки установить код поделиться другие лаунчеры',
	],
}

/**
 * Whether a tab answers a query: every word typed has to be found, in the
 * tab's names or in its words, as the start of one of them — "jav" finds
 * Java, "мод" does not find "модальн…" in the middle of something else.
 */
export function matchSettingsTab(query: string, tab: SettingsTabEntry, names: string[]): boolean {
	const words = query.toLowerCase().split(/\s+/).filter(Boolean)
	if (!words.length) return false
	const haystack = [...names, tab.name.defaultMessage, ...(KEYWORDS[tab.id] ?? [])]
		.join(' ')
		.toLowerCase()
		.replace(/ё/g, 'е')
		.split(/[^\p{L}\p{N}.+-]+/u)
	return words.every((word) => {
		const needle = word.replace(/ё/g, 'е')
		return haystack.some((candidate) => candidate.startsWith(needle))
	})
}
