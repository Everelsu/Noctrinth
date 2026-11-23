// Plugin developer documentation (/docs/): complete guide for building,
// testing, publishing, and updating themes, content sources, and extensions.

import { icon, page, PLUGINS_REPO, TEMPLATE_REPO, t } from './site-style.mjs'

const DOCS_CSS = `
	.docs-hero {
		padding: 2.5rem 0 2rem;
		border-bottom: 1px solid var(--brand-gradient-border);
		background: var(--brand-gradient-bg);
	}
	.docs-hero h1 {
		margin: 0;
		font-size: clamp(1.85rem, 3.8vw, 2.4rem);
		font-weight: 800;
		letter-spacing: -0.025em;
		color: var(--text-contrast);
	}
	.docs-hero p {
		margin: 0.5rem 0 0;
		max-width: 46rem;
		color: var(--text);
	}
	.docs-hero__actions { margin-top: 1.25rem; display: flex; flex-wrap: wrap; gap: 0.6rem; }

	.docs-layout {
		display: grid;
		grid-template-columns: 235px minmax(0, 1fr);
		gap: 2.5rem;
		margin-top: 2rem;
		align-items: start;
	}
	.docs-toc {
		position: sticky;
		top: 5.25rem;
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
		padding: 1rem;
		border-radius: var(--radius-lg);
		background: var(--surface-2);
		border: 1px solid var(--button-border);
	}
	.docs-toc__title {
		font-size: 0.74rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--text-secondary);
		margin: 0 0 0.4rem 0.5rem;
	}
	.docs-toc a {
		padding: 0.38rem 0.6rem;
		border-radius: 0.5rem;
		color: var(--text);
		font-size: 0.88rem;
		font-weight: 500;
	}
	.docs-toc a:hover { background: var(--surface-3); color: var(--text-contrast); text-decoration: none; }

	.docs-content { min-width: 0; }
	.docs-section { scroll-margin-top: 5.25rem; margin-bottom: 3.25rem; }
	.docs-section h2 {
		margin: 0 0 0.9rem;
		padding-bottom: 0.45rem;
		border-bottom: 1px solid var(--divider);
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--text-contrast);
	}
	.docs-section h3 {
		margin: 1.6rem 0 0.5rem;
		font-size: 1.12rem;
		font-weight: 700;
		color: var(--text-contrast);
	}
	.docs-section p { margin: 0.65rem 0; line-height: 1.65; }
	.docs-section ul, .docs-section ol { margin: 0.65rem 0; padding-left: 1.35rem; line-height: 1.65; }
	.docs-section li { margin: 0.35rem 0; }
	.docs-table-wrap { overflow-x: auto; margin: 1rem 0; border-radius: var(--radius-lg); border: 1px solid var(--button-border); }
	.docs-table { width: 100%; border-collapse: collapse; font-size: 0.9rem; background: var(--surface-2); }
	.docs-table th, .docs-table td {
		text-align: left;
		padding: 0.65rem 0.85rem;
		border-bottom: 1px solid var(--divider);
		vertical-align: top;
	}
	.docs-table th { color: var(--text-contrast); font-weight: 700; background: var(--surface-3); }
	.docs-table tr:last-child td { border-bottom: 0; }

	@media (max-width: 860px) {
		.docs-layout { grid-template-columns: 1fr; gap: 1.5rem; }
		.docs-toc { position: static; flex-direction: row; flex-wrap: wrap; }
		.docs-toc__title { width: 100%; }
	}
`

export function renderDocsPage() {
	return page({
		title: 'Plugin Documentation — Noctrinth',
		description:
			'Complete developer guide for building, testing, publishing, and updating themes, content sources, and extensions for Noctrinth Launcher.',
		prefix: '../',
		current: 'docs',
		css: DOCS_CSS,
		body: `
		<header class="docs-hero">
			<div class="container container--wide">
				<h1>${t('Plugin Documentation', 'Документация по плагинам')}</h1>
				<p>${t(
					'Complete guide to building, debugging, publishing, and updating CSS themes, Discover content sources, and sandboxed JavaScript extensions for Noctrinth (API v1).',
					'Полное руководство по созданию, отладке, публикации и обновлению CSS-тем, источников контента в «Исследовании» и изолированных JS-расширений для Noctrinth (API v1).',
				)}</p>
				<div class="docs-hero__actions">
					<a class="btn btn--brand" href="${TEMPLATE_REPO}" target="_blank" rel="noopener">${icon('github')} ${t('Use Starter Template', 'Открыть шаблон')}</a>
					<a class="btn" href="${PLUGINS_REPO}" target="_blank" rel="noopener">${icon('external')} ${t('Plugin Catalog Repo', 'Репозиторий каталога')}</a>
					<a class="btn" href="../plugins/">${icon('plug')} ${t('Browse Catalog', 'Смотреть каталог')}</a>
				</div>
			</div>
		</header>

		<main class="container container--wide docs-layout">
			<aside class="docs-toc">
				<p class="docs-toc__title">${t('On this page', 'Содержание')}</p>
				<a href="#quickstart">${t('1. Quick start & dev loop', '1. Быстрый старт и отладка')}</a>
				<a href="#manifest">${t('2. Manifest (plugin.json)', '2. Манифест (plugin.json)')}</a>
				<a href="#themes">${t('3. CSS themes & settings', '3. CSS-темы и настройки')}</a>
				<a href="#permissions">${t('4. Permissions & limits', '4. Права и ограничения')}</a>
				<a href="#sdk">${t('5. JavaScript SDK', '5. JavaScript SDK')}</a>
				<a href="#publish">${t('6. Publishing to catalog', '6. Публикация в каталог')}</a>
				<a href="#updating">${t('7. Updating your plugin', '7. Как выпускать обновления')}</a>
				<a href="#troubleshooting">${t('8. Troubleshooting', '8. Частые ошибки')}</a>
			</aside>

			<div class="docs-content">
				<section class="docs-section" id="quickstart">
					<h2>${t('1. Quick start & local dev loop', '1. Быстрый старт и локальная отладка')}</h2>
					<p>${t(
						'A Noctrinth plugin is a plain folder (or <code>.zip</code>) with a <code>plugin.json</code> at its root. No build step is required unless you want TypeScript or a bundler.',
						'Плагин Noctrinth — это обычная папка (или <code>.zip</code>-архив) с файлом <code>plugin.json</code> в корне. Сборщик не обязателен, если вам не нужен TypeScript.',
					)}</p>
<pre><code>my-plugin/
├── plugin.json   # Required manifest
├── theme.css     # Optional stylesheet(s)
├── index.js      # Optional JS worker script
├── icon.png      # Optional square icon for catalog cards
└── preview.jpg   # Optional 16:9 banner preview for catalog cards</code></pre>
					<ol>
						<li>${t(
							`Click <strong>Use this template</strong> on <a href="${TEMPLATE_REPO}" target="_blank" rel="noopener">Noctrinth-Plugin-Template</a> and clone your new repository.`,
							`Нажмите <strong>Use this template</strong> в <a href="${TEMPLATE_REPO}" target="_blank" rel="noopener">Noctrinth-Plugin-Template</a> и склонируйте свой репозиторий.`,
						)}</li>
						<li>${t(
							'Edit <code>plugin.json</code> (change <code>id</code>, <code>name</code>, <code>author</code>, and <code>homepage</code>).',
							'Отредактируйте <code>plugin.json</code> (задайте свой <code>id</code>, <code>name</code>, <code>author</code> и <code>homepage</code>).',
						)}</li>
						<li>${t(
							'Zip the files directly (make sure <code>plugin.json</code> is at the <strong>root</strong> of the archive, not inside a nested subfolder) and install via <strong>Settings → Plugins → Install from .zip</strong>.',
							'Упакуйте файлы в <code>.zip</code> (важно: <code>plugin.json</code> должен лежать прямо в <strong>корне</strong> архива, а не внутри вложенной папки) и установите через <strong>Настройки → Плагины → Установить из .zip</strong>.',
						)}</li>
						<li>${t(
							'For fast iteration, click <strong>Open folder</strong> in Settings → Plugins, edit your plugin files directly in place, and press the <strong>Reload</strong> icon on the plugin card. Both CSS and the JS worker reload immediately without restarting Noctrinth.',
							'Для быстрой итерации нажмите <strong>Папка плагинов</strong>, правьте файлы прямо там и нажимайте кнопку <strong>Перезагрузить</strong> на карточке плагина — CSS и JS-воркер перезапустятся мгновенно без перезапуска лаунчера.',
						)}</li>
						<li>${t(
							'Click the <strong>Logs</strong> icon (<code>&lt;/&gt;</code>) on your plugin card to inspect live <code>console.log / warn / error</code> output from the worker.',
							'Нажмите иконку <strong>логов</strong> (<code>&lt;/&gt;</code>) на карточке плагина, чтобы в реальном времени видеть вывод <code>console.log / warn / error</code> из воркера.',
						)}</li>
					</ol>
				</section>

				<section class="docs-section" id="manifest">
					<h2>${t('2. Manifest (plugin.json)', '2. Манифест (plugin.json)')}</h2>
					<p>${t(
						'Include <code>$schema</code> at the top of <code>plugin.json</code> to get live validation and autocomplete in VS Code / WebStorm:',
						'Добавьте <code>$schema</code> в начало <code>plugin.json</code> — редактор сам подскажет поля и проверит ошибки:',
					)}</p>
<pre><code>{
  "$schema": "https://everelsu.github.io/Noctrinth-Plugins/plugin-schema.json",
  "id": "my-awesome-plugin",
  "name": "My Awesome Plugin",
  "version": "1.0.0",
  "apiVersion": 1,
  "type": "extension",
  "author": "YourName",
  "description": "Short summary of what this plugin does.",
  "homepage": "https://github.com/You/My-Awesome-Plugin",
  "repository": "https://github.com/You/My-Awesome-Plugin",
  "icon": "icon.png",
  "banner": "preview.jpg",
  "tags": ["utility", "ui"],
  "styles": ["theme.css"],
  "entry": "index.js",
  "permissions": {
    "domains": ["api.example.com"],
    "events": ["instance:launch", "instance:crash"],
    "instances": ["read"],
    "ui": ["notifications", "actions", "modal", "settings"]
  }
}</code></pre>
					<div class="docs-table-wrap">
						<table class="docs-table">
							<thead>
								<tr>
									<th>${t('Field', 'Поле')}</th>
									<th>${t('Required', 'Обяз.')}</th>
									<th>${t('Description', 'Описание')}</th>
								</tr>
							</thead>
							<tbody>
								<tr><td><code>id</code></td><td>${t('Yes', 'Да')}</td><td>${t('Unique kebab-case ID (3–64 lowercase chars, digits, hyphens). Never change it after publishing.', 'Уникальный kebab-case ID (3–64 строчных символа, цифры, дефисы). Не меняйте его после публикации.')}</td></tr>
								<tr><td><code>name</code></td><td>${t('Yes', 'Да')}</td><td>${t('Human-readable display name (up to 80 chars).', 'Отображаемое название (до 80 символов).')}</td></tr>
								<tr><td><code>version</code></td><td>${t('Yes', 'Да')}</td><td>${t('Semantic version (<code>1.0.0</code>, <code>1.2.0-beta.1</code>). Bump this on every release so users get the Update button.', 'Версия по SemVer (<code>1.0.0</code>, <code>1.2.0-beta.1</code>). Повышайте при каждом обновлении, чтобы у юзеров появилась кнопка «Обновить».')}</td></tr>
								<tr><td><code>apiVersion</code></td><td>${t('Yes', 'Да')}</td><td>${t('Must be <code>1</code>.', 'Должно быть <code>1</code>.')}</td></tr>
								<tr><td><code>type</code></td><td>${t('Yes', 'Да')}</td><td><code>"theme"</code> | <code>"content-source"</code> | <code>"extension"</code></td></tr>
								<tr><td><code>icon</code> / <code>banner</code></td><td>${t('No', 'Нет')}</td><td>${t('Relative image paths inside your repo (e.g. <code>icon.png</code>, <code>preview.jpg</code>) or HTTPS URLs. The catalog builder automatically copies local images for the website and app.', 'Относительные пути к картинкам в репо (<code>icon.png</code>, <code>preview.jpg</code>) или HTTPS-ссылки. Сборщик каталога сам скопирует локальные картинки для сайта и приложения.')}</td></tr>
								<tr><td><code>styles</code></td><td>${t('No', 'Нет')}</td><td>${t('Array of relative CSS files injected while the plugin is enabled.', 'Массив относительных CSS-файлов, подключаемых при включении плагина.')}</td></tr>
								<tr><td><code>entry</code></td><td>${t('No', 'Нет')}</td><td>${t('Relative JS file executed inside a sandboxed Web Worker.', 'Относительный JS-файл, запускаемый в изолированном Web Worker.')}</td></tr>
								<tr><td><code>tags</code></td><td>${t('No', 'Нет')}</td><td>${t('Short search tags displayed on the catalog card.', 'Короткие теги для поиска и отображения на карточке в каталоге.')}</td></tr>
							</tbody>
						</table>
					</div>
				</section>

				<section class="docs-section" id="themes">
					<h2>${t('3. CSS themes & live settings', '3. CSS-темы и живые настройки')}</h2>
					<p>${t(
						'Pure themes (<code>"type": "theme"</code>) need zero JavaScript. Every field you declare in <code>settings</code> is automatically rendered in the plugin card and projected onto <code>&lt;html&gt;</code> in real time:',
						'Чистым темам (<code>"type": "theme"</code>) не нужен JS. Каждое поле из <code>settings</code> автоматически превращается в контрол на карточке плагина и проецируется на <code>&lt;html&gt;</code> в реальном времени:',
					)}</p>
					<ul>
						<li><code>data-plugin-&lt;id&gt;-&lt;key&gt;="&lt;value&gt;"</code> — ${t('great for toggling CSS rules with <code>boolean</code> or <code>select</code>.', 'идеально для переключения блоков CSS через <code>boolean</code> или <code>select</code>.')}</li>
						<li><code>--noctrinth-plugin-&lt;id&gt;-&lt;key&gt;</code> — ${t('automatic CSS custom property.', 'автоматическая CSS-переменная.')}</li>
						<li><code>cssVar</code> — ${t('optional custom CSS variable name (e.g. <code>--fb-radius</code> or <code>--color-brand</code>).', 'необязательное своё имя CSS-переменной (например, <code>--fb-radius</code> или <code>--color-brand</code>).')}</li>
						<li><code>group</code> &amp; <code>when</code> — ${t('organize settings into titled groups and conditionally show fields with <code>when: { "key": "...", "equals": true }</code>.', 'группируйте настройки по разделам и скрывайте зависимые поля через <code>when: { "key": "...", "equals": true }</code>.')}</li>
						<li><code>presets</code> — ${t('one-click preset buttons in plugin settings that apply a bundle of setting values at once.', 'готовые пресеты в один клик, применяющие сразу набор настроек темы.')}</li>
						<li>${t('<strong>Local assets in CSS:</strong> relative <code>url("./fonts/my.woff2")</code> or <code>url("./bg.png")</code> references in your stylesheets are automatically inlined from your plugin folder (up to 4 MB per file).', '<strong>Локальные ассеты в CSS:</strong> относительные ссылки <code>url("./fonts/my.woff2")</code> или <code>url("./bg.png")</code> в стилях автоматически подгружаются из папки плагина (до 4 МБ на файл).')}</li>
					</ul>
<pre><code>"settings": [
  {
    "key": "floating",
    "type": "boolean",
    "label": "Floating layout",
    "default": true
  },
  {
    "key": "radius",
    "type": "select",
    "label": "Corner radius",
    "default": "16px",
    "cssVar": "--fb-radius",
    "options": [
      { "value": "10px", "label": "Compact (10px)" },
      { "value": "16px", "label": "Rounded (16px)" }
    ]
  },
  {
    "key": "borderAccent",
    "type": "color",
    "label": "Border tint",
    "default": "#a855f7",
    "cssVar": "--fb-accent"
  }
]</code></pre>
					<h3>${t('Useful Noctrinth selectors & variables', 'Полезные селекторы и переменные Noctrinth')}</h3>
					<p>${t(
						'You can prototype CSS live in <strong>Settings → Plugins → Quick CSS</strong> before copying it into your <code>theme.css</code>:',
						'Удобно набрасывать стили прямо в <strong>Настройки → Плагины → Свой CSS (Quick CSS)</strong>, а потом переносить в <code>theme.css</code>:',
					)}</p>
					<div class="docs-table-wrap">
						<table class="docs-table">
							<thead>
								<tr>
									<th>${t('Selector / Variable', 'Селектор / переменная')}</th>
									<th>${t('What it targets', 'За что отвечает')}</th>
								</tr>
							</thead>
							<tbody>
								<tr><td><code>.app-grid-layout</code></td><td>${t('Outer window shell grid.', 'Внешняя сетка окна приложения.')}</td></tr>
								<tr><td><code>.app-title-bar</code></td><td>${t('Top title bar (logo, back/forward, breadcrumbs, actions).', 'Верхняя панель (логотип, навигация, хлебные крошки).')}</td></tr>
								<tr><td><code>.app-window-controls</code></td><td>${t('Minimize / maximize / close buttons (teleported to <code>body</code>).', 'Кнопки свернуть/развернуть/закрыть (телепортируются в <code>body</code>).')}</td></tr>
								<tr><td><code>.app-grid-navbar</code></td><td>${t('Left navigation rail.', 'Левая боковая панель навигации.')}</td></tr>
								<tr><td><code>.app-contents</code> / <code>.app-viewport</code></td><td>${t('Main content area wrapper and scrollable page viewport.', 'Обёртка основного контента и скроллируемая область страниц.')}</td></tr>
								<tr><td><code>.app-sidebar</code></td><td>${t('Right collapsible sidebar (accounts, friends, news, Discover filters).', 'Правый сворачиваемый сайдбар (аккаунты, друзья, новости, фильтры).')}</td></tr>
								<tr><td><code>--color-bg</code>, <code>--color-raised-bg</code>, <code>--color-brand</code>, <code>--surface-1..5</code></td><td>${t('Core theme tokens that adapt to the user’s chosen accent and dark/light mode.', 'Основные токены темы, подстраивающиеся под выбранный акцент и режим.')}</td></tr>
							</tbody>
						</table>
					</div>
				</section>

				<section class="docs-section" id="permissions">
					<h2>${t('4. Permissions & runtime limits', '4. Права и ограничения рантайма')}</h2>
					<p>${t(
						'Scripts run inside an isolated Web Worker with no direct DOM, Node, or filesystem access. Every host capability requires an explicit permission in <code>plugin.json</code>:',
						'Скрипты работают в изолированном Web Worker без прямого доступа к DOM, Node или файловой системе. Любая возможность требует явного права в <code>plugin.json</code>:',
					)}</p>
					<ul>
						<li><code>permissions.domains</code> — ${t('HTTPS host allowlist enforced in Rust for <code>noctrinth.fetch()</code> (e.g. <code>["api.github.com"]</code>). Local HTTP is also allowed for <code>"localhost"</code> / <code>"127.0.0.1"</code> (e.g. Ollama or LM Studio).', 'белый список HTTPS-доменов для <code>noctrinth.fetch()</code>, проверяется в Rust (например, <code>["api.github.com"]</code>). Для <code>"localhost"</code> / <code>"127.0.0.1"</code> разрешён и локальный HTTP (например, Ollama или LM Studio).')}</li>
						<li><code>permissions.events</code> — <code>instance:launch</code>, <code>instance:exit</code>, <code>instance:crash</code>, <code>download:complete</code>.</li>
						<li><code>permissions.instances</code> — <code>read</code> (${t('list, inspect, logs, list/read instance files', 'список, свойства, логи, чтение файлов сборки')}), <code>write</code> (${t('toggle/remove mods, write text/config files inside an instance', 'вкл/выкл/удаление модов, запись текстовых конфигов в папке сборки')}), <code>launch</code>.</li>
						<li><code>permissions.ui</code> — <code>notifications</code>, <code>actions</code>, <code>settings</code>, <code>modal</code>, <code>panel</code> (${t('sandboxed slide-over HTML drawer', 'изолированная выдвижная HTML-панель')}), <code>page</code> (${t('dedicated full-page sidebar view', 'отдельная страница в левом сайдбаре')}), <code>clipboard</code>.</li>
					</ul>
					<div class="callout">
						<strong>${t('Built-in safeguards:', 'Встроенная защита:')}</strong>
						${t(
							' Up to 60 RPC calls/sec (excessive floods >200/sec stop the worker), up to 6 toast notifications per 10 seconds (extra toasts are silently throttled), 15s timeout on Discover searches, automatic Circuit Breaker that disables crashing plugins, and emergency <strong>Safe Mode</strong> via <code>Ctrl+Shift+Alt+P</code> or <code>--safe-mode</code>.',
							' До 60 RPC-вызовов/с (злостный флуд >200/с останавливает воркер), до 6 уведомлений за 10 секунд (лишние мягко пропускаются), таймаут 15с на поиск в «Исследовании», автоматический Circuit Breaker при падении и аварийный <strong>Безопасный режим</strong> по <code>Ctrl+Shift+Alt+P</code> или флагу <code>--safe-mode</code>.',
						)}
					</div>
				</section>

				<section class="docs-section" id="sdk">
					<h2>${t('5. JavaScript SDK (noctrinth.*)', '5. JavaScript SDK (noctrinth.*)')}</h2>
					<p>${t(
						'Full TypeScript definitions are shipped in <code>sdk/index.d.ts</code> inside the starter template. Top-level <code>await</code> is supported out of the box.',
						'Полные TypeScript-типы лежат в <code>sdk/index.d.ts</code> внутри шаблона. Top-level <code>await</code> поддерживается из коробки.',
					)}</p>
					<h3>${t('UI Actions & Declarative Modals', 'Кнопки UI и декларативные модалки')}</h3>
					<p>${t(
						'Available action slots: <code>"sidebar"</code> (left nav rail), <code>"instance-header"</code> (receives <code>{ instanceId, instanceName }</code>), and <code>"mod-menu"</code> (receives <code>{ instanceId, filePath, projectName }</code>).',
						'Доступные слоты кнопок: <code>"sidebar"</code> (левая панель), <code>"instance-header"</code> (получает <code>{ instanceId, instanceName }</code>) и <code>"mod-menu"</code> (получает <code>{ instanceId, filePath, projectName }</code>).',
					)}</p>
<pre><code>await noctrinth.ui.registerAction(
  { id: 'summary', slot: 'sidebar', label: 'My Plugin', tooltip: 'Open overview' },
  async () => {
    const instances = await noctrinth.instances.list()
    const choice = await noctrinth.ui.showModal({
      title: 'Instances Overview',
      description: 'Rendered natively by Noctrinth.',
      blocks: [
        { type: 'text', text: 'Installed instances: ' + instances.length },
        { type: 'progress', label: 'Status', value: 100, max: 100 },
        { type: 'list', items: instances.slice(0, 5).map((i) => ({ title: i.name, subtitle: i.game_version })) },
        { type: 'code', text: JSON.stringify({ count: instances.length }, null, 2) }
      ],
      actions: [
        { id: 'toast', label: 'Send toast', color: 'brand' },
        { id: 'close', label: 'Close' }
      ]
    })
    if (choice === 'toast') {
      await noctrinth.notify({ title: 'Done', text: 'Modal action clicked', type: 'success' })
    }
  }
)</code></pre>
					<h3>${t('Network, Storage, Settings & Events', 'Сеть, хранилище, настройки и события')}</h3>
<pre><code>// HTTP fetch (domain must be listed in permissions.domains)
const res = await noctrinth.fetch('https://api.example.com/status')
const json = JSON.parse(res.body)

// Per-plugin persistent key-value storage
await noctrinth.storage.set('lastSync', String(Date.now()))
const lastSync = await noctrinth.storage.get('lastSync')

// Read settings & react to live changes
const allSettings = await noctrinth.settings.getAll()
noctrinth.on('settings:change', ({ key, value }) => {
  console.log('Setting updated:', key, value)
})

// React to launcher events (requires permissions.events)
noctrinth.on('instance:crash', ({ instanceId, summary }) => {
  console.warn('Crash detected in', instanceId, summary)
})</code></pre>
					<h3>${t('Sandboxed Custom UI: Panels & Dedicated Pages', 'Свой изолированный UI: выдвижные панели и страницы')}</h3>
					<p>${t(
						'Need a full custom interface such as an AI chat, server monitor, or config editor? Plugins can open a slide-over drawer (<code>noctrinth.ui.openPanel</code>) or add a dedicated page to the left navigation rail (<code>noctrinth.ui.registerPage</code> or <code>pages</code> in <code>plugin.json</code>). Your HTML runs inside a sandboxed <code>iframe</code> with Noctrinth’s live theme CSS variables and communicates with your worker via <code>window.noctrinthPanel</code>.',
						'Нужен свой полноценный интерфейс — например, ИИ-чат, монитор серверов или редактор конфигов? Плагин может открыть выдвижную панель (<code>noctrinth.ui.openPanel</code>) или добавить отдельную страницу в левый сайдбар (<code>noctrinth.ui.registerPage</code> или поле <code>pages</code> в <code>plugin.json</code>). HTML работает в изолированном <code>iframe</code> с живыми CSS-переменными темы Noctrinth и общается с воркером через <code>window.noctrinthPanel</code>.',
					)}</p>
<pre><code>// main.js (Worker) — requires permissions.ui: ["actions", "panel"]
await noctrinth.ui.registerAction(
  { id: 'ai-chat', slot: 'sidebar', label: 'AI Chat', tooltip: 'Open AI assistant' },
  async () => {
    await noctrinth.ui.openPanel({
      id: 'chat',
      title: 'AI Assistant',
      entry: 'panel.html', // loaded from your plugin folder
      width: 440
    })
  }
)

noctrinth.ui.onPanelMessage(async (msg, panelId) => {
  if (msg.type === 'ask') {
    await noctrinth.fetchStream(
      'http://localhost:11434/api/generate',
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ model: 'llama3', prompt: msg.text, stream: true })
      },
      async (chunk) => {
        for (const line of chunk.split('\n').filter(Boolean)) {
          const token = JSON.parse(line).response
          if (token) await noctrinth.ui.postPanelMessage({ type: 'token', text: token }, panelId)
        }
      }
    )
  }
})</code></pre>
<pre><code>&lt;!-- panel.html (runs inside sandboxed iframe) --&gt;
&lt;input id="q" placeholder="Ask something..." /&gt;
&lt;button onclick="out.textContent=''; noctrinthPanel.send({ type: 'ask', text: q.value })"&gt;Send&lt;/button&gt;
&lt;pre id="out"&gt;&lt;/pre&gt;
&lt;script&gt;
  noctrinthPanel.onMessage((msg) =&gt; {
    if (msg.type === 'token') document.getElementById('out').textContent += msg.text
  })
&lt;/script&gt;</code></pre>
					<h3>${t('Interactive Forms, Instance Configs & App Info', 'Интерактивные формы, файлы сборок и данные приложения')}</h3>
<pre><code>// Interactive form modal with input fields + live updates
const form = await noctrinth.ui.showForm({
  title: 'Configure endpoint',
  blocks: [
    { type: 'input', id: 'url', label: 'Server URL', placeholder: 'https://...' },
    { type: 'input', id: 'notes', label: 'Notes', multiline: true }
  ],
  actions: [{ id: 'save', label: 'Save', color: 'brand' }]
})

// Read & write config files safely inside an instance folder
const files = await noctrinth.instances.listFiles(instanceId, 'config')
const cfg = await noctrinth.instances.readFile(instanceId, 'options.txt')
await noctrinth.instances.writeFile(instanceId, 'config/my-plugin.json', JSON.stringify({ ok: true }))

// Clipboard & read-only app info (locale, theme)
await noctrinth.clipboard.writeText('Copied from plugin')
const info = await noctrinth.app.getInfo()</code></pre>
					<h3>${t('Custom Discover Content Source', 'Свой источник контента в «Исследовании»')}</h3>
<pre><code>await noctrinth.contentSource.register({
  label: 'Hangar',
  projectTypes: ['mod', 'plugin'],
  async search({ query, projectType, offset, limit }) {
    const res = await noctrinth.fetch('https://hangar.papermc.io/api/v1/projects?q=' + encodeURIComponent(query))
    const data = JSON.parse(res.body)
    return {
      hits: (data.result || []).map((item) => ({
        id: item.namespace.slug,
        title: item.name,
        description: item.description,
        author: item.namespace.owner,
        icon_url: item.avatarUrl,
        downloads: item.stats.downloads,
        external_url: 'https://hangar.papermc.io/' + item.namespace.owner + '/' + item.namespace.slug
      })),
      total_hits: data.pagination?.count || 0,
      offset,
      limit
    }
  }
})</code></pre>
				</section>

				<section class="docs-section" id="publish">
					<h2>${t('6. Publishing to the Official Catalog', '6. Публикация в официальный каталог')}</h2>
					<p>${t(
						`Official community plugins live in <a href="${PLUGINS_REPO}" target="_blank" rel="noopener">Everelsu/Noctrinth-Plugins</a> as <strong>Git submodules</strong> (see <a href="https://github.com/Everelsu/Noctrinth-Floating-Bars" target="_blank" rel="noopener">Noctrinth-Floating-Bars</a> for a live example). This keeps every plugin pinned to a reviewed commit.`,
						`Плагины сообщества добавляются в <a href="${PLUGINS_REPO}" target="_blank" rel="noopener">Everelsu/Noctrinth-Plugins</a> как <strong>Git Submodule</strong> (пример — <a href="https://github.com/Everelsu/Noctrinth-Floating-Bars" target="_blank" rel="noopener">Noctrinth-Floating-Bars</a>). Так каждый плагин зафиксирован на проверенном коммите.`,
					)}</p>
					<ol>
						<li>${t('Push your plugin to a public GitHub repository.', 'Выгрузите плагин в публичный репозиторий на GitHub.')}</li>
						<li>${t(`Fork <a href="${PLUGINS_REPO}" target="_blank" rel="noopener">Everelsu/Noctrinth-Plugins</a> and add your repository as a submodule under <code>plugins/&lt;your-plugin-id&gt;</code>:`, `Сделайте форк <a href="${PLUGINS_REPO}" target="_blank" rel="noopener">Everelsu/Noctrinth-Plugins</a> и добавьте свой репозиторий как submodule в <code>plugins/&lt;your-plugin-id&gt;</code>:`)}</li>
					</ol>
<pre><code>git clone https://github.com/YOUR_USER/Noctrinth-Plugins.git
cd Noctrinth-Plugins
git submodule add https://github.com/YOUR_USER/your-plugin-repo.git plugins/your-plugin-id
git commit -m "Add your-plugin-id"
git push</code></pre>
					<p>${t(
						'Open a Pull Request. Once reviewed and merged, GitHub Actions automatically packages the archive, copies your local <code>icon</code> / <code>banner</code>, and publishes the updated catalog within ~30 seconds.',
						'Откройте Pull Request. После ревью и мержа GitHub Actions автоматически соберёт архив, скопирует локальные <code>icon</code> / <code>banner</code> и обновит каталог примерно за 30 секунд.',
					)}</p>
					<p>${t(
						'You can also put a one-click install link in your README or website: <code>noctrinth://plugin/install/your-plugin-id</code>.',
						'В свой README или на сайт можно добавить ссылку установки в один клик: <code>noctrinth://plugin/install/your-plugin-id</code>.',
					)}</p>
				</section>

				<section class="docs-section" id="updating">
					<h2>${t('7. How to update a published plugin', '7. Как выпускать обновления плагина')}</h2>
					<p>${t(
						'Because the catalog pins your plugin to a specific Git commit, pushing new commits to your own repository will <strong>not</strong> auto-update the catalog until you bump the submodule pointer in <code>Noctrinth-Plugins</code>:',
						'Поскольку каталог фиксирует плагин на конкретном коммите, пуш в ваш репозиторий <strong>не</strong> обновит каталог автоматически — нужно обновить указатель submodule в <code>Noctrinth-Plugins</code>:',
					)}</p>
					<ol>
						<li>${t(
							'Make your changes in your plugin repository and <strong>bump <code>version</code> in <code>plugin.json</code></strong> (for example <code>1.0.0</code> → <code>1.0.1</code>). Noctrinth compares <code>version</code> against the catalog to show the <strong>Update</strong> / <strong>Update all</strong> button.',
							'Внесите правки в своём репозитории и <strong>обязательно повысьте <code>version</code> в <code>plugin.json</code></strong> (например, <code>1.0.0</code> → <code>1.0.1</code>). Именно по смене <code>version</code> Noctrinth показывает пользователям кнопку <strong>Обновить</strong> и <strong>Обновить все</strong>.',
						)}</li>
						<li>${t('Commit and push your changes to your plugin repo.', 'Закоммитьте и запушьте изменения в свой репозиторий плагина.')}</li>
						<li>${t('Update the submodule commit in your fork of <code>Noctrinth-Plugins</code>:', 'Обновите коммит submodule в своём форке <code>Noctrinth-Plugins</code>:')}</li>
					</ol>
<pre><code>cd Noctrinth-Plugins
git pull upstream main
git submodule update --remote plugins/your-plugin-id
git add plugins/your-plugin-id
git commit -m "Update your-plugin-id to 1.0.1"
git push</code></pre>
					<ol start="4">
						<li>${t(
							'Open a Pull Request to <code>Everelsu/Noctrinth-Plugins</code> with a short summary of what changed. Once merged, every user with your plugin installed will see the update automatically.',
							'Откройте Pull Request в <code>Everelsu/Noctrinth-Plugins</code> с кратким описанием изменений. После мержа у всех пользователей с установленным плагином автоматически появится обновление.',
						)}</li>
					</ol>
				</section>

				<section class="docs-section" id="troubleshooting">
					<h2>${t('8. Troubleshooting', '8. Частые ошибки и отладка')}</h2>
					<ul>
						<li><strong>${t('“Missing plugin.json” on .zip install:', '«Missing plugin.json» при установке .zip:')}</strong> ${t('Make sure you zipped the files themselves, not the parent folder.', 'Убедитесь, что вы заархивировали сами файлы, а не родительскую папку целиком.')}</li>
						<li><strong>${t('Plugin auto-disabled after error:', 'Плагин отключился сам после ошибки:')}</strong> ${t('Check the red error banner on the plugin card and open the <code>&lt;/&gt;</code> Logs panel to see the exact exception or missing permission.', 'Посмотрите красную плашку на карточке плагина и откройте консоль логов <code>&lt;/&gt;</code> — там будет точная ошибка или недостающее право.')}</li>
						<li><strong>${t('CSS theme broke the interface:', 'CSS-тема сломала интерфейс:')}</strong> ${t('Press <code>Ctrl+Shift+Alt+P</code> (or launch with <code>--safe-mode</code>) to pause all plugins and Quick CSS immediately.', 'Нажмите <code>Ctrl+Shift+Alt+P</code> (или запустите с <code>--safe-mode</code>) — все плагины и Quick CSS мгновенно отключатся.')}</li>
						<li><strong>${t('Update didn’t appear for users:', 'Обновление не появилось у пользователей:')}</strong> ${t('Verify that <code>version</code> in <code>plugin.json</code> changed and that the submodule PR in <code>Noctrinth-Plugins</code> was merged.', 'Проверьте, что вы повысили <code>version</code> в <code>plugin.json</code> и что PR с обновлением submodule в <code>Noctrinth-Plugins</code> был принят.')}</li>
					</ul>
				</section>
			</div>
		</main>`,
	})
}
