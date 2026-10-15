// The site's front page: what Noctrinth is, the download for this computer,
// what it adds over the Modrinth App, the latest news and a few answers.

import { articleCard, NEWS_CSS } from './site-news.mjs'
import { icon, page, REPO, t } from './site-style.mjs'

const FEATURES = [
	{
		media: 'assets/accounts.png',
		title: t('Microsoft, Ely.by or just a name', 'Microsoft, Ely.by или просто ник'),
		text: t(
			'Add a Microsoft account, sign in with Ely.by, or play singleplayer and offline servers under any name. All of them sit in one list and you switch with a click.',
			'Добавь аккаунт Microsoft, войди через Ely.by или играй в одиночку и на офлайн-серверах под любым ником. Все аккаунты в одном списке, переключение в один клик.',
		),
	},
	{
		media: 'assets/ely-by-skins.png',
		title: t('Ely.by skins in the launcher', 'Скины Ely.by в лаунчере'),
		text: t(
			'Browse, wear, upload and delete your Ely.by skins in the same grid the Microsoft account uses, without opening the website.',
			'Смотри, надевай, загружай и удаляй скины Ely.by в той же сетке, что и у аккаунта Microsoft, не открывая сайт.',
		),
	},
	{
		media: 'assets/skin-preview.gif',
		title: t('Everyone sees everyone', 'Все видят всех'),
		text: t(
			'Offline servers send no skins, so everyone is Steve. Noctrinth finds each player by name on Ely.by, Mojang and LittleSkin, with capes from OptiFine, LabyMod and MinecraftCapes. No mod needed.',
			'Офлайн-серверы не передают скины, и все ходят Стивами. Noctrinth находит каждого игрока по нику в Ely.by, Mojang и LittleSkin, а плащи берёт из OptiFine, LabyMod и MinecraftCapes. Моды не нужны.',
		),
	},
	{
		media: 'assets/curseforge-import.png',
		title: t('CurseForge packs and other launchers', 'Паки CurseForge и другие лаунчеры'),
		text: t(
			'Install a CurseForge <code>.zip</code> straight from disk, or bring instances over from CurseForge, Prism, MultiMC and the Modrinth App. Missing mods are fetched on the way in.',
			'Ставь <code>.zip</code> с CurseForge прямо с диска или переноси сборки из CurseForge, Prism, MultiMC и Modrinth App. Недостающие моды докачаются при импорте.',
		),
	},
	{
		media: 'assets/library-search.gif',
		title: t('A library that understands mods', 'Библиотека, которая понимает моды'),
		text: t(
			'<code>@sodium</code> finds every instance with a mod, <code>#shader</code> filters by type, <code>!outdated</code> by state.',
			'<code>@sodium</code> находит все сборки с модом, <code>#shader</code> фильтрует по типу, <code>!outdated</code> по состоянию.',
		),
	},
	{
		media: 'assets/translate.gif',
		title: t('Descriptions in your language', 'Описания на твоём языке'),
		text: t(
			'One button translates a project description with Google Translate or DeepL, and one more brings the original back.',
			'Одна кнопка переводит описание проекта через Google Translate или DeepL, ещё одна возвращает оригинал.',
		),
	},
	{
		media: 'assets/accents.gif',
		title: t('Nine accent colours', 'Девять акцентов'),
		text: t(
			'The whole interface, the splash screen and the taskbar icon in the colour you pick.',
			'Весь интерфейс, заставка и иконка на панели задач в выбранном цвете.',
		),
	},
]

const SMALL_FEATURES = [
	t('Two copies of one instance on two accounts', 'Две копии одной сборки на двух аккаунтах'),
	t('Modern Java for 1.7.10 in one click', 'Современная Java для 1.7.10 в один клик'),
	t('A graphics adapter per Java runtime', 'Своя видеокарта для каждой Java'),
	t('Touchpad swipes back and forward', 'Жесты тачпада: назад и вперёд'),
	t('A proxy for blocked regions', 'Прокси для заблокированных регионов'),
	t('Collections and notifications built in', 'Коллекции и уведомления внутри'),
	t('Animated instance icons', 'Анимированные иконки сборок'),
	t('No ads', 'Без рекламы'),
]

const FAQ = [
	{
		q: t('Is this the Modrinth App?', 'Это Modrinth App?'),
		a: t(
			'It is a fork of it. Noctrinth follows every Modrinth App release and keeps its version number, and adds what is listed above. It is not affiliated with Modrinth.',
			'Это его форк. Noctrinth повторяет каждый релиз Modrinth App с тем же номером версии и добавляет то, что перечислено выше. С Modrinth он не связан.',
		),
	},
	{
		q: t('Do I need a licensed Minecraft account?', 'Нужен ли лицензионный аккаунт?'),
		a: t(
			'No. A Microsoft account, an Ely.by account or just a name for singleplayer and offline-mode servers all work.',
			'Нет. Подойдёт аккаунт Microsoft, аккаунт Ely.by или просто ник для одиночной игры и офлайн-серверов.',
		),
	},
	{
		q: t(
			'Will my instances from the Modrinth App come over?',
			'Переедут ли сборки из Modrinth App?',
		),
		a: t(
			'Yes. Open Noctrinth and a banner offers to import them. Prism, MultiMC, ATLauncher, GDLauncher and CurseForge are under Create instance → Import.',
			'Да. Открой Noctrinth, и баннер предложит их импортировать. Prism, MultiMC, ATLauncher, GDLauncher и CurseForge — в «Создать сборку → Импорт».',
		),
	},
	{
		q: t('How does it update?', 'Как он обновляется?'),
		a: t(
			'By itself. Updates are signed and delivered through GitHub Releases; there is nothing to reinstall.',
			'Сам. Обновления подписаны и приходят через GitHub Releases, переустанавливать ничего не нужно.',
		),
	},
	{
		q: t('Something broke. Where do I report it?', 'Что-то сломалось. Куда писать?'),
		a: t(
			`Open an issue on <a href="${REPO}/issues" rel="noopener">GitHub</a>. Settings → Resource management → Export debug info gathers what is needed to look into it.`,
			`Открой issue на <a href="${REPO}/issues" rel="noopener">GitHub</a>. «Настройки → Управление ресурсами → Экспорт отладочной информации» соберёт всё, что нужно для разбора.`,
		),
	},
]

const HOME_CSS = `
	.hero { border-bottom: 1px solid var(--brand-gradient-border); background: var(--brand-gradient-bg); }
	.hero__banner { display: block; width: 100%; height: auto; max-height: 340px; object-fit: cover; }
	.hero__body { text-align: center; padding: 2rem 1.5rem 3rem; }
	.hero__lead { max-width: 44rem; margin: 0 auto; font-size: clamp(1.15rem, 2.4vw, 1.5rem); font-weight: 600; line-height: 1.4; color: var(--text-contrast); }
	.hero__sub { margin: 0.75rem auto 0; max-width: 38rem; color: var(--text-secondary); }
	.download { margin-top: 1.75rem; display: flex; flex-direction: column; align-items: center; gap: 0.9rem; }
	.download__main { box-shadow: 0 10px 40px -12px rgba(199, 138, 255, 0.7); }
	.download__main small { font-weight: 600; opacity: 0.7; }
	.download__others { display: flex; flex-wrap: wrap; justify-content: center; gap: 0.5rem; }
	.download__others .btn { min-height: 2rem; padding: 0.35rem 0.85rem; font-size: 0.875rem; }

	.home-section { margin-top: 4rem; }
	.home-section > .section-heading { font-size: 1.75rem; text-align: center; margin-bottom: 2rem; }
	.feature { display: grid; grid-template-columns: 1.15fr 1fr; gap: 2rem; align-items: center; margin-bottom: 1.25rem; padding: 1.25rem; background: var(--surface-2); border: 1px solid var(--button-border); border-radius: var(--radius-xl); }
	.feature--flip { grid-template-columns: 1fr 1.15fr; }
	.feature--flip .feature__media { order: 2; }
	.feature__media { aspect-ratio: 16 / 10; display: flex; align-items: center; justify-content: center; overflow: hidden; border-radius: var(--radius-lg); background: var(--bg); border: 1px solid var(--button-border); }
	.feature__media img { width: 100%; height: 100%; object-fit: contain; }
	.feature h3 { margin: 0 0 0.5rem; font-size: 1.3rem; color: var(--text-contrast); }
	.feature p { margin: 0; line-height: 1.6; }
	.chips { list-style: none; padding: 0; margin: 2rem 0 0; display: flex; flex-wrap: wrap; justify-content: center; gap: 0.5rem; }
	.chips li { padding: 0.45rem 0.95rem; border-radius: 999px; background: var(--surface-3); border: 1px solid var(--button-border); color: var(--text-contrast); font-size: 0.9rem; font-weight: 500; }
	.section-row { display: flex; align-items: center; justify-content: space-between; gap: 1rem; margin-bottom: 0.5rem; }
	.section-row .section-heading { margin: 0; font-size: 1.75rem; }
	.faq { display: flex; flex-direction: column; gap: 0.6rem; max-width: 46rem; margin: 0 auto; }
	.faq details { background: var(--surface-2); border: 1px solid var(--button-border); border-radius: var(--radius-lg); padding: 0.9rem 1.2rem; }
	.faq summary { cursor: pointer; font-weight: 700; color: var(--text-contrast); }
	.faq p { margin: 0.6rem 0 0; line-height: 1.6; }

	@media (max-width: 760px) {
		.feature, .feature--flip { grid-template-columns: 1fr; }
		.feature--flip .feature__media { order: 0; }
	}
`

/**
 * The front page.
 *
 * `version` is the build's own, baked into the download links so they work
 * without JavaScript; the script then asks GitHub for the latest release and
 * corrects them if one has come out since.
 */
export function renderHome({ version, articles }) {
	const asset = (name) => `${REPO}/releases/latest/download/${name}`
	const downloads = {
		windows: asset(`Noctrinth_${version}_x64-setup.exe`),
		macos: asset(`Noctrinth_${version}_universal.dmg`),
		appimage: asset(`Noctrinth_${version}_amd64.AppImage`),
		deb: asset(`Noctrinth_${version}_amd64.deb`),
	}

	const features = FEATURES.map(
		(feature, index) => `
		<section class="feature${index % 2 ? ' feature--flip' : ''}">
			<div class="feature__media"><img src="${feature.media}" alt="" loading="lazy"></div>
			<div>
				<h3>${feature.title}</h3>
				<p>${feature.text}</p>
			</div>
		</section>`,
	).join('')

	const news = articles.length
		? `
	<section class="container container--wide home-section">
		<div class="section-row">
			<h2 class="section-heading">${t('News', 'Новости')}</h2>
			<a class="btn" href="news/">${t('All news', 'Все новости')} ${icon('chevron')}</a>
		</div>
		<div class="news-grid">${articles
			.slice(0, 3)
			.map((article) => articleCard(article, ''))
			.join('')}</div>
	</section>`
		: ''

	const body = `
	<header class="hero">
		<img class="hero__banner" src="assets/noctrinth-banner.svg" alt="Noctrinth">
		<div class="hero__body">
			<p class="hero__lead">${t(
				'Ely.by, offline accounts and CurseForge packs, in the Modrinth App you already know.',
				'Ely.by, офлайн-аккаунты и паки с CurseForge — в том же Modrinth App, к которому ты привык.',
			)}</p>
			<p class="hero__sub">${t(
				'Free and open source. Follows every Modrinth App release.',
				'Бесплатный и с открытым кодом. Повторяет каждый релиз Modrinth App.',
			)}</p>
			<div class="download">
				<a class="btn btn--brand btn--xl download__main" id="download-main" href="${downloads.windows}">
					${icon('download')}
					<span>${t('Download', 'Скачать')}</span>
					<small id="download-label">Windows · v${version}</small>
				</a>
				<div class="download__others">
					<a class="btn" href="${downloads.windows}" data-os="windows">Windows</a>
					<a class="btn" href="${downloads.macos}" data-os="macos">macOS</a>
					<a class="btn" href="${downloads.appimage}" data-os="appimage">Linux AppImage</a>
					<a class="btn" href="${downloads.deb}" data-os="deb">Linux .deb</a>
					<a class="btn" href="${REPO}/releases" rel="noopener">${t('All releases', 'Все релизы')}</a>
				</div>
			</div>
		</div>
	</header>

	<main>
		<section class="container container--wide home-section">
			<h2 class="section-heading">${t('What it adds', 'Что он добавляет')}</h2>
			${features}
			<ul class="chips">${SMALL_FEATURES.map((item) => `<li>${item}</li>`).join('')}</ul>
		</section>
		${news}
		<section class="container home-section" id="faq">
			<h2 class="section-heading" style="text-align:center">${t('Questions', 'Вопросы')}</h2>
			<div class="faq">${FAQ.map(
				(item) => `
				<details>
					<summary>${item.q}</summary>
					<p>${item.a}</p>
				</details>`,
			).join('')}</div>
		</section>
	</main>`

	return page({
		title: 'Noctrinth — Minecraft launcher',
		current: 'home',
		css: NEWS_CSS + HOME_CSS,
		body,
		scripts: `<script>${downloadScript(version)}</script>`,
	})
}

/**
 * Points the main button at this computer's installer, from the latest release.
 *
 * Asked of GitHub's API, which needs no key for a public repository; if it does
 * not answer, the links baked in at build time already work.
 */
function downloadScript(version) {
	return `
	(function () {
		var ua = (navigator.userAgent || '').toLowerCase();
		var os = ua.indexOf('mac') !== -1 ? 'macos' : ua.indexOf('linux') !== -1 && ua.indexOf('android') === -1 ? 'appimage' : 'windows';
		var names = { windows: 'Windows', macos: 'macOS', appimage: 'Linux', deb: 'Linux .deb' };
		var main = document.getElementById('download-main');
		var label = document.getElementById('download-label');

		function apply(version) {
			var link = document.querySelector('[data-os="' + os + '"]');
			if (link) main.href = link.href;
			label.textContent = names[os] + ' · v' + version;
		}
		apply(${JSON.stringify(version)});

		fetch('https://api.github.com/repos/Everelsu/Noctrinth/releases/latest')
			.then(function (r) { return r.ok ? r.json() : null; })
			.then(function (release) {
				if (!release || !release.assets) return;
				var patterns = { windows: /setup\\.exe$/, macos: /\\.dmg$/, appimage: /\\.AppImage$/, deb: /\\.deb$/ };
				for (var key in patterns) {
					var asset = release.assets.filter(function (a) { return patterns[key].test(a.name); })[0];
					var link = document.querySelector('[data-os="' + key + '"]');
					if (asset && link) link.href = asset.browser_download_url;
				}
				apply(String(release.tag_name || '').replace(/^v/, '') || ${JSON.stringify(version)});
			})
			.catch(function () {});
	})();`
}
