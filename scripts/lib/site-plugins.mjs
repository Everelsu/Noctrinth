// The Plugins catalog page (/plugins/): live search and filter over
// https://everelsu.github.io/Noctrinth-Plugins/index.json with one-click
// noctrinth://plugin/install/<id> deep links.

import { CATALOG_URL, icon, page, PLUGINS_REPO, TEMPLATE_REPO, t } from './site-style.mjs'

const PLUGINS_CSS = `
	.plugins-hero {
		padding: 2.75rem 0 2.25rem;
		border-bottom: 1px solid var(--brand-gradient-border);
		background: var(--brand-gradient-bg);
	}
	.plugins-hero__row {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-end;
		justify-content: space-between;
		gap: 1.25rem;
	}
	.plugins-hero h1 {
		margin: 0;
		font-size: clamp(1.85rem, 3.8vw, 2.5rem);
		font-weight: 800;
		letter-spacing: -0.025em;
		color: var(--text-contrast);
	}
	.plugins-hero p {
		margin: 0.5rem 0 0;
		max-width: 40rem;
		color: var(--text);
		font-size: 1.02rem;
	}
	.plugins-hero__actions { display: flex; flex-wrap: wrap; gap: 0.6rem; }

	.plugins-toolbar {
		margin-top: 2rem;
		display: flex;
		flex-direction: column;
		gap: 0.9rem;
	}
	.search-box {
		position: relative;
		display: flex;
		align-items: center;
	}
	.search-box svg {
		position: absolute;
		left: 1rem;
		width: 1.1rem;
		height: 1.1rem;
		color: var(--text-secondary);
		pointer-events: none;
	}
	.search-box input {
		width: 100%;
		padding: 0.78rem 1rem 0.78rem 2.75rem;
		border-radius: var(--radius-lg);
		border: 1px solid var(--button-border);
		background: var(--surface-2);
		color: var(--text-contrast);
		font: inherit;
		outline: none;
		transition: border-color 0.15s ease, box-shadow 0.15s ease;
	}
	.search-box input::placeholder { color: var(--text-secondary); }
	.search-box input:focus {
		border-color: var(--brand);
		box-shadow: 0 0 0 3px rgba(192, 132, 252, 0.18);
	}

	.filter-row {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: 0.75rem;
	}
	.filter-pills { display: flex; flex-wrap: wrap; gap: 0.45rem; }
	.filter-pills button {
		min-height: 2rem;
		padding: 0.3rem 0.85rem;
		font-size: 0.875rem;
	}
	.filter-pills button[aria-pressed="true"] {
		background: var(--brand-highlight);
		color: var(--brand);
		border-color: rgba(192, 132, 252, 0.4);
	}
	.plugins-count { color: var(--text-secondary); font-size: 0.88rem; }

	.plugins-grid {
		margin-top: 1.5rem;
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
		gap: 1.15rem;
	}
	.plugin-card--cta {
		justify-content: center;
		align-items: flex-start;
		border-style: dashed;
		background: color-mix(in srgb, var(--surface-2) 60%, transparent);
	}
	.plugin-card--cta h3 { margin: 0; font-size: 1.08rem; color: var(--text-contrast); }
	.plugin-card--cta p { margin: 0; font-size: 0.92rem; color: var(--text-secondary); line-height: 1.5; }
	.plugin-card {
		display: flex;
		flex-direction: column;
		gap: 0.9rem;
		padding: 1.25rem;
		border-radius: var(--radius-xl);
		background: var(--surface-2);
		border: 1px solid var(--button-border);
		transition: border-color 0.15s ease, transform 0.15s ease, box-shadow 0.15s ease;
	}
	.plugin-card__banner {
		width: calc(100% + 2.5rem);
		max-width: none;
		margin: -1.25rem -1.25rem 0.15rem;
		aspect-ratio: 16 / 9;
		object-fit: cover;
		border-radius: var(--radius-xl) var(--radius-xl) 0 0;
		border-bottom: 1px solid var(--button-border);
		display: block;
	}
	.plugin-card:hover {
		border-color: rgba(192, 132, 252, 0.35);
		transform: translateY(-2px);
		box-shadow: 0 12px 30px -14px rgba(0, 0, 0, 0.65);
	}
	.plugin-card__head {
		display: flex;
		align-items: flex-start;
		gap: 0.85rem;
	}
	.plugin-card__icon {
		width: 44px;
		height: 44px;
		border-radius: 0.85rem;
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		font-weight: 800;
		overflow: hidden;
	}
	.plugin-card__icon img { width: 100%; height: 100%; object-fit: cover; }
	.plugin-card__icon--theme { background: rgba(192, 132, 252, 0.16); color: #d8b4fe; }
	.plugin-card__icon--content-source { background: rgba(96, 165, 250, 0.16); color: #93c5fd; }
	.plugin-card__icon--extension { background: rgba(74, 222, 128, 0.16); color: #86efac; }
	.plugin-card__title-wrap { min-width: 0; flex: 1; }
	.plugin-card__title-row { display: flex; flex-wrap: wrap; align-items: center; gap: 0.45rem; }
	.plugin-card__title {
		margin: 0;
		font-size: 1.08rem;
		font-weight: 700;
		color: var(--text-contrast);
	}
	.badge {
		display: inline-flex;
		align-items: center;
		padding: 0.12rem 0.5rem;
		border-radius: 6px;
		font-size: 0.74rem;
		font-weight: 600;
	}
	.badge--version {
		background: var(--surface-3);
		color: var(--text-secondary);
		font-family: 'JetBrains Mono', ui-monospace, monospace;
	}
	.badge--theme { background: rgba(192, 132, 252, 0.16); color: #d8b4fe; }
	.badge--content-source { background: rgba(96, 165, 250, 0.16); color: #93c5fd; }
	.badge--extension { background: rgba(74, 222, 128, 0.16); color: #86efac; }
	.plugin-card__author { font-size: 0.84rem; color: var(--text-secondary); margin-top: 0.15rem; }
	.plugin-card__desc { margin: 0; font-size: 0.92rem; line-height: 1.5; color: var(--text); }
	.plugin-card__tags { display: flex; flex-wrap: wrap; gap: 0.35rem; }
	.plugin-card__tag {
		padding: 0.12rem 0.5rem;
		border-radius: 6px;
		background: var(--surface-3);
		color: var(--text-secondary);
		font-size: 0.76rem;
	}
	.plugin-card__actions {
		margin-top: auto;
		padding-top: 0.75rem;
		border-top: 1px solid var(--divider);
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.45rem;
	}
	.plugin-card__actions .btn { min-height: 2rem; padding: 0.35rem 0.78rem; font-size: 0.82rem; }
	.plugin-card__actions .btn--brand { flex: 1 1 auto; }

	.plugins-state {
		margin-top: 2rem;
		padding: 2.5rem 1.5rem;
		text-align: center;
		border-radius: var(--radius-xl);
		border: 1px dashed var(--button-border);
		background: var(--surface-2);
		color: var(--text-secondary);
	}
	.plugins-disclaimer {
		margin-top: 1.5rem;
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 0.75rem;
	}
	.plugins-disclaimer[hidden] { display: none; }
	.plugins-disclaimer__close {
		flex-shrink: 0;
		width: 1.75rem;
		height: 1.75rem;
		border-radius: 999px;
		border: 1px solid rgba(251, 191, 36, 0.28);
		background: transparent;
		color: #fde68a;
		font: inherit;
		font-size: 1.05rem;
		line-height: 1;
		cursor: pointer;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		transition: background 0.15s ease;
	}
	.plugins-disclaimer__close:hover { background: rgba(251, 191, 36, 0.14); }
`

const PLUGINS_SCRIPT = `
	(function () {
		var CATALOG = ${JSON.stringify(CATALOG_URL)};
		var grid = document.getElementById('plugins-grid');
		var stateBox = document.getElementById('plugins-state');
		var countEl = document.getElementById('plugins-count');
		var searchInput = document.getElementById('plugins-search');
		var disclaimerEl = document.getElementById('plugins-disclaimer');
		var disclaimerClose = document.getElementById('plugins-disclaimer-close');
		var pills = document.querySelectorAll('[data-type-filter]');
		var allPlugins = [];
		var activeType = 'all';

		if (disclaimerEl) {
			try {
				if (localStorage.getItem('noctrinth-plugins-disclaimer-dismissed') === '1') {
					disclaimerEl.hidden = true;
				}
			} catch (e) {}
			if (disclaimerClose) {
				disclaimerClose.addEventListener('click', function () {
					disclaimerEl.hidden = true;
					try { localStorage.setItem('noctrinth-plugins-disclaimer-dismissed', '1'); } catch (e) {}
				});
			}
		}

		function isRu() {
			return document.documentElement.getAttribute('data-lang') === 'ru';
		}

		function esc(s) {
			return String(s || '')
				.replace(/&/g, '&amp;')
				.replace(/</g, '&lt;')
				.replace(/>/g, '&gt;')
				.replace(/"/g, '&quot;');
		}

		function typeLabel(type) {
			if (type === 'theme') return isRu() ? 'Тема' : 'Theme';
			if (type === 'content-source') return isRu() ? 'Источник' : 'Content source';
			return isRu() ? 'Расширение' : 'Extension';
		}

		function typeSvg(type) {
			if (type === 'theme') {
				return '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18.37 2.63 14 7l-1.59-1.59a2 2 0 0 0-2.82 0L8 7l9 9 1.59-1.59a2 2 0 0 0 0-2.82L17 10l4.37-4.37a2.12 2.12 0 1 0-3-3Z"/><path d="M9 8c-2 3-4 3.5-7 4l8 10c2-1 6-5 6-7"/><path d="M14.5 17.5 4.5 15"/></svg>';
			}
			if (type === 'content-source') {
				return '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20"/><path d="M2 12h20"/></svg>';
			}
			return '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22v-5"/><path d="M9 8V2"/><path d="M15 8V2"/><path d="M18 8v5a4 4 0 0 1-4 4h-4a4 4 0 0 1-4-4V8Z"/></svg>';
		}

		function render() {
			searchInput.placeholder = isRu()
				? 'Поиск по плагинам, темам, авторам, тегам...'
				: 'Search plugins, themes, authors, tags...';
			var q = (searchInput.value || '').trim().toLowerCase();
			var list = allPlugins.filter(function (p) {
				if (activeType !== 'all' && p.type !== activeType) return false;
				if (!q) return true;
				var hay = [p.name, p.id, p.description, p.author, (p.tags || []).join(' ')].join(' ').toLowerCase();
				return hay.indexOf(q) !== -1;
			});

			countEl.textContent = isRu()
				? ('Показано: ' + list.length)
				: ('Showing ' + list.length + (list.length === 1 ? ' plugin' : ' plugins'));

			if (!list.length) {
				grid.innerHTML = '';
				stateBox.hidden = false;
				stateBox.textContent = isRu()
					? 'По вашему запросу плагинов не найдено.'
					: 'No plugins match your search.';
				return;
			}

			stateBox.hidden = true;
			var installLabel = isRu() ? 'Установить' : 'Install';
			var zipLabel = '.zip';
			var repoLabel = 'GitHub';

			grid.innerHTML = list.map(function (p) {
				var bannerHtml = p.banner_url
					? '<img class="plugin-card__banner" src="' + esc(p.banner_url) + '" alt="" loading="lazy">'
					: '';
				var iconHtml = p.icon_url
					? '<img src="' + esc(p.icon_url) + '" alt="" loading="lazy">'
					: typeSvg(p.type);
				var tagsHtml = (p.tags && p.tags.length)
					? '<div class="plugin-card__tags">' + p.tags.map(function (tag) {
						return '<span class="plugin-card__tag">#' + esc(tag) + '</span>';
					}).join('') + '</div>'
					: '';
				var linkUrl = p.homepage || p.repository || '';
				var repoBtn = linkUrl
					? '<a class="btn" href="' + esc(linkUrl) + '" target="_blank" rel="noopener">' + repoLabel + '</a>'
					: '';
				return '<article class="plugin-card">' +
					bannerHtml +
					'<div class="plugin-card__head">' +
						'<div class="plugin-card__icon plugin-card__icon--' + esc(p.type) + '">' + iconHtml + '</div>' +
						'<div class="plugin-card__title-wrap">' +
							'<div class="plugin-card__title-row">' +
								'<h3 class="plugin-card__title">' + esc(p.name) + '</h3>' +
								'<span class="badge badge--version">v' + esc(p.version) + '</span>' +
								'<span class="badge badge--' + esc(p.type) + '">' + esc(typeLabel(p.type)) + '</span>' +
							'</div>' +
							(p.author ? '<div class="plugin-card__author">' + esc(p.author) + '</div>' : '') +
						'</div>' +
					'</div>' +
					(p.description ? '<p class="plugin-card__desc">' + esc(p.description) + '</p>' : '') +
					tagsHtml +
					'<div class="plugin-card__actions">' +
						'<a class="btn btn--brand" href="noctrinth://plugin/install/' + encodeURIComponent(p.id) + '">' + installLabel + '</a>' +
						'<a class="btn" href="' + esc(p.download_url) + '" rel="noopener">' + zipLabel + '</a>' +
						repoBtn +
					'</div>' +
				'</article>';
			}).join('') +
			(!q && activeType === 'all'
				? '<article class="plugin-card plugin-card--cta">' +
					'<h3>' + (isRu() ? 'Создай свой плагин или тему' : 'Build your own plugin or theme') + '</h3>' +
					'<p>' + (isRu()
						? 'Начни с готового шаблона и отправь Pull Request в каталог — твой плагин появится здесь автоматически.'
						: 'Start from the official template and open a Pull Request to the catalog — your plugin will appear here automatically.') + '</p>' +
					'<div class="plugin-card__actions" style="border-top:0;padding-top:0.25rem;width:100%">' +
						'<a class="btn" href="../docs/">' + (isRu() ? 'Документация' : 'Read Docs') + '</a>' +
						'<a class="btn" href="${TEMPLATE_REPO}" target="_blank" rel="noopener">' + (isRu() ? 'Шаблон' : 'Template') + '</a>' +
					'</div>' +
				'</article>'
				: '');
		}

		for (var i = 0; i < pills.length; i++) {
			pills[i].addEventListener('click', function () {
				activeType = this.getAttribute('data-type-filter');
				for (var j = 0; j < pills.length; j++) {
					pills[j].setAttribute('aria-pressed', String(pills[j] === this));
				}
				render();
			});
		}

		searchInput.addEventListener('input', render);
		new MutationObserver(render).observe(document.documentElement, {
			attributes: true,
			attributeFilter: ['data-lang'],
		});

		fetch(CATALOG)
			.then(function (r) {
				if (!r.ok) throw new Error('HTTP ' + r.status);
				return r.json();
			})
			.then(function (data) {
				allPlugins = Array.isArray(data.plugins) ? data.plugins : [];
				render();
			})
			.catch(function () {
				stateBox.hidden = false;
				stateBox.textContent = isRu()
					? 'Не удалось загрузить каталог плагинов. Попробуйте позже или откройте GitHub.'
					: 'Could not load the plugin catalog. Try again later or open GitHub.';
			});
	})();
`

export function renderPluginsPage() {
	return page({
		title: 'Plugins — Noctrinth',
		description:
			'Browse community themes, content sources, and sandboxed extensions for Noctrinth Launcher.',
		prefix: '../',
		current: 'plugins',
		css: PLUGINS_CSS,
		body: `
		<header class="plugins-hero">
			<div class="container container--wide plugins-hero__row">
				<div>
					<h1>${t('Plugin Catalog', 'Каталог плагинов')}</h1>
					<p>${t(
						'Community themes, Discover content sources, and extensions. Click Install to open directly in Noctrinth.',
						'Темы оформления, дополнительные источники для «Исследования» и расширения. Кнопка «Установить» открывает плагин прямо в Noctrinth.',
					)}</p>
				</div>
				<div class="plugins-hero__actions">
					<a class="btn btn--brand" href="../docs/">${icon('book')} ${t('Plugin Docs', 'Документация')}</a>
					<a class="btn" href="${PLUGINS_REPO}" target="_blank" rel="noopener">${icon('github')} ${t('Submit a plugin', 'Добавить плагин')}</a>
					<a class="btn" href="${TEMPLATE_REPO}" target="_blank" rel="noopener">${icon('external')} ${t('Template', 'Шаблон')}</a>
					<button type="button" class="btn btn--disabled" disabled aria-disabled="true" title="Discord server coming soon">${icon('discord')} Discord <span class="btn__tag">${t('Soon', 'Скоро')}</span></button>
				</div>
			</div>
		</header>

		<main class="container container--wide">
			<div id="plugins-disclaimer" class="callout callout--warn plugins-disclaimer">
				<div>
					<strong>${t('Third-party plugins:', 'Сторонние плагины:')}</strong>
					${t(
						' Plugins are created by independent authors. Noctrinth is not responsible for the safety, reliability, data loss, or malicious code in installed plugins. Only install plugins from sources you trust.',
						' Плагины создаются сторонними авторами. Noctrinth не несёт ответственности за безопасность, стабильность, потерю данных или вредоносный код в установленных плагинах. Устанавливайте плагины только из источников, которым доверяете.',
					)}
				</div>
				<button id="plugins-disclaimer-close" type="button" class="plugins-disclaimer__close" aria-label="Dismiss">&times;</button>
			</div>

			<div class="plugins-toolbar">
				<div class="search-box">
					${icon('search')}
					<input id="plugins-search" type="search" placeholder="Search plugins, themes, authors, tags..." aria-label="Search plugins">
				</div>
				<div class="filter-row">
					<div class="filter-pills">
						<button class="btn" type="button" data-type-filter="all" aria-pressed="true">${t('All', 'Все')}</button>
						<button class="btn" type="button" data-type-filter="theme" aria-pressed="false">${t('Themes', 'Темы')}</button>
						<button class="btn" type="button" data-type-filter="content-source" aria-pressed="false">${t('Content sources', 'Источники')}</button>
						<button class="btn" type="button" data-type-filter="extension" aria-pressed="false">${t('Extensions', 'Расширения')}</button>
					</div>
					<span id="plugins-count" class="plugins-count"></span>
				</div>
			</div>

			<div id="plugins-state" class="plugins-state">${t('Loading catalog...', 'Загрузка каталога...')}</div>
			<div id="plugins-grid" class="plugins-grid"></div>
		</main>`,
		scripts: `<script>${PLUGINS_SCRIPT}</script>`,
	})
}
