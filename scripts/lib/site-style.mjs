// The site's look, aligned with Noctrinth's dark obsidian + amethyst palette,
// and the shared page frame: sticky glass navigation bar, footer, and EN/RU toggle.

export const REPO = 'https://github.com/Everelsu/Noctrinth'
export const PLUGINS_REPO = 'https://github.com/Everelsu/Noctrinth-Plugins'
export const TEMPLATE_REPO = 'https://github.com/Everelsu/Noctrinth-Plugin-Template'
export const CATALOG_URL = 'https://everelsu.github.io/Noctrinth-Plugins/index.json'
/** Where the site is published, for the links that have to be absolute (the news feed). */
export const SITE_URL = 'https://everelsu.github.io/Noctrinth'

/** One string in both languages; the stylesheet shows the one the page is in. */
export function t(en, ru) {
	return `<span class="i18n" lang="en">${en}</span><span class="i18n" lang="ru">${ru}</span>`
}

const LANG_BOOT = `
	(function () {
		var saved = null;
		try { saved = localStorage.getItem('noctrinth-lang'); } catch (e) {}
		var lang = saved || ((navigator.language || '').toLowerCase().indexOf('ru') === 0 ? 'ru' : 'en');
		document.documentElement.setAttribute('data-lang', lang);
		document.documentElement.lang = lang;
	})();
`

const LANG_TOGGLE = `
	(function () {
		var buttons = document.querySelectorAll('[data-lang-toggle]');
		for (var i = 0; i < buttons.length; i++) {
			buttons[i].addEventListener('click', function () {
				var next = document.documentElement.getAttribute('data-lang') === 'ru' ? 'en' : 'ru';
				document.documentElement.setAttribute('data-lang', next);
				document.documentElement.lang = next;
				try { localStorage.setItem('noctrinth-lang', next); } catch (e) {}
			});
		}
	})();
`

/** Dates written in the page's language, from \`<time datetime>\` elements. */
const DATE_SCRIPT = `
	(function () {
		var lang = document.documentElement.getAttribute('data-lang') === 'ru' ? 'ru-RU' : 'en-US';
		var fmt = new Intl.DateTimeFormat(lang, { year: 'numeric', month: 'long', day: 'numeric' });
		function update() {
			lang = document.documentElement.getAttribute('data-lang') === 'ru' ? 'ru-RU' : 'en-US';
			fmt = new Intl.DateTimeFormat(lang, { year: 'numeric', month: 'long', day: 'numeric' });
			var nodes = document.querySelectorAll('time[datetime][data-date]');
			for (var i = 0; i < nodes.length; i++) {
				var d = new Date(nodes[i].getAttribute('datetime'));
				if (!isNaN(d.getTime())) nodes[i].textContent = fmt.format(d);
			}
		}
		update();
		new MutationObserver(update).observe(document.documentElement, { attributes: true, attributeFilter: ['data-lang'] });
	})();
`

export const BASE_CSS = `
	:root {
		--surface-1: #111017;
		--surface-2: #181622;
		--surface-3: #221f30;
		--surface-4: #2d2940;
		--surface-5: #3d3854;
		--bg: var(--surface-1);
		--bg-raised: var(--surface-3);
		--button-bg: var(--surface-3);
		--button-border: rgba(205, 190, 240, 0.12);
		--divider: rgba(205, 190, 240, 0.1);
		--text: #b8b3cc;
		--text-contrast: #f8f7fc;
		--text-secondary: #8e88a6;
		--brand: #c084fc;
		--brand-strong: #a855f7;
		--brand-highlight: rgba(192, 132, 252, 0.18);
		--brand-gradient-bg: radial-gradient(900px 420px at 50% -10%, rgba(168, 85, 247, 0.2), transparent 70%), linear-gradient(180deg, rgba(35, 18, 58, 0.35) 0%, rgba(17, 16, 23, 0) 100%);
		--brand-gradient-border: rgba(192, 132, 252, 0.14);
		--link: #c792ff;
		--radius-md: 0.75rem;
		--radius-lg: 1rem;
		--radius-xl: 1.25rem;
	}

	* { box-sizing: border-box; }
	html { scroll-behavior: smooth; }
	html, body {
		margin: 0;
		background: var(--bg);
		color: var(--text);
		font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
		font-size: 16px;
		line-height: 1.55;
		-webkit-font-smoothing: antialiased;
	}
	html[data-lang="en"] .i18n[lang="ru"],
	html[data-lang="ru"] .i18n[lang="en"] { display: none; }
	a { color: var(--link); text-decoration: none; }
	a:hover { text-decoration: underline; }
	img { max-width: 100%; }
	code {
		background: var(--surface-2);
		border: 1px solid var(--button-border);
		padding: 0.08rem 0.38rem;
		border-radius: 6px;
		font-size: 0.86em;
		color: #e9d5ff;
		font-family: 'JetBrains Mono', ui-monospace, Menlo, monospace;
	}
	pre {
		margin: 1rem 0;
		padding: 1rem 1.15rem;
		border-radius: var(--radius-lg);
		background: #0c0b12;
		border: 1px solid var(--button-border);
		overflow-x: auto;
		line-height: 1.55;
	}
	pre code {
		background: none;
		border: 0;
		padding: 0;
		color: #e4dff2;
		font-size: 0.85rem;
	}

	.container { max-width: 58rem; margin-inline: auto; padding-inline: 1.5rem; }
	.container--wide { max-width: 74rem; }

	.nav {
		position: sticky;
		top: 0;
		z-index: 30;
		background: color-mix(in srgb, var(--surface-2) 84%, transparent);
		backdrop-filter: blur(16px);
		-webkit-backdrop-filter: blur(16px);
		border-bottom: 1px solid var(--divider);
	}
	.nav__inner { display: flex; align-items: center; gap: 1.25rem; height: 4rem; }
	.nav__brand {
		display: inline-flex;
		align-items: center;
		gap: 0.6rem;
		min-height: 2.25rem;
		padding: 0.25rem 0.35rem;
		margin-left: -0.35rem;
		border-radius: 0.75rem;
		color: var(--text-contrast);
		font-weight: 800;
		font-size: 1.12rem;
		letter-spacing: -0.02em;
		flex-shrink: 0;
		transition: background 0.15s ease;
	}
	.nav__brand:hover { background: var(--surface-3); text-decoration: none; }
	.nav__logo {
		width: 28px;
		height: 28px;
		flex-shrink: 0;
		display: block;
	}
	.nav__links {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		margin-right: auto;
		min-width: 0;
		overflow-x: auto;
		scrollbar-width: none;
	}
	.nav__links::-webkit-scrollbar { display: none; }
	.nav__links a {
		color: var(--text);
		font-weight: 600;
		font-size: 0.94rem;
		padding: 0.45rem 0.85rem;
		border-radius: 999px;
		white-space: nowrap;
		transition: background 0.15s ease, color 0.15s ease;
	}
	.nav__links a:hover { background: var(--surface-3); color: var(--text-contrast); text-decoration: none; }
	.nav__links a[aria-current="page"] { background: var(--brand-highlight); color: var(--brand); }
	.nav__actions { display: flex; gap: 0.5rem; align-items: center; flex-shrink: 0; }

	.btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		min-height: 2.25rem;
		padding: 0.5rem 1rem;
		border-radius: 999px;
		border: 1px solid var(--button-border);
		background: var(--button-bg);
		color: var(--text-contrast);
		font: inherit;
		font-size: 0.93rem;
		font-weight: 600;
		cursor: pointer;
		transition: background 0.15s ease, border-color 0.15s ease, filter 0.15s ease, transform 0.1s ease;
	}
	.btn:hover { filter: brightness(1.14); border-color: rgba(192, 132, 252, 0.35); text-decoration: none; }
	.btn:active { transform: scale(0.97); }
	.btn--brand {
		background: linear-gradient(180deg, #cb97ff 0%, #b370fa 100%);
		border-color: transparent;
		color: #160c26;
	}
	.btn--brand:hover { filter: brightness(1.07); border-color: transparent; }
	.btn--xl { min-height: 3.1rem; padding: 0.75rem 1.65rem; font-size: 1.05rem; }
	.btn--disabled,
	.btn[disabled] {
		opacity: 0.55;
		cursor: not-allowed;
		border-style: dashed;
	}
	.btn--disabled:hover,
	.btn[disabled]:hover { filter: none; border-color: var(--button-border); }
	.btn--disabled:active,
	.btn[disabled]:active { transform: none; }
	.btn__tag {
		padding: 0.08rem 0.42rem;
		border-radius: 999px;
		background: var(--surface-4);
		color: var(--text-secondary);
		font-size: 0.72rem;
		font-weight: 700;
		letter-spacing: 0.02em;
	}
	.btn svg { width: 1.1em; height: 1.1em; flex-shrink: 0; }

	.page-title { margin: 2rem 0 0; font-size: 1.875rem; font-weight: 700; color: var(--text-contrast); letter-spacing: -0.02em; }
	.section-heading { margin: 0 0 1rem; font-size: 1.35rem; font-weight: 800; color: var(--text-contrast); letter-spacing: -0.02em; }

	.full-width-bg { background: var(--brand-gradient-bg); border-block: 1px solid var(--brand-gradient-border); }

	.callout {
		padding: 1rem 1.2rem;
		border-radius: var(--radius-lg);
		border: 1px solid var(--button-border);
		background: var(--surface-2);
		font-size: 0.94rem;
		line-height: 1.6;
	}
	.callout strong { color: var(--text-contrast); }
	.callout--warn {
		border-color: rgba(251, 191, 36, 0.28);
		background: rgba(251, 191, 36, 0.07);
	}
	.callout--warn strong { color: #fde68a; }

	.footer {
		margin-top: 5rem;
		padding: 2.25rem 0 3rem;
		border-top: 1px solid var(--divider);
		color: var(--text-secondary);
		font-size: 0.9rem;
	}
	.footer__row { display: flex; flex-wrap: wrap; gap: 0.75rem 1.5rem; align-items: center; justify-content: space-between; }
	.footer a { color: var(--text); }
	.footer a:hover { color: var(--text-contrast); }

	@media (max-width: 700px) {
		.container { padding-inline: 1rem; }
		.nav__inner { gap: 0.6rem; height: 3.75rem; }
		.nav__brand { padding: 0.35rem; margin-left: -0.25rem; }
		.nav__brand-name { display: none; }
		.nav__links a { padding: 0.38rem 0.65rem; font-size: 0.88rem; }
		.nav__actions .btn { min-height: 2rem; padding: 0.35rem 0.7rem; font-size: 0.85rem; }
	}

	@keyframes nm-hero-in {
		from { opacity: 0; transform: translateY(14px); }
		to { opacity: 1; transform: translateY(0); }
	}
	.hero__body, .plugins-hero h1, .plugins-hero p, .docs-hero h1, .docs-hero p {
		animation: nm-hero-in 0.55s cubic-bezier(0.22, 1, 0.36, 1) both;
	}
	.reveal {
		opacity: 0;
		transform: translateY(18px);
		transition: opacity 0.55s cubic-bezier(0.22, 1, 0.36, 1), transform 0.55s cubic-bezier(0.22, 1, 0.36, 1);
	}
	.reveal.is-visible {
		opacity: 1;
		transform: translateY(0);
	}
	.feature, .bento-card, .plugin-card {
		position: relative;
		overflow: hidden;
	}
	.feature::after, .bento-card::after, .plugin-card::after {
		content: '';
		position: absolute;
		inset: 0;
		pointer-events: none;
		opacity: 0;
		transition: opacity 0.25s ease;
		background: radial-gradient(460px circle at var(--mx, 50%) var(--my, 50%), rgba(192, 132, 252, 0.11), transparent 65%);
	}
	.feature:hover::after, .bento-card:hover::after, .plugin-card:hover::after { opacity: 1; }
	.feature__media img, .plugin-card__banner, .news-card img {
		transition: transform 0.45s cubic-bezier(0.22, 1, 0.36, 1);
	}
	.feature:hover .feature__media img, .bento-card:hover .feature__media img, .plugin-card:hover .plugin-card__banner, .news-card:hover img {
		transform: scale(1.03);
	}
	.chips li {
		transition: transform 0.2s ease, border-color 0.2s ease, background 0.2s ease;
	}
	.chips li:hover {
		transform: translateY(-2px);
		border-color: rgba(192, 132, 252, 0.35);
		background: var(--surface-4);
	}
	.faq details {
		transition: border-color 0.2s ease, background 0.2s ease;
	}
	.faq details[open] {
		border-color: rgba(192, 132, 252, 0.32);
		background: color-mix(in srgb, var(--surface-2) 88%, var(--brand) 12%);
	}

	@media (prefers-reduced-motion: reduce) {
		.hero__body, .plugins-hero h1, .plugins-hero p, .docs-hero h1, .docs-hero p { animation: none; }
		.reveal { opacity: 1; transform: none; transition: none; }
		.feature__media img, .plugin-card__banner, .news-card img, .chips li { transition: none; transform: none !important; }
	}
`

const MOTION_SCRIPT = `
	(function () {
		if (window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
		var items = document.querySelectorAll('.feature, .bento-card, .news-card, .faq details, .docs-section, .entry');
		if ('IntersectionObserver' in window) {
			var io = new IntersectionObserver(function (entries) {
				for (var i = 0; i < entries.length; i++) {
					if (entries[i].isIntersecting) {
						entries[i].target.classList.add('is-visible');
						io.unobserve(entries[i].target);
					}
				}
			}, { rootMargin: '0px 0px -40px 0px', threshold: 0.05 });
			for (var i = 0; i < items.length; i++) {
				items[i].classList.add('reveal');
				io.observe(items[i]);
			}
		}
		document.addEventListener('pointermove', function (e) {
			var card = e.target && e.target.closest ? e.target.closest('.feature, .bento-card, .plugin-card') : null;
			if (!card) return;
			var rect = card.getBoundingClientRect();
			card.style.setProperty('--mx', (e.clientX - rect.left) + 'px');
			card.style.setProperty('--my', (e.clientY - rect.top) + 'px');
		}, { passive: true });
	})();
`

const ICONS = {
	github:
		'<svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M12 .5a11.5 11.5 0 0 0-3.64 22.41c.58.1.79-.25.79-.56v-2c-3.2.7-3.88-1.38-3.88-1.38-.53-1.33-1.28-1.69-1.28-1.69-1.05-.71.08-.7.08-.7 1.16.08 1.77 1.19 1.77 1.19 1.03 1.77 2.71 1.26 3.37.96.1-.75.4-1.26.73-1.55-2.55-.29-5.24-1.28-5.24-5.69 0-1.26.45-2.29 1.19-3.09-.12-.29-.52-1.46.11-3.05 0 0 .97-.31 3.17 1.18a11 11 0 0 1 5.77 0c2.2-1.49 3.17-1.18 3.17-1.18.63 1.59.23 2.76.11 3.05.74.8 1.19 1.83 1.19 3.09 0 4.42-2.7 5.39-5.26 5.68.41.36.78 1.06.78 2.14v3.17c0 .31.21.67.8.56A11.5 11.5 0 0 0 12 .5Z"/></svg>',
	download:
		'<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><path d="m7 10 5 5 5-5"/><path d="M12 15V3"/></svg>',
	rss: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 11a9 9 0 0 1 9 9"/><path d="M4 4a16 16 0 0 1 16 16"/><circle cx="5" cy="19" r="1"/></svg>',
	changelog:
		'<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="5" cy="6" r="3"/><path d="M5 9v6"/><circle cx="5" cy="18" r="3"/><path d="M12 3v18"/><circle cx="19" cy="6" r="3"/><path d="M16 15.7A9 9 0 0 0 19 9"/></svg>',
	chevron:
		'<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m9 18 6-6-6-6"/></svg>',
	plug: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 22v-5"/><path d="M9 8V2"/><path d="M15 8V2"/><path d="M18 8v5a4 4 0 0 1-4 4h-4a4 4 0 0 1-4-4V8Z"/></svg>',
	book: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 19.5v-15A2.5 2.5 0 0 1 6.5 2H20v20H6.5a2.5 2.5 0 0 1 0-5H20"/></svg>',
	external:
		'<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M15 3h6v6"/><path d="M10 14 21 3"/><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/></svg>',
	search:
		'<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/></svg>',
	discord:
		'<svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M20.32 4.37a19.8 19.8 0 0 0-4.89-1.52.07.07 0 0 0-.08.04c-.21.38-.45.87-.61 1.25a18.27 18.27 0 0 0-5.49 0 12.64 12.64 0 0 0-.62-1.25.08.08 0 0 0-.08-.04A19.74 19.74 0 0 0 3.67 4.37a.07.07 0 0 0-.03.03C.53 9.05-.32 13.58.1 18.06a.08.08 0 0 0 .03.06 19.9 19.9 0 0 0 5.99 3.03.08.08 0 0 0 .08-.03c.46-.63.87-1.3 1.22-2a.08.08 0 0 0-.04-.11 13.1 13.1 0 0 1-1.87-.89.08.08 0 0 1-.01-.13c.13-.09.25-.19.37-.29a.07.07 0 0 1 .08-.01c3.93 1.79 8.18 1.79 12.06 0a.07.07 0 0 1 .08.01c.12.1.25.2.37.29a.08.08 0 0 1-.01.13c-.6.35-1.22.65-1.87.89a.08.08 0 0 0-.04.11c.36.7.77 1.37 1.22 2a.08.08 0 0 0 .08.03 19.83 19.83 0 0 0 6-3.03.08.08 0 0 0 .03-.06c.5-5.18-.84-9.68-3.55-13.66a.06.06 0 0 0-.03-.03ZM8.02 15.33c-1.18 0-2.16-1.08-2.16-2.42 0-1.33.96-2.42 2.16-2.42 1.21 0 2.18 1.1 2.16 2.42 0 1.34-.96 2.42-2.16 2.42Zm7.97 0c-1.18 0-2.16-1.08-2.16-2.42 0-1.33.96-2.42 2.16-2.42 1.21 0 2.18 1.1 2.16 2.42 0 1.34-.95 2.42-2.16 2.42Z"/></svg>',
}

export function icon(name) {
	return ICONS[name]
}

/**
 * A whole page: head, the bar along the top, `body`, the footer.
 *
 * `prefix` reaches the site root from this page, since GitHub Pages serves the
 * site under `/<repo>/` and root-relative links would leave it.
 */
export function page({
	title,
	description,
	prefix = '',
	current,
	css = '',
	head = '',
	body,
	scripts = '',
}) {
	const link = (href, label, key) =>
		`<a href="${prefix}${href}"${current === key ? ' aria-current="page"' : ''}>${label}</a>`
	const desc =
		description ??
		'A Modrinth App fork with Ely.by and offline accounts, CurseForge packs, plugins, themes, and skins for everyone on offline servers.'

	return `<!DOCTYPE html>
<html lang="en" data-lang="en">
<head>
	<meta charset="UTF-8">
	<meta name="viewport" content="width=device-width, initial-scale=1">
	<title>${title}</title>
	<meta name="description" content="${desc}">
	<meta property="og:title" content="${title}">
	<meta property="og:description" content="${desc}">
	<meta property="og:image" content="${SITE_URL}/assets/social-preview.png">
	<meta name="twitter:card" content="summary_large_image">
	<meta name="google-site-verification" content="ayBhsFYwpg8Dii-9MgI3aKSmUTBK_eOpAa3P1ohrrds" />
	<link rel="icon" type="image/svg+xml" href="${prefix}logo.svg">
	<link rel="alternate" type="application/json" title="Noctrinth news" href="${prefix}news/feed/articles.json">
	<link rel="preconnect" href="https://fonts.googleapis.com">
	<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
	<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800&family=JetBrains+Mono:wght@400;500&display=swap">
	<script>${LANG_BOOT}</script>
	<style>${BASE_CSS}${css}</style>
	${head}
</head>
<body>
	<nav class="nav">
		<div class="container container--wide nav__inner">
			<a class="nav__brand" href="${prefix || './'}" aria-label="Noctrinth Home">
				<svg class="nav__logo" viewBox="0 0 552 552" fill="none" aria-hidden="true"><path transform="rotate(45 276 276)" fill-rule="evenodd" clip-rule="evenodd" fill="var(--brand)" d="M81 81H231V171H201V111H111V171H81ZM261 81H471V171H441V111H291V171H261ZM141 141H171V201H291V261H321V141H411V411H381V351H261V231H171V291H141ZM381 171H351V291H291V321H381ZM81 201H111V441H261V381H291V471H81ZM441 201H471V471H321V381H351V441H441ZM201 261H231V411H141V321H171V381H201Z"/></svg>
				<span class="nav__brand-name">Noctrinth</span>
			</a>
			<div class="nav__links">
				${link('plugins/', t('Plugins', 'Плагины'), 'plugins')}
				${link('docs/', t('Docs', 'Документация'), 'docs')}
				${link('news/', t('News', 'Новости'), 'news')}
				${link('changelog/', t('Changelog', 'Изменения'), 'changelog')}
			</div>
			<div class="nav__actions">
				<button class="btn" type="button" data-lang-toggle aria-label="Language">
					<span class="i18n" lang="en">RU</span><span class="i18n" lang="ru">EN</span>
				</button>
				<a class="btn" href="${REPO}" rel="noopener" aria-label="GitHub">${icon('github')}</a>
			</div>
		</div>
	</nav>
	${body}
	<footer class="footer">
		<div class="container container--wide footer__row">
			<span>${t(
				'Noctrinth is an independent fork of the Modrinth App, not affiliated with or endorsed by Rinth, Inc.',
				'Noctrinth — независимый форк Modrinth App, не связанный с Rinth, Inc. и не одобренный ею.',
			)}</span>
			<span>
				<a href="${prefix}plugins/">${t('Plugins', 'Плагины')}</a> ·
				<a href="${prefix}docs/">${t('Docs', 'Документация')}</a> ·
				<a href="${REPO}" rel="noopener">GitHub</a> ·
				<a href="${REPO}/issues" rel="noopener">Issues</a> ·
				<a href="${prefix}news/feed/rss.xml">RSS</a> ·
				GPL-3.0
			</span>
		</div>
	</footer>
	<script>${LANG_TOGGLE}</script>
	<script>${DATE_SCRIPT}</script>
	<script>${MOTION_SCRIPT}</script>
	${scripts}
</body>
</html>
`
}

/** A date the page rewrites into its own language; the English one stands in without JavaScript. */
export function dateTag(iso) {
	const d = new Date(iso)
	const text = Number.isNaN(d.getTime())
		? ''
		: new Intl.DateTimeFormat('en-US', { year: 'numeric', month: 'long', day: 'numeric' }).format(d)
	return `<time datetime="${iso}" data-date>${text}</time>`
}
