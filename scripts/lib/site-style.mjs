// The site's look, taken from Modrinth's own dark theme
// (packages/assets/styles/variables.scss) with Noctrinth's purple as the brand,
// and the page frame every page shares: the bar along the top, the footer, and
// the English/Russian switch.

export const REPO = 'https://github.com/Everelsu/Noctrinth'
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
		--surface-1: #16181c;
		--surface-2: #1d1f23;
		--surface-3: #27292e;
		--surface-4: #34363c;
		--surface-5: #42444a;
		--bg: var(--surface-1);
		--bg-raised: var(--surface-3);
		--button-bg: var(--surface-4);
		--button-border: rgba(193, 190, 209, 0.12);
		--divider: var(--surface-4);
		--text: #b0bac5;
		--text-contrast: #ffffff;
		--text-secondary: #96a2b0;
		--brand: #c78aff;
		--brand-highlight: rgba(199, 138, 255, 0.25);
		--brand-gradient-bg: linear-gradient(0deg, rgba(35, 14, 55, 0.2) 0%, rgba(110, 45, 160, 0.1) 100%);
		--brand-gradient-border: rgba(200, 155, 255, 0.08);
		--link: #c78aff;
		--radius-md: 0.75rem;
		--radius-lg: 1rem;
		--radius-xl: 1.25rem;
	}

	* { box-sizing: border-box; }
	html, body {
		margin: 0;
		background: var(--bg);
		color: var(--text);
		font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
		font-size: 16px;
		line-height: 1.5;
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
		padding: 0.05rem 0.35rem;
		border-radius: 6px;
		font-size: 0.875em;
		font-family: 'JetBrains Mono', ui-monospace, Menlo, monospace;
	}

	.container { max-width: 56rem; margin-inline: auto; padding-inline: 1.5rem; }
	.container--wide { max-width: 72rem; }

	.nav {
		position: sticky;
		top: 0;
		z-index: 20;
		background: color-mix(in srgb, var(--surface-2) 88%, transparent);
		backdrop-filter: blur(14px);
		border-bottom: 1px solid var(--divider);
	}
	.nav__inner { display: flex; align-items: center; gap: 1.5rem; height: 4rem; }
	.nav__brand { display: flex; align-items: center; gap: 0.6rem; color: var(--text-contrast); font-weight: 800; font-size: 1.15rem; }
	.nav__brand:hover { text-decoration: none; }
	.nav__logo {
		width: 28px;
		height: 28px;
		background: var(--brand);
		-webkit-mask: var(--logo) center / contain no-repeat;
		mask: var(--logo) center / contain no-repeat;
	}
	.nav__links { display: flex; gap: 0.25rem; margin-right: auto; }
	.nav__links a {
		color: var(--text);
		font-weight: 600;
		padding: 0.45rem 0.85rem;
		border-radius: 999px;
	}
	.nav__links a:hover { background: var(--surface-3); color: var(--text-contrast); text-decoration: none; }
	.nav__links a[aria-current="page"] { background: var(--brand-highlight); color: var(--brand); }
	.nav__actions { display: flex; gap: 0.5rem; align-items: center; }

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
		font-weight: 600;
		cursor: pointer;
		transition: filter 0.15s ease, transform 0.1s ease;
	}
	.btn:hover { filter: brightness(1.15); text-decoration: none; }
	.btn:active { transform: scale(0.97); }
	.btn--brand { background: var(--brand); border-color: transparent; color: #1a0f2b; }
	.btn--xl { min-height: 3rem; padding: 0.75rem 1.5rem; font-size: 1.05rem; }
	.btn svg { width: 1.15em; height: 1.15em; }

	.page-title { margin: 2rem 0 0; font-size: 1.875rem; font-weight: 600; color: var(--text-contrast); }
	.section-heading { margin: 0 0 1rem; font-size: 1.25rem; font-weight: 800; color: var(--text-contrast); }

	.full-width-bg { background: var(--brand-gradient-bg); border-block: 1px solid var(--brand-gradient-border); }

	.footer {
		margin-top: 5rem;
		padding: 2rem 0 3rem;
		border-top: 1px solid var(--divider);
		color: var(--text-secondary);
		font-size: 0.9rem;
	}
	.footer__row { display: flex; flex-wrap: wrap; gap: 0.5rem 1.5rem; align-items: center; justify-content: space-between; }
	.footer a { color: var(--text); }

	@media (max-width: 640px) {
		.nav__inner { gap: 0.75rem; }
		.nav__links a { padding: 0.4rem 0.55rem; font-size: 0.92rem; }
		.nav__brand span { display: none; }
	}
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
		'A Modrinth App fork with Ely.by and offline accounts, CurseForge packs and skins for everyone on offline servers.'

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
	<link rel="icon" type="image/svg+xml" href="${prefix}logo.svg">
	<link rel="alternate" type="application/json" title="Noctrinth news" href="${prefix}news/feed/articles.json">
	<link rel="preconnect" href="https://fonts.googleapis.com">
	<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
	<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800&family=JetBrains+Mono:wght@400;500&display=swap">
	<script>${LANG_BOOT}</script>
	<style>:root { --logo: url('${prefix}logo.svg'); }${BASE_CSS}${css}</style>
	${head}
</head>
<body>
	<nav class="nav">
		<div class="container container--wide nav__inner">
			<a class="nav__brand" href="${prefix || './'}"><span class="nav__logo" role="img" aria-label="Noctrinth"></span><span>Noctrinth</span></a>
			<div class="nav__links">
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
			<span><a href="${REPO}" rel="noopener">GitHub</a> · <a href="${REPO}/issues" rel="noopener">Issues</a> · <a href="${prefix}news/feed/rss.xml">RSS</a> · GPL-3.0</span>
		</div>
	</footer>
	<script>${LANG_TOGGLE}</script>
	<script>${DATE_SCRIPT}</script>
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
