/**
 * Noctrinth changelog.
 *
 * Mirrors the shape of Modrinth's `packages/blog/changelog.ts` so the
 * Changelog settings tab can render both with the same logic. This file is
 * Noctrinth-specific and intentionally lives outside `packages/blog` so it is
 * never overwritten when the repo is synced with upstream Modrinth.
 *
 * Add a new entry on top for each Noctrinth release. The `body` is markdown:
 * use `### Added`, `### Changed`, `### Deprecated`, `### Removed`,
 * `### Fixed`, `### Security` section headings (matching the Modrinth PR
 * changelog template / Keep a Changelog convention) and `- item` bullets.
 *
 * An entry can carry the same notes in Russian as `body_ru`, a second template
 * literal right after `body`: the launcher shows it when it runs in Russian and
 * the changelog site when it is switched to RU. Release notes on GitHub stay
 * English, from `body`.
 *
 * Keep every bullet to one line, in the plain style Modrinth's own changelog
 * uses: what changed, not why or how it was done. Links work, and so do
 * screenshots:
 *
 * ```markdown
 * ![Modern Java](/changelog/screenshots/modern-java.png)
 * ```
 *
 * Screenshots are the one part of an entry that is not written here. They live
 * in `src/changelog/screenshots/` and are published with the changelog site,
 * which is what the path above resolves against — so one is downloaded by
 * whoever scrolls to it instead of sitting inside every installer from then
 * on. See that folder's README.
 */

export interface NoctrinthVersionEntry {
	version: string
	/** ISO date string. */
	date: string
	body: string
	/** The same notes in Russian, where they have been translated. */
	body_ru?: string
}

export const NOCTRINTH_CHANGELOG: NoctrinthVersionEntry[] = [
	{
		version: '0.22.0+2',
		date: '2026-10-29T18:00:00+00:00',
		body: `### Fixed
- The game no longer crashes with "Invalid profile texture url" when joining a world on NeoForge. NeoForge makes Java prefer IPv6, and the launcher handed the game skins from an IPv6 loopback address written in a form the game cannot read; skins are now served from 127.0.0.1.
- A skin or cape whose address the game could not read is left out instead of crashing the game.`,
		body_ru: `### Исправлено
- Игра больше не вылетает с «Invalid profile texture url» при входе в мир на NeoForge. NeoForge заставляет Java предпочитать IPv6, и лаунчер отдавал игре скины с IPv6-адреса loopback в записи, которую игра не может прочитать; теперь скины отдаются с 127.0.0.1.
- Скин или плащ с адресом, который игра не может прочитать, пропускается, а не роняет игру.`,
	},
	{
		version: '0.22.0+1',
		date: '2026-10-29T12:00:00+00:00',
		body: `### Added
- Quick search with Ctrl+K (⌘K on macOS) from anywhere: instances with Play or Stop, installed mods and packs with Update, settings tabs by what is in them, in English or Russian, and the query handed to Discover on Modrinth or CurseForge. In the library it is the search field itself.
- The interface font can be changed in Settings → Appearance: Inter, the theme's own, the system's, the Minecraft font, or any font installed on the computer, picked from a searchable list. The Minecraft font now has Cyrillic letters, from Monocraft.
- The background can be a looping MP4 or WebM video of up to 100 MB; pictures and GIFs still take up to 32 MB.
- A CurseForge profile code, or a share link holding one, installs its modpack from the Import step of creating an instance.
- The Install button on curseforge.com can open the project in Noctrinth, at the file it was for: a toggle in the CurseForge settings, everywhere but macOS.
- Modrinth App's changelog in Russian too, in the Changelog tab and on the site. Thanks to [@Portalrolls](https://github.com/Portalrolls) for asking for it ([Everelsu/Noctrinth#10](https://github.com/Everelsu/Noctrinth/issues/10)).

### Changed
- Synced with upstream Modrinth main: version tables of projects with many game versions load faster, dropdowns with a search field stop jumping, Discover moves to its new layout, and translations are updated.
- A custom accent follows the colour wheel without lag.

### Fixed
- Instances sort by name the way numbers are read, 2 before 10.
- An instance opens with a tap on the touchpad again.
- Resizing Jump back in by dragging works with several columns.`,
		body_ru: `### Добавлено
- Быстрый поиск по Ctrl+K (⌘K на macOS) из любого места: сборки с кнопками «Играть» и «Остановить», установленные моды и паки с «Обновить», вкладки настроек по тому, что в них есть, — на английском или на русском, — и запрос, переданный в «Обзор» на Modrinth или CurseForge. В библиотеке это само поле поиска.
- Шрифт интерфейса меняется в Настройки → Внешний вид: Inter, шрифт темы, системный, шрифт Minecraft или любой шрифт, установленный на компьютере, из списка с поиском. У шрифта Minecraft теперь есть кириллица — из Monocraft.
- Фоном может быть зацикленное видео MP4 или WebM до 100 МБ; картинки и GIF — по-прежнему до 32 МБ.
- Код профиля CurseForge или ссылка с ним ставит модпак на шаге «Импорт» при создании сборки.
- Кнопка «Install» на curseforge.com может открыть проект в Noctrinth, на том файле, для которого нажата: переключатель в настройках CurseForge, везде, кроме macOS.
- Список изменений Modrinth App тоже на русском — во вкладке «Список изменений» и на сайте. Спасибо [@Portalrolls](https://github.com/Portalrolls) за идею ([Everelsu/Noctrinth#10](https://github.com/Everelsu/Noctrinth/issues/10)).

### Изменено
- Синхронизация с основной веткой Modrinth: таблицы версий у проектов с множеством версий игры грузятся быстрее, выпадающие списки с поиском больше не прыгают, «Обзор» переехал на новую раскладку, обновлены переводы.
- Свой цвет акцента следует за цветовым колесом без задержки.

### Исправлено
- Сборки сортируются по имени так, как читаются числа: 2 раньше 10.
- Сборка снова открывается касанием тачпада.
- Изменение размера «Недавнего» перетаскиванием работает при нескольких колонках.`,
	},
	{
		version: '0.22.0',
		date: '2026-10-22T12:00:00+00:00',
		body: `### Added
- Any accent colour, not only a preset: Custom in Settings → Appearance opens a colour wheel with a brightness slider and a field for the hex value, and the window recolours as it moves.

### Changed
- Synced with upstream Modrinth 0.22.0: shared instances can carry any file in the instance, the Push update window is redesigned with Changes and Files tabs, the download manager shows shared instance uploads, the Play button no longer stalls after the launcher has been open a while, and old launcher logs and cached project data are cleaned up.
- Error messages say what went wrong in full, down to its cause.
- The Amethyst and Glacier accents are gone from the list; whoever picked one keeps that colour as a custom one.

### Fixed
- The installer no longer stops with "Error opening file for writing" when Noctrinth has only just closed.`,
		body_ru: `### Добавлено
- Любой цвет акцента, а не только пресет: «Свой» в Настройки → Внешний вид раскрывает цветовое колесо с ползунком яркости и полем для HEX, и окно перекрашивается прямо во время выбора.

### Изменено
- Синхронизация с Modrinth 0.22.0: в общие сборки можно добавлять любые файлы сборки, окно публикации обновления переделано и разделено на вкладки «Изменения» и «Файлы», управление загрузками показывает выгрузку общих сборок, кнопка «Играть» больше не зависает после долгой работы лаунчера, а старые логи лаунчера и кеш проектов теперь чистятся.
- Сообщения об ошибках говорят, что пошло не так, целиком — вплоть до причины.
- Акценты «Аметист» и «Ледник» убраны из списка; у тех, кто их выбрал, этот цвет остаётся как свой.

### Исправлено
- Установщик больше не останавливается с ошибкой «Error opening file for writing», если Noctrinth только что закрыли.`,
	},
	{
		version: '0.21.10',
		date: '2026-10-20T12:00:00+00:00',
		body: `### Added
- CurseForge in Discover, next to Modrinth: a switch above the results, CurseForge's own categories and filters, and project pages with the description, versions, a page for each version with its changelog, the gallery, and a star to keep a project among favourites.
- Mods, resource packs, shaders and modpacks install from CurseForge with the same checks as Modrinth's: which instances they fit, a warning for an incompatible version, and their required dependencies.
- A file whose author keeps it off launchers is looked for on disk first, in Downloads and in this and other launchers' instances; otherwise one window links to its pages and picks it up from Downloads as soon as it lands there.
- CurseForge mods in an instance are recognised by their fingerprint, even ones added by hand or brought from another launcher, and show their name, icon, author and a link to their page.
- CurseForge mods get updates like Modrinth ones: the update badge, Update all, and switching versions with their changelogs.
- A CurseForge tab in Settings: turn it off, choose where Discover opens, and set whether files are recognised, how far updates go (releases, betas or anything) and whether other launchers are searched for files kept off launchers.
- In the Files tab, mods, resource packs and shaders show their project's icon, name, version and author, and screenshots a preview.

### Changed
- Synced with upstream Modrinth 0.21.10: mods can be installed, enabled, disabled and removed while the instance is running, and translations are updated.
- Settings are sorted into Display, Account, Instance defaults and System. Default Java and memory, window and launch hooks each have a tab of their own, Synced settings is called Syncing now, and Resource management is split into Resources and Troubleshooting.
- Java installations, the Java runtimes Noctrinth downloads and the graphics card each one uses are all in the Java and memory tab; the proxy is in Resources, next to the app directory.
- Instance settings are split into Instance and Game sections, and every Game tab has a button that opens the default settings it overrides.
- The empty Logs tab is kept by a night owl under the moon instead of Modrinth's frog.
- The settings windows' sidebar, backgrounds and scrollbars are refreshed, and one opened on a given tab scrolls its sidebar to it.

### Fixed
- The update window on the Content tab no longer closes when another project finishes updating.
- The Content tab and Discover keep their scroll position when coming back from a project page.
- Joining a server whose modpack is unavailable or under review no longer opens a blank window.
- On Linux the launcher no longer keeps a CPU core busy because of the database.
- Beta and alpha versions are coloured the same everywhere.
- Discover no longer throws an error when leaving an instance it was adding to.
- The header over Discover while adding to an instance lets the background picture through as frosted glass.`,
		body_ru: `### Добавлено
- CurseForge в «Обзоре» рядом с Modrinth: переключатель над результатами, собственные категории и фильтры CurseForge и страницы проектов с описанием, версиями, страницей каждой версии с её списком изменений, галереей и звёздочкой, чтобы добавить проект в избранное.
- Моды, ресурспаки, шейдеры и модпаки ставятся с CurseForge с теми же проверками, что и с Modrinth: в какие сборки они подходят, предупреждение о несовместимой версии и обязательные зависимости.
- Файл, который автор не разрешает скачивать лаунчерам, сначала ищется на диске — в «Загрузках» и в сборках этого и других лаунчеров; иначе одно окно даёт ссылки на его страницы и подхватывает файл из «Загрузок», как только он там появится.
- Моды CurseForge в сборке распознаются по отпечатку, даже добавленные вручную или перенесённые из другого лаунчера, и показывают название, иконку, автора и ссылку на страницу.
- Моды CurseForge обновляются так же, как моды Modrinth: значок обновления, «Обновить всё» и смена версии со списками изменений.
- Вкладка CurseForge в настройках: выключить его, выбрать, где открывается «Обзор», и настроить распознавание файлов, какие обновления предлагать (релизы, беты или всё подряд) и искать ли в других лаунчерах файлы, закрытые для лаунчеров.
- Во вкладке «Файлы» моды, ресурспаки и шейдеры показывают иконку, название, версию и автора своего проекта, а скриншоты — превью.

### Изменено
- Синхронизация с Modrinth 0.21.10: моды можно ставить, включать, выключать и удалять, пока сборка запущена, и обновлены переводы.
- Настройки разложены на «Отображение», «Аккаунт», «Сборки по умолчанию» и «Система». У Java и памяти по умолчанию, окна и команд запуска теперь свои вкладки, «Синхронизируемые настройки» называются «Синхронизация», а «Управление ресурсами» разделено на «Ресурсы» и «Устранение неполадок».
- Расположение Java, среды Java, которые скачивает Noctrinth, и видеокарта для каждой из них собраны во вкладке «Java и память»; прокси — в «Ресурсах», рядом с каталогом приложения.
- Настройки сборки разделены на разделы «Сборка» и «Игра», и в каждой вкладке «Игры» есть кнопка, открывающая настройки по умолчанию, которые она переопределяет.
- Пустую вкладку «Журнал» сторожит сова под луной вместо лягушки Modrinth.
- Боковая панель, фоны и полосы прокрутки окон настроек обновлены, а окно, открытое на конкретной вкладке, прокручивает панель к ней.

### Исправлено
- Окно обновления на вкладке «Контент» больше не закрывается, когда обновление другого проекта завершается.
- Вкладка «Контент» и «Обзор» сохраняют позицию прокрутки при возврате со страницы проекта.
- Вход на сервер, чей модпак недоступен или на проверке, больше не открывает пустое окно.
- На Linux лаунчер больше не держит ядро процессора занятым из-за базы данных.
- Бета- и альфа-версии везде окрашены одинаково.
- «Обзор» больше не выдаёт ошибку при уходе из сборки, в которую добавлял контент.
- Шапка над «Обзором» при добавлении в сборку пропускает фоновую картинку как матовое стекло.`,
	},
	{
		version: '0.21.9+2',
		date: '2026-10-16T12:00:00+00:00',
		body: `### Added
- A background picture behind the launcher, chosen in Settings → Appearance, blurred or not. The frame lets it through, the cards on a page a little of it, and dialogs turn to frosted glass over it. The idea is [Celestial Launcher](https://github.com/Wemsur/Celestial-Launcher)'s.
- The background picture can be dropped onto the window from Settings → Appearance, and darkened with a slider for a bright or busy one. Menus and tooltips over it turn to frosted glass too.

### Changed
- Hiding and showing the right sidebar is one smooth motion: the page narrows and widens along with it, the library's cards travel to their new places in step with it, and the sidebar keeps its contents until it is gone. The folding sections in menus open and close on the same curve, their contents fading in and out.
- Synced with upstream Modrinth main again: a shortcut to an instance with a dot in its name keeps the whole name, the command history no longer syncs with that setting off, the files tab no longer reads a whole file into memory to download it, and on Linux the instance file watcher no longer floods the log.

### Fixed
- In the library's compact view the cards keep their size: more room fits more of them, instead of every card getting smaller to make space for another column.
- Hide nametag hides the nametag on an Ely.by account's skin preview too.
- Wearing, uploading, editing and deleting Ely.by skins only goes ahead when the Ely.by site is signed in as the selected account, and says so when it is not, so with several accounts a change never lands on another one.
- In-game skins work on Minecraft 26.1 and later, where authlib renamed the classes the launcher hooks into and Ely.by and offline players all showed up as Steve.
- Skins in game load faster and stop going missing: every texture, a licensed server's signed ones included, is fetched by the launcher through its proxy, over http and https at once, with retries, instead of once by the game straight from a CDN that is not always reachable. A stalled download no longer leaves a player as Steve for the session, which is how a licensed player came to see nobody's skin on Hypixel and not their own among offline ones.
- A player seen for the first time no longer gets a default skin when the skin system is slow to answer: on 1.20.2 and later the game is given the full answer instead of an empty early one, so the right skin shows without a rejoin.
- Stopping every copy of an instance no longer freezes the launcher until the games have closed, and the page shows them as stopping until they have, instead of offering Play while they are still running.
- A skin can be added to an Ely.by account that has none yet: the add tile shows even when the list is empty.
- A Microsoft account's head in the account list is cut from the skin Mojang has for it, instead of a head service that kept showing a changed skin as Steve.
- Database backups are kept in a Noctrinth folder of their own, no longer in Modrinth App's, where the two apps' backups shared names and could be mixed up. The debug export, the screenshots archive and a few settings say Noctrinth instead of Modrinth App, and the telemetry switch says what is true: Noctrinth sends no analytics either way.`,
		body_ru: `### Добавлено
- Фоновая картинка за лаунчером выбирается в Настройки → Внешний вид, размытая или нет. Рамка её пропускает, карточки на странице — немного, а диалоги превращаются поверх неё в матовое стекло. Идея взята у [Celestial Launcher](https://github.com/Wemsur/Celestial-Launcher).
- Фоновую картинку можно перетащить в окно из Настройки → Внешний вид и затемнить ползунком, если она яркая или пёстрая. Меню и подсказки поверх неё тоже становятся матовым стеклом.

### Изменено
- Скрытие и показ правой панели — одно плавное движение: страница сужается и расширяется вместе с ней, карточки библиотеки переезжают на новые места в такт, а панель держит содержимое, пока не скроется. Раскрывающиеся разделы в меню открываются и закрываются по той же кривой, а их содержимое плавно появляется и исчезает.
- Снова синхронизация с основной веткой Modrinth: ярлык сборки с точкой в названии сохраняет имя целиком, история команд больше не синхронизируется при выключенной настройке, вкладка «Файлы» больше не читает файл целиком в память, чтобы его скачать, а на Linux наблюдатель за файлами сборки больше не засыпает журнал сообщениями.

### Исправлено
- В компактном виде библиотеки карточки сохраняют размер: на большее место помещается больше карточек, а не все уменьшаются ради ещё одной колонки.
- «Без имени над скином» убирает имя и в превью скина аккаунта Ely.by.
- Надевание, загрузка, изменение и удаление скинов Ely.by выполняются, только когда на сайте Ely.by выполнен вход в выбранный аккаунт, а иначе об этом говорится — так при нескольких аккаунтах изменение не попадёт в чужой.
- Скины в игре работают на Minecraft 26.1 и новее, где authlib переименовал классы, к которым подключается лаунчер, и игроки Ely.by и офлайн-игроки все выглядели как Стив.
- Скины в игре грузятся быстрее и не пропадают: каждую текстуру, включая подписанные текстуры лицензионного сервера, скачивает лаунчер через свой прокси, по http и https сразу и с повторами, а не игра один раз напрямую с CDN, который доступен не всегда. Зависшая загрузка больше не оставляет игрока Стивом на весь сеанс — из-за этого лицензионный игрок не видел ничьих скинов на Hypixel, а свой — среди офлайн-игроков.
- Игрок, увиденный впервые, больше не получает скин по умолчанию, когда система скинов медленно отвечает: на 1.20.2 и новее игре отдаётся полный ответ вместо пустого раннего, и нужный скин появляется без перезахода.
- Остановка всех копий сборки больше не замораживает лаунчер, пока игры не закроются, а страница показывает их останавливающимися, пока они не закроются, вместо кнопки «Играть», пока они ещё работают.
- Скин можно добавить аккаунту Ely.by, у которого скинов ещё нет: плитка добавления видна и при пустом списке.
- Голова аккаунта Microsoft в списке аккаунтов вырезается из скина, который хранит Mojang, а не берётся из сервиса голов, где сменённый скин так и оставался Стивом.
- Резервные копии базы данных хранятся в собственной папке Noctrinth, а не в папке Modrinth App, где копии двух приложений назывались одинаково и могли перепутаться. Экспорт отладочной информации, архив скриншотов и несколько настроек называют Noctrinth вместо Modrinth App, а переключатель телеметрии говорит правду: Noctrinth не отправляет аналитику в любом случае.`,
	},
	{
		version: '0.21.9+1',
		date: '2026-10-15T12:00:00+00:00',
		body: `### Added
- Noctrinth's own news shows up in the News feed on the home page next to Modrinth's, in Russian when the launcher is. The articles live on the [new site](https://everelsu.github.io/Noctrinth/news/), which has the changelog on a page of its own now.
- Stop on an instance running more than once asks which copy to stop, each named by its account and the time it started, or stops all of them.
- In-game skins and capes come from more places: LittleSkin for skins, and LabyMod, MinecraftCapes and SkinMC for capes, after Mojang and OptiFine.
- The player in the skin preview turns its head to follow the pointer, and looks ahead again once it stops.

### Changed
- Synced with upstream Modrinth main: the "CancelledError" that popped up when opening an instance from the home page is gone, a failed database backup no longer stops the app, install backups are kept per job, and the install-to-play window says more when a modpack is missing.
- Jump back in shows up to ten instances and worlds instead of five.
- The button for another copy spins while that copy starts.
- Minecraft starts sooner: Ely.by and offline accounts no longer wait on Mojang's profile servers, and a Java already checked is not checked again on every launch.
- Skin lookups in game answer from the first source that has one instead of waiting on all of them, go through the launcher's proxy, and a source that is down is skipped for a while rather than slowing every player.
- authlib-injector updates itself, and a download is checked against its checksum before it is used.
- The "Cannot reach authentication servers" banner shows only for Microsoft accounts.

### Fixed
- The retro theme is green throughout: the sidebar, the dimmer behind dialogs, the loading bar and the window and taskbar icon no longer stay purple.
- The skins page shows the default skin when no account is signed in, instead of an empty space.
- More than one Ely.by account can be signed in: adding another one asks which account to sign in with, instead of signing the first one in again.
- Export debug info no longer fails with "os error 33" while the game is running.
- Ely.by skins load again after a failed request instead of staying empty, and the cape shows in the skin preview.
- Leaving an instance's page while it runs and coming back keeps its log.
- Stopping one copy of an instance no longer shows the instance as stopped while another copy is still running.
- The splash cube chosen in the developer flags no longer goes back to the default: it is kept in the launcher's settings instead of the webview's storage alone.
- Closing the game no longer ends every session with an error about the launcher's connection to it.`,
		body_ru: `### Добавлено
- Новости самого Noctrinth появляются в ленте новостей на главной рядом с новостями Modrinth — на русском, если лаунчер на русском. Статьи живут на [новом сайте](https://everelsu.github.io/Noctrinth/news/), где у списка изменений теперь своя страница.
- «Остановить» у сборки, запущенной несколько раз, спрашивает, какую копию остановить — каждая подписана аккаунтом и временем запуска, — или останавливает все.
- Скины и плащи в игре берутся из большего числа мест: LittleSkin для скинов и LabyMod, MinecraftCapes и SkinMC для плащей, после Mojang и OptiFine.
- Персонаж в превью скина поворачивает голову за указателем и снова смотрит прямо, когда тот замирает.

### Изменено
- Синхронизация с основной веткой Modrinth: пропала ошибка «CancelledError» при открытии сборки с главной, неудачная резервная копия базы больше не останавливает приложение, резервные копии установки хранятся для каждой задачи отдельно, а окно «установить и играть» подробнее объясняет, если модпака нет.
- «Недавнее» показывает до десяти сборок и миров вместо пяти.
- Кнопка ещё одной копии крутится, пока эта копия запускается.
- Minecraft запускается быстрее: аккаунты Ely.by и офлайн-аккаунты больше не ждут серверов профилей Mojang, а уже проверенная Java не проверяется заново при каждом запуске.
- Поиск скинов в игре отвечает по первому источнику, у которого скин есть, а не ждёт все, идёт через прокси лаунчера, а недоступный источник на время пропускается, а не тормозит каждого игрока.
- authlib-injector обновляется сам, а скачанный файл сверяется с контрольной суммой до использования.
- Баннер «Нет связи с серверами авторизации» показывается только для аккаунтов Microsoft.

### Исправлено
- Ретро-тема зелёная целиком: боковая панель, затемнение за диалогами, полоса загрузки и иконка окна и панели задач больше не остаются фиолетовыми.
- Страница скинов показывает скин по умолчанию, когда ни в один аккаунт не выполнен вход, а не пустое место.
- Можно войти в несколько аккаунтов Ely.by: при добавлении ещё одного спрашивается, каким аккаунтом войти, а не выполняется повторный вход в первый.
- «Экспорт отладочной информации» больше не падает с «os error 33», пока игра запущена.
- Скины Ely.by снова загружаются после неудачного запроса, а не остаются пустыми, и плащ виден в превью скина.
- Если уйти со страницы запущенной сборки и вернуться, её лог сохраняется.
- Остановка одной копии сборки больше не показывает сборку остановленной, пока другая копия ещё работает.
- Куб на заставке, выбранный во флагах разработчика, больше не сбрасывается на стандартный: он хранится в настройках лаунчера, а не только в хранилище веб-окна.
- Закрытие игры больше не завершает каждый сеанс ошибкой о соединении лаунчера с ней.`,
	},
	{
		version: '0.21.9',
		date: '2026-10-08T12:00:00+00:00',
		body: `### Added
- Two fingers across the touchpad go back and forward through pages, exactly the way Chrome does. An arrow slides out of the window's edge and lights up once the swipe is far enough; the page changes when the fingers lift, so pulling back before that cancels it. Scrolling up and down, and anything that scrolls sideways itself, are left alone.
- A project's description can be translated with one button and switched back to the original, the paragraphs washing over from one language to the other; links, formatting and code stay as they are. Settings → Language, at the top, picks the service (Google Translate, or DeepL on your own free key), the language to translate into, languages you read anyway, whether to translate as soon as a description opens, and whether to check the language first so the button stays away when there is nothing to translate; a test button tries the service out.
- The changelog here is read from the changelog site, so notes for releases since this build, and corrections to older ones, show up without an update. The copy that ships with the app is still there offline.

### Changed
- Synced with upstream Modrinth 0.21.9. Signing in to a Modrinth account can now come back to the launcher through a deep link, is better protected, and gets a session of its own, so signing out of the website no longer signs the launcher out at random. The "missing field synced_options" error is fixed.
- The launch button is one shape that flows between Play, Starting, Installing and Stop, changing width and colour on a spring while the label melts from one into the next. The button for a second copy runs out of Stop like a drop and comes away, and runs back in when the game closes.
- In the skin preview the nametag stays above the head through every turn, tilt and zoom, and turns with the model like a sign. The shadow tilts with the floor, and the hint under the feet says the wheel zooms.
- The splash screen is redrawn to stay smooth while the app starts behind it: the bar and the fade out run on the compositor, so a busy start no longer freezes them, and the window opens already painted instead of flashing black. It takes its colours from the theme and the accent, with a grey cube behind them, so a light theme or a non-purple accent no longer shows a lavender splash that then changes colour as it fades.
- One "Add account" button asks which kind of account to add, instead of three buttons in a row.
- The Play and Stop buttons on the home page's recent instances and worlds slide into one another, pop their spinner in, give a little when pressed and nudge their icon on hover.
- Pages open faster: every page's code is loaded while the launcher is idle instead of on the first visit, and a project's details start loading while the pointer rests on its card, so opening it from Browse finds them ready.
- The skin preview's hints fit a narrow preview, like the one in the edit dialog, on two tidy lines below the shadow instead of breaking each over three on top of it.
- Clicking a big modpack's card opens its page straight away: the page no longer waits for every file in the instance to be scanned, and the mod list shows its own loading while it does. Lists of up to 150 mods are drawn whole, so a quick scroll no longer shows empty gaps.
- The splash cube can be inverted, black on white or white on black, or picked at random each launch, from the developer flags. The default shows the inverted one now and then.
- Adding an Ely.by account opens Ely.by's sign-in in a window, the way a Microsoft account does, with the code already filled in.
- The "Cannot reach authentication servers" banner can be dismissed, has a button to check again, rechecks every thirty seconds until the servers answer, and says that offline and Ely.by accounts are not affected. A single dropped request no longer raises it.

### Fixed
- Opening an older crash report from the Logs tab names the mod it happened in too, with a button to disable it.
- The taskbar shows the accent's icon in the installed launcher too: a pinned launcher's button is drawn from its shortcut, not the window, so the launcher's own shortcuts on the taskbar, in the Start menu and on the desktop are now given the accent's icon as well, and get the original back with the theme's colour.
- Instance logs no longer come out with a placeholder after every letter once an Ely.by account had signed in through its page: the log censor replaced that account's empty client token everywhere.
- An Ely.by account's skins show up again, and wearing, uploading, editing and deleting them works: Ely.by moved its site's API to new addresses.`,
		body_ru: `### Добавлено
- Два пальца по тачпаду листают страницы назад и вперёд — ровно как в Chrome. Из края окна выезжает стрелка и подсвечивается, когда жест достаточно длинный; страница меняется, когда пальцы отрываются, так что если отвести их назад раньше, жест отменится. Прокрутка вверх-вниз и всё, что само прокручивается вбок, не затрагиваются.
- Описание проекта можно перевести одной кнопкой и вернуть оригинал, абзацы перетекают с одного языка на другой; ссылки, форматирование и код остаются как есть. В Настройки → Язык, в самом верху, выбираются сервис (Google Переводчик или DeepL с вашим бесплатным ключом), язык перевода, языки, которые вы и так читаете, переводить ли сразу при открытии описания и проверять ли сначала язык, чтобы кнопка не появлялась, когда переводить нечего; тестовая кнопка проверяет сервис.
- Список изменений здесь читается с сайта списка изменений, так что заметки о релизах, вышедших после этой сборки, и исправления старых появляются без обновления. Копия, поставляемая с приложением, остаётся на случай офлайна.

### Изменено
- Синхронизация с Modrinth 0.21.9. Вход в аккаунт Modrinth теперь может возвращаться в лаунчер по глубокой ссылке, лучше защищён и получает собственный сеанс, так что выход на сайте больше не выкидывает из лаунчера наугад. Ошибка «missing field synced_options» исправлена.
- Кнопка запуска — одна форма, перетекающая между «Играть», «Запуск», «Установка» и «Остановить»: ширина и цвет меняются пружинисто, а надпись плавно перетекает в следующую. Кнопка второй копии вытекает из «Остановить» каплей и отделяется, а когда игра закрывается, втекает обратно.
- В превью скина имя держится над головой при любом повороте, наклоне и приближении и поворачивается с моделью, как табличка. Тень наклоняется вместе с полом, а подсказка под ногами говорит, что колёсико приближает.
- Заставка перерисована, чтобы оставаться плавной, пока за ней запускается приложение: полоса и исчезновение идут на композиторе, так что нагруженный запуск их больше не замораживает, а окно открывается уже отрисованным, без чёрной вспышки. Она берёт цвета из темы и акцента, с серым кубом за ними, так что светлая тема или нефиолетовый акцент больше не показывают лавандовую заставку, меняющую цвет при исчезновении.
- Одна кнопка «Добавить аккаунт» спрашивает, какой аккаунт добавить, вместо трёх кнопок в ряд.
- Кнопки «Играть» и «Остановить» у недавних сборок и миров на главной перетекают друг в друга, показывают спиннер, чуть поддаются при нажатии и подталкивают иконку при наведении.
- Страницы открываются быстрее: код каждой страницы подгружается, пока лаунчер простаивает, а не при первом посещении, а данные проекта начинают грузиться, пока указатель лежит на его карточке, так что при открытии из «Обзора» они уже готовы.
- Подсказки превью скина помещаются в узкое превью, как в окне редактирования, — в две аккуратные строки под тенью, а не по три строки поверх неё.
- Клик по карточке большого модпака сразу открывает его страницу: страница больше не ждёт, пока просканируются все файлы сборки, а список модов показывает собственную загрузку. Списки до 150 модов рисуются целиком, так что быстрая прокрутка больше не показывает пустые дыры.
- Куб на заставке можно инвертировать — чёрный на белом или белый на чёрном — или выбирать случайно при каждом запуске во флагах разработчика. По умолчанию изредка показывается инвертированный.
- Добавление аккаунта Ely.by открывает вход Ely.by в отдельном окне, как у аккаунта Microsoft, с уже заполненным кодом.
- Баннер «Нет связи с серверами авторизации» можно закрыть, у него есть кнопка повторной проверки, он перепроверяет каждые тридцать секунд, пока серверы не ответят, и говорит, что офлайн-аккаунтов и аккаунтов Ely.by это не касается. Один оборвавшийся запрос его больше не вызывает.

### Исправлено
- Открытие старого отчёта о краше из вкладки «Журнал» тоже называет мод, в котором он случился, с кнопкой его отключить.
- Панель задач показывает иконку акцента и в установленном лаунчере: кнопка закреплённого лаунчера рисуется по ярлыку, а не по окну, поэтому собственные ярлыки лаунчера на панели задач, в меню «Пуск» и на рабочем столе тоже получают иконку акцента, а с цветом темы — исходную обратно.
- Логи сборки больше не выходят с заглушкой после каждой буквы после входа в аккаунт Ely.by через его страницу: цензор логов заменял везде пустой клиентский токен этого аккаунта.
- Скины аккаунта Ely.by снова видны, а надевание, загрузка, изменение и удаление работают: Ely.by перенёс API своего сайта на новые адреса.`,
	},
	{
		version: '0.21.8',
		date: '2026-10-07T00:00:00+00:00',
		body: `### Added
- An "Export debug info" button in Resource management that gathers what support needs to look into a problem.
- 34 more official skins in the skin selector, from the Minecraft Dungeons hero packs.
- An instance running more than once shows its copies' consoles behind tabs named after each account, or side by side.
- The skin preview of an offline account wears the cape its name has: a licensed player's official one, or one from OptiFine, LabyMod, MinecraftCapes or SkinMC, with where it came from shown under the name.
- Motion in the style of the download manager's button: the Play, Starting and Stop states slide into one another with a spinner while starting or stopping, the running instance in the top bar breathes and slides in, and the crash panel rises in with its causes one after another.

### Changed
- Synced with upstream Modrinth (0.21.6 → 0.21.8).
- The crash panel is rebuilt: one card with the likely causes in order of how bad they are, what the game itself said under the heading, and the rest folded away past three.
- A custom Java in an instance's settings can be any version, not only the one the instance would pick.
- Update all with a single project uses the normal update window.

### Fixed
- Launching a second copy of a running instance works again; a check added upstream refused it as already running.
- A class a mod only checks for — whether another mod is installed — is no longer reported as what crashed the game. Those rules now read only the crash itself.
- Leaving the Logs tab while a game runs and coming back no longer loses what it printed in between.
- Instances whose files the content store cannot link no longer retry adopting every file before each launch, which filled the launcher's log with hundreds of warnings.
- Download manager tasks no longer get stuck on "Pending...", and failed installs of a deleted instance no longer freeze the app on launch.
- Several Update all failures: two files of the same project, "modpack data invalid", and a single project.
- Disabling content no longer resets its "Date added".
- The window no longer grows on Windows.`,
		body_ru: `### Добавлено
- Кнопка «Экспорт отладочной информации» в «Управлении ресурсами» собирает то, что нужно поддержке, чтобы разобраться в проблеме.
- Ещё 34 официальных скина в выборе скинов — из наборов героев Minecraft Dungeons.
- Сборка, запущенная несколько раз, показывает консоли копий во вкладках с именами аккаунтов или рядом.
- Превью скина офлайн-аккаунта носит плащ, который есть у этого имени: официальный у лицензионного игрока или из OptiFine, LabyMod, MinecraftCapes или SkinMC, а под именем указано, откуда он.
- Анимация в стиле кнопки управления загрузками: состояния «Играть», «Запуск» и «Остановить» перетекают друг в друга со спиннером при запуске и остановке, запущенная сборка в верхней панели «дышит» и выезжает, а панель краша поднимается, показывая причины одну за другой.

### Изменено
- Синхронизация с Modrinth (0.21.6 → 0.21.8).
- Панель краша переделана: одна карточка с вероятными причинами по степени серьёзности, под заголовком — что сказала сама игра, а остальное свёрнуто после трёх.
- Своя Java в настройках сборки может быть любой версии, а не только той, что выбрала бы сборка.
- «Обновить всё» с одним проектом использует обычное окно обновления.

### Исправлено
- Запуск второй копии работающей сборки снова работает; проверка, добавленная в Modrinth, отказывала, считая её уже запущенной.
- Класс, который мод лишь проверяет — установлен ли другой мод, — больше не называется причиной краша. Эти правила теперь читают только сам краш.
- Уход из вкладки «Журнал», пока игра работает, и возврат больше не теряют то, что она напечатала в промежутке.
- Сборки, чьи файлы хранилище контента не может связать, больше не пытаются принять каждый файл перед каждым запуском, из-за чего журнал лаунчера заполнялся сотнями предупреждений.
- Задачи в управлении загрузками больше не застревают на «Ожидание...», а неудачные установки удалённой сборки больше не замораживают приложение при запуске.
- Несколько сбоев «Обновить всё»: два файла одного проекта, «modpack data invalid» и единственный проект.
- Отключение контента больше не сбрасывает его «Дату добавления».
- Окно больше не растёт на Windows.`,
	},
	{
		version: '0.21.6+1',
		date: '2026-09-29T00:00:00+00:00',
		body: `### Added
- A window comes up by itself when a game crashes, with what the launcher found and buttons that fix it, instead of waiting for someone to open the Logs tab.
- The crash reader names the mod the crash happened in: classes in the stack trace, mixin configs and loader errors are matched to the jar in \`mods/\` they came from, with a button to disable it.
- It reads how the game ended: closed by Windows for not responding, killed by the system for memory, a missing system library, or native code crashing without a report.
- It says which mods were added or removed since the instance last ran without crashing.
- It finds damaged mod jars, empty config files (a world's serverconfig too), the same mod installed twice, and Intel 13th and 14th generation processors known to become unstable.
- An addon built for another version of the mod it extends is told apart from one whose dependency is not installed.
- More causes recognised: an incomplete install, a JNA library the antivirus blocked, a broken KubeJS datapack, a damaged server config, FerriteCore's neighbour table, Spark's profiler on newer Java, and more ways a graphics driver goes down.
- Keyboard shortcuts in the screenshots gallery: Ctrl+C copies the selected screenshots, or the focused one; Ctrl+A selects all of them, Delete deletes, Escape clears the selection.
- Several screenshots copy as files, like Ctrl+C in Explorer, so pasting into a folder or a chat gets all of them. One still copies as an image.

### Changed
- Switching between the launcher and the game no longer makes every open page reload its data. Each switch used to fire a round of requests, which is where the launcher stuttered with Minecraft running.
- The launcher keeps the game's log file open while the game runs instead of reopening it for every line. A large pack prints tens of thousands of lines as it starts.
- An instance without its own memory setting gets more for a large pack: 6 GB from 100 mods, 8 GB from 200, 10 GB from 300, as far as the computer can spare. A world in a large pack loading on 4 GB is what froze until Windows closed it.
- A crash that ran out of memory offers a button that gives the instance more.

### Fixed
- Launching no longer hangs on "Launching..." while Discord is running but not answering, and turning Discord RPC off no longer connects to Discord at all. Thanks to [@loyslow-dev](https://github.com/loyslow-dev) for tracking it down ([#7](https://github.com/Everelsu/Noctrinth/issues/7)).
- A Java that never exits no longer holds a launch forever; the check before launch gives up after 30 seconds.
- A stalled friends connection no longer holds a launch or the end of a game.
- A mod built for another version of Minecraft is recognised again in crashes on modern Java, which words the missing method differently.
- A \`latest.log\` left over from an earlier run is no longer read as if it described a launch that failed before the game started.`,
		body_ru: `### Добавлено
- При краше игры само появляется окно с тем, что нашёл лаунчер, и кнопками, которые это исправляют, — не нужно ждать, пока кто-то откроет вкладку «Журнал».
- Разбор крашей называет мод, в котором произошёл краш: классы из стека вызовов, конфиги миксинов и ошибки загрузчика сопоставляются с jar-файлом в \`mods/\`, из которого они пришли, с кнопкой его отключить.
- Он понимает, как завершилась игра: закрыта Windows за то, что не отвечала, убита системой из-за памяти, нет системной библиотеки или нативный код упал без отчёта.
- Он говорит, какие моды добавлены или удалены с последнего запуска сборки без краша.
- Он находит повреждённые jar-файлы модов, пустые файлы конфигов (и serverconfig мира), один мод, установленный дважды, и процессоры Intel 13-го и 14-го поколений, известные нестабильностью.
- Аддон, собранный под другую версию расширяемого мода, отличается от аддона, у которого не установлена зависимость.
- Распознаётся больше причин: неполная установка, библиотека JNA, заблокированная антивирусом, сломанный датапак KubeJS, повреждённый конфиг сервера, таблица соседей FerriteCore, профайлер Spark на новой Java и новые варианты падения графического драйвера.
- Горячие клавиши в галерее скриншотов: Ctrl+C копирует выбранные скриншоты или тот, что в фокусе; Ctrl+A выделяет все, Delete удаляет, Escape снимает выделение.
- Несколько скриншотов копируются как файлы, как Ctrl+C в Проводнике, так что вставка в папку или чат получает их все. Один по-прежнему копируется как изображение.

### Изменено
- Переключение между лаунчером и игрой больше не заставляет каждую открытую страницу перезагружать данные. Каждое переключение запускало волну запросов — отсюда и подтормаживания лаунчера с запущенным Minecraft.
- Лаунчер держит файл лога игры открытым, пока игра работает, а не открывает его заново на каждую строку. Большая сборка печатает при запуске десятки тысяч строк.
- Сборка без своей настройки памяти получает больше для большой сборки: 6 ГБ от 100 модов, 8 ГБ от 200, 10 ГБ от 300 — насколько компьютер может выделить. Из-за загрузки мира в большой сборке на 4 ГБ игра и зависала, пока Windows её не закрывала.
- Краш из-за нехватки памяти предлагает кнопку, которая даёт сборке больше.

### Исправлено
- Запуск больше не зависает на «Запуск...», когда Discord запущен, но не отвечает, а выключенный Discord RPC больше вообще не подключается к Discord. Спасибо [@loyslow-dev](https://github.com/loyslow-dev) за то, что нашёл причину ([#7](https://github.com/Everelsu/Noctrinth/issues/7)).
- Java, которая никогда не завершается, больше не держит запуск вечно; проверка перед запуском сдаётся через 30 секунд.
- Зависшее соединение с друзьями больше не держит запуск или завершение игры.
- Мод под другую версию Minecraft снова распознаётся в крашах на современной Java, которая иначе формулирует отсутствующий метод.
- \`latest.log\`, оставшийся от прошлого запуска, больше не читается так, будто он описывает запуск, упавший до старта игры.`,
	},
	{
		version: '0.21.6',
		date: '2026-09-28T00:00:00+00:00',
		body: `### Added
- Update all in an instance's Content tab opens a window where each project's new version can be picked and its changelog read before anything is installed.
- A setting to bring the launcher back to the front when Minecraft closes, off by default.

### Changed
- Synced with upstream Modrinth (0.21.4 → 0.21.6).
- Bulk updates run through the download manager, so the Content tab can be left while they install.
- Profile pictures can be up to 512 KiB after compression, up from 256 KiB.

### Fixed
- "Database is locked" errors.
- A modpack no longer stops updating after one of its files is renamed outside the launcher.
- Very old instances whose folder names end in a dot or a space open again.
- Instance pages load faster.
- A locally installed loader no longer needs its metadata downloaded again on launch.
- Symlinked screenshots no longer break the Screenshots page.
- Alt Gr is no longer read as Left Ctrl in keybind inputs on Windows.
- Duplicating an instance works again with command history syncing.
- The download manager no longer always says it needs attention.
- The close button turns red on hover.`,
		body_ru: `### Добавлено
- «Обновить всё» во вкладке «Контент» сборки открывает окно, где для каждого проекта можно выбрать новую версию и прочитать её список изменений до установки.
- Настройка, возвращающая лаунчер на передний план при закрытии Minecraft; по умолчанию выключена.

### Изменено
- Синхронизация с Modrinth (0.21.4 → 0.21.6).
- Массовые обновления идут через управление загрузками, так что с вкладки «Контент» можно уйти, пока они ставятся.
- Аватарки могут весить до 512 КиБ после сжатия вместо 256 КиБ.

### Исправлено
- Ошибки «Database is locked».
- Модпак больше не перестаёт обновляться после того, как один из его файлов переименовали вне лаунчера.
- Очень старые сборки, чьи папки заканчиваются точкой или пробелом, снова открываются.
- Страницы сборок грузятся быстрее.
- Локально установленному загрузчику больше не нужно заново скачивать метаданные при запуске.
- Скриншоты-симлинки больше не ломают страницу скриншотов.
- Alt Gr больше не читается как левый Ctrl в полях назначения клавиш на Windows.
- Создание копии сборки снова работает при синхронизации истории команд.
- Управление загрузками больше не говорит всегда, что требует внимания.
- Кнопка закрытия краснеет при наведении.`,
	},
	{
		version: '0.21.4',
		date: '2026-09-17T00:00:00+00:00',
		body: `### Changed
- Synced with upstream Modrinth (0.21.2 → 0.21.4), two rounds of fixes to the store that arrived with 0.21.2.
- An instance holds its own copy of a file again. The last release said the shared store points several instances at one copy where the filesystem allows it; upstream has turned that off for now, because a couple of mods did not take being opened through a link. Nothing about the store changes otherwise — a mod is still downloaded once — but the disk it saved between instances comes back until upstream puts linking back.
- Minecraft's own libraries are checked on launch, so a file that went missing or arrived damaged is caught before the game does something inexplicable with it. The same check no longer runs against the content store on every launch, which is what was stopping some people from starting the game at all.

### Fixed
- Sliders snap where they look like they should, and stop drifting a step when dragged.`,
		body_ru: `### Изменено
- Синхронизация с Modrinth (0.21.2 → 0.21.4) — два раунда исправлений хранилища, появившегося в 0.21.2.
- Сборка снова держит собственную копию файла. В прошлом релизе говорилось, что общее хранилище указывает нескольким сборкам на одну копию, где файловая система это позволяет; в Modrinth это пока выключили, потому что пара модов не пережила открытие через ссылку. В остальном хранилище не меняется — мод по-прежнему скачивается один раз, — но место на диске, которое оно экономило между сборками, возвращается, пока связывание не вернут.
- Собственные библиотеки Minecraft проверяются при запуске, так что пропавший или повреждённый файл ловится до того, как игра сделает с ним что-то необъяснимое. Та же проверка больше не прогоняется по хранилищу контента при каждом запуске — именно она у некоторых вообще не давала запустить игру.

### Исправлено
- Ползунки встают туда, куда, судя по виду, должны, и больше не съезжают на шаг при перетаскивании.`,
	},
	{
		version: '0.21.2',
		date: '2026-09-15T00:00:00+00:00',
		body: `### Changed
- Synced with upstream Modrinth (0.20.5 → 0.21.2), which rebuilt how content files are kept. Every mod, pack and archive the launcher downloads now lives once in a shared store, and an instance points at that copy rather than holding one of its own — so the same mod in five instances costs the disk what one does. There is a Content storage section in Resource management showing what is unique, what is shared and what nothing uses any more, with a size limit, a clear button and a check-and-repair pass for files that went missing or bad. Installs got a download manager with speeds, time remaining and pause, resume and cancel.
- The fork's own store of downloaded files is gone, and so is the reuse it did when a modpack changed version. Both existed because an update deleted every file the old pack listed and downloaded the whole new one back; upstream now answers the same question and answers it further. A file the store already holds is placed into an instance by reflink or hard link where the filesystem allows one, so an unchanged mod costs neither bandwidth nor disk, where the fork's version still copied it. What it kept was limited to a size this fork picked and never shared between instances.
- Downloads no longer stage through a directory of the fork's own. Upstream stages inside its store, which is under the launcher's folder for the same reason the fork's was, and clears out what an interrupted run leaves behind.
- A file dragged into an instance under a name the filesystem cannot take is now refused rather than quietly renamed. That is upstream's rule, applied where content arrives; a modpack's own files are still cleaned up on the way in, so a CurseForge pack carrying such a name installs as before.`,
		body_ru: `### Изменено
- Синхронизация с Modrinth (0.20.5 → 0.21.2), где переделано хранение файлов контента. Каждый мод, пак и архив, который скачивает лаунчер, теперь живёт в одном экземпляре в общем хранилище, а сборка указывает на эту копию, а не держит свою, — так что один мод в пяти сборках занимает на диске столько же, сколько в одной. В «Управлении ресурсами» есть раздел «Хранилище контента», показывающий, что своё, что общее, а что уже ничем не используется, с лимитом размера, кнопкой очистки и проверкой с исправлением пропавших или испорченных файлов. У установок появилось управление загрузками со скоростью, оставшимся временем, паузой, продолжением и отменой.
- Собственное хранилище скачанных файлов форка убрано, а с ним и переиспользование при смене версии модпака. Оба существовали потому, что обновление удаляло все файлы старого пака и скачивало весь новый заново; теперь на тот же вопрос отвечает Modrinth, и отвечает полнее. Файл, который уже есть в хранилище, помещается в сборку reflink'ом или жёсткой ссылкой, где файловая система это позволяет, так что неизменённый мод не тратит ни трафик, ни диск, тогда как версия форка всё равно его копировала. Хранила она лишь до выбранного форком размера и не делилась между сборками.
- Загрузки больше не проходят через собственную папку форка. Modrinth складывает их внутри своего хранилища, которое лежит в папке лаунчера по той же причине, что и у форка, и убирает то, что оставляет прерванный запуск.
- Файл, перетащенный в сборку под именем, которое файловая система не принимает, теперь отклоняется, а не тихо переименовывается. Это правило Modrinth, применяемое там, где контент поступает; собственные файлы модпака по-прежнему чистятся при установке, так что пак CurseForge с таким именем ставится как раньше.`,
	},
	{
		version: '0.20.5',
		date: '2026-09-14T00:00:00+00:00',
		body: `### Changed
- Synced with upstream Modrinth (0.20.4 → 0.20.5). Tooltips are its own work now, in place of the library it had been using: they wait before appearing, do not re-open when you sweep past a neighbouring one, move between positions instead of jumping, and take a moment to close. Focus no longer escapes an open dialog, and the action bar in the modpack content modal stops drawing underneath it.
- The fork's cure for tooltips that would not go away was rebuilt on what replaced the library it was written against. The case it handled best — something unmounted while its tooltip is up — upstream now handles in the tooltip itself, which is the better place for it. What stays here are the three moments a tooltip cannot see: the window losing focus, the pointer leaving the window rather than moving to something inside it, and a navigation past an element that outlives the route it was on.`,
	},
	{
		version: '0.20.4',
		date: '2026-09-13T00:00:00+00:00',
		body: `### Added
- An exported .mrpack carries the instance's picture. The export walks the instance folder and an icon is not in it — it is cached beside the launcher's other state — so every pack made here arrived somewhere else blank. It goes in where Modrinth's own launcher looks for it, and where a launcher that has never heard of pack icons simply unpacks it into the instance folder and finds an icon again. A checkbox, on by default. An icon generated in the editor needs nothing special: it is a real 256-pixel picture in the same place.
- An animated icon survives the trip. It goes in a second time under its own extension, because whether an icon keeps moving is decided by the name, so a GIF under the name compatibility demands comes back as a single frame. Importing prefers the one that still moves.

### Changed
- Synced with upstream Modrinth (0.20.2 → 0.20.4). Both are bugfix releases; nothing new came with them.
- The crash reader recognises fifty-three things instead of twenty-six, and rules can now say what they are *not* about. Among the new ones: a name the game cannot use getting into a path, which is what happens when the Windows account is named in an alphabet other than Latin and is a crash that only ever finds some people; dependency ranges quoted with both versions; a mixin that failed told apart from two mixins that collided; Java too new as distinct from too old; a world held shut by a datapack; a damaged chunk; an unreadable config; the memory allocator; the sound stack; the graphics libraries of Linux and macOS; the wrong Java build on an Apple Silicon Mac; Oculus without Embeddium; Sodium without Indium; ids run out on 1.12; a server thread that stopped answering; and world generation waiting on itself.
- A release build now records how its own start-up went, in the launcher log. The measuring was already there and had been switched off everywhere except development builds, which is the one place a slow start is nobody's complaint.
- Resource pack syncing now reconciles an instance in the background, in a queue that retries on its own, rather than doing the work in front of whatever asked for it. Launching an instance and opening the app were both waiting on it.

### Fixed
- The "Sync your settings" notice stops coming back after it has been dismissed. The dismissal is written down now instead of being remembered only until the window closes.
- An instance that belongs to a server project starts from Jump In. Its Play button ran the instance directly, which is not what a server project instance is launched by, so it did nothing that lasted.
- A migration upstream shipped without is included, so the table behind a server-linked instance's server list is the shape the code expects.
- An offline account appears once it is added. It was being written down correctly and then had nowhere to show: the sidebar's "Playing as" card only appears once the getting-started checklist counts you as signed in to Minecraft, and only the Microsoft and Ely.by sign-ins ever said so. Adding a name says so too now. An add that genuinely fails says why, instead of leaving the dialog sitting there.
- Java is not downloaded again when it is already installed. The launcher looked for it in its settings rather than on disk, so anything that lost the setting — an install interrupted after the archive was unpacked, most of all — meant fetching the whole runtime a second time. Worse, unpacking deletes the installed copy first, so each of those attempts took away the Java that was working until the new one landed. The runtime already sitting in the launcher's Java folder is used.
- A download is staged inside the launcher's own folder instead of Windows' temporary one. Every file arrives in a temporary file before it is checked and moved into place, and that file was going to the system drive no matter where the launcher was put — so installing a large modpack wrote all of it to the drive it was moved off of, and a full disk ended the install without saying what filled it. Anything an interrupted run leaves behind is cleared out a day later.`,
	},
	{
		version: '0.20.2',
		date: '2026-09-11T00:00:00+00:00',
		body: `### Added
- An imported CurseForge instance is checked against the list of mods it carries, and anything the folder did not actually have is fetched from CurseForge: a mod the CurseForge app never finished downloading, a pack copied off another machine without its mods folder, a file that came across cut short. The instance arrives whole instead of a mod short with nothing to show for it.
- A mod restored that way is recorded with the CurseForge project it came from, so it can be updated from the Content tab like any other mod instead of sitting there as an anonymous jar. A mod that was switched off in CurseForge comes back switched off.
- A file that cannot be fetched at all is named when the import finishes, so a pack that is genuinely missing something says so rather than quietly launching without it.
- Content that came from CurseForge is named. A mod installed from a CurseForge pack showed its filename, "Unknown" for a version and no author at all, because the launcher only ever asks Modrinth what an installed file is and a CurseForge jar is not there to be asked about. It carries the ids it was installed with, so CurseForge is asked instead: the mod's real title, its icon, its author with their picture, and the version name CurseForge itself shows. Both link out to CurseForge rather than into a page in the app that cannot exist.
- Answers are kept for a week in the launcher's own database, so a pack of several hundred mods is looked up once rather than every time its Content tab is opened. A build with no CurseForge key configured, or no connection, draws exactly what it drew before — filenames — rather than failing.
- An instance icon that is an animated GIF stays animated. Every icon used to be decoded and written back out as a PNG, which for a GIF means keeping the first frame and throwing the rest away — a mod's icon moves because it is drawn straight from its URL and never goes near that. A GIF within the same size limits every other icon is held to is now kept exactly as it arrived, which includes the picture a CurseForge pack ships; one too large still falls back to the still frame rather than being refused. The library grid draws it too: it builds a small PNG of every icon so a wall of cards stays cheap, and that alone was quietly flattening the one place the icons are most on show.
- A CurseForge modpack installed from a .zip takes the pack's own picture as its instance icon — the image file in the zip if it carries one, and otherwise the picture its manifest links to, which for most packs is the only copy there is. Reinstalling over an instance you have since given an icon of your own leaves that icon alone.

### Changed
- Synced with upstream Modrinth (0.20.0 → 0.20.2): the app and its settings window open noticeably faster, and the "New update" notice stops coming back once it has been dismissed.
- Game settings gained a great deal with it: a keybinding names the mod it belongs to and links to that mod's Modrinth page or its file in the Content tab, keybindings can be searched by mod name or by filename, mod keybindings are shown under the translated names carried in the mod's own jar in the launcher's language, and Language became a searchable dropdown listing language names and regions.
- Memory sliders read from 0MB upwards in GB and no longer cut large values off, sliders were redrawn throughout, and a slider's number field updates properly after a value has been rounded or held to the allowed range.
- Resource packs in the Content tab and multiplayer servers in the Worlds tab carry sync badges of their own.

### Fixed
- A modpack installed from a CurseForge .zip shows its mods as the pack's content. They were recorded as plain CurseForge files, which is what a mod you add from CurseForge yourself is, so "Pack content" came up empty while all several hundred of the pack's mods were filed under "Additional content" beside it. The pack's own files are marked as the pack's now, and an instance installed before this is corrected on first launch rather than having to be reinstalled.
- An imported CurseForge instance keeps its picture. CurseForge records it as an absolute path into its own installation, so it was dead as soon as the folder moved or CurseForge was uninstalled — and because the path and the modpack thumbnail were written as one either/or, a dead path also ruled out the thumbnail that would have worked. The folder being imported is searched now, and the thumbnail is still there to fall back on.
- From upstream: field of view and mouse sensitivity were saved and shown wrongly on Minecraft 1.19 and newer, setting up resource pack syncing failed outright when a single pack had malformed metadata, quick instances pushed the sidebar's navigation buttons off-screen in a small window, compact library cards had their bottom borders clipped, sync source pickers showed a loading screen over a list they already had, and the Screenshots breadcrumb icon did not match its sidebar icon.`,
	},
	{
		version: '0.20.0',
		date: '2026-09-08T00:00:00+00:00',
		body: `### Changed
- Synced with upstream Modrinth (0.19.2 → 0.20.0): Minecraft's own settings are edited and shared between instances from the launcher, with per-instance overrides and a source instance to take them from; resource packs can be shared the same way, off by default; a Features tab collects the parts of the interface that can be turned off; and a long list of memory fixes for the library, skins, screenshots, installs and instance icons.
- Noctrinth's own options profile is gone, and upstream's settings syncing takes its place: it does everything the profile did and holds a version's own options against the ones being shared, which the profile could only guess at. The options it was applying stay in each instance's options.txt; what to share is now chosen in Settings → Instances.
- The wheel zoom and the up-and-down tilt of the skin preview moved with it into the component upstream split out.

### Fixed
- Skins for every player is a setting again. Upstream rewrote the page it lived on.`,
	},
	{
		version: '0.19.2',
		date: '2026-09-03T00:00:00+00:00',
		body: `### Added
- Signing in to Minecraft falls back to the classic Xbox Live flow when Microsoft's Sisu service is unavailable. It runs on different servers, which are usually up when Sisu is not.

### Changed
- Synced with upstream Modrinth (0.19.1 → 0.19.2): an instance's hooks, window and behaviour settings are gathered into a Behavior tab, syncing an option between instances asks which instance to take it from, the Screenshots page stays smooth with thousands of screenshots in it, turning a Java override off puts the recommended installation back, and an instance's page no longer shows what it knew before the instance was last edited.
- An instance installed from a modpack starts with the launcher's shared settings kept out of it: the options profile skips it and every synced option is off, so the pack's own options.txt, server list and hotbars are not written over on the first launch. An instance you made yourself starts as it always did, and each of these is still a switch in the instance's own settings.
- A sign-in request that fails because Microsoft is having trouble is tried again with a growing wait, and an outage on their side says so instead of arriving as a JSON parsing error.
- The Minecraft token is refreshed when it is about to expire, once a day, rather than every hour on the Microsoft one behind it. An account that is already signed in keeps working while Xbox Live is down.

### Fixed
- Licensed players stayed Steve on offline-mode servers whenever Ely.by's proxy to Mojang was slow or failing. Every skin source is now asked at once rather than one after another, so a source that hangs no longer runs out the clock before Mojang has answered, and one that is failing is left alone for a minute.`,
	},
	{
		version: '0.19.1',
		date: '2026-08-28T00:00:00+00:00',
		body: `### Changed
- Synced with upstream Modrinth (0.18.2 → 0.19.1): a Screenshots page in the sidebar that gathers every instance's screenshots in one place, an editor that annotates, doodles on and blurs them, switching between several signed-in Modrinth accounts, syncing of command history, the in-game server list and creative hotbars between instances, right-click actions on "Jump in" cards, context menus drawn like overflow menus, a splash screen that follows the colour theme, and fixes for icon padding, window controls behind the splash screen, snapshot grouping since 26.1, cut-off settings headers and text inputs.
- Spoilers, code blocks and inline snippets in descriptions are drawn to upstream's new style, and fourth-level headings stand apart from body text.
- Screenshots are upstream's page now: grouped by date, instance or a group of your own, searchable, exported as a ZIP and edited in place. An instance can be kept out of it, which gives that instance a tab of its own again.
- Skins for every player moved to Settings → Instances, where upstream folded the rest of the default instance options.
- Modern Java is a section of its own at the top of an instance's Java tab, above the installation, memory, arguments and environment sections upstream now leads with a toggle.
- Collections and Notifications are still in the account menu, beside upstream's new profile, Modrinth+, add-a-friend and account-switching entries.

### Removed
- Noctrinth's own screenshots tab, and the strings that went with it. Upstream's page does more, and does it for every instance at once.

### Fixed
- The Screenshots page and tab lagged with a lot of screenshots in them.`,
	},
	{
		version: '0.18.2',
		date: '2026-08-21T00:00:00+00:00',
		body: `### Added
- Accent presets in Settings → Appearance: Theme, Amethyst, Nightshade, Midnight, Glacier, Verdant, Lantern, Ember and Rose, each drawn deeper on light themes and brighter on dark ones.
- A toggle beside the presets that tints the app's backgrounds and panels with the chosen colour.
- The splash screen and the window icon are drawn in the chosen accent.
- The wordmark in the title bar turns and sweeps while a download, an install or a page load is running.
- Signing in to Minecraft from the getting started checklist asks which account to add: Microsoft or Ely.by.
- A folder of the launcher's own for skins put there by hand: one \`<name>.png\` per player, with \`capes/\` and \`elytras/\` beside it. It is asked before any skin system, so it also overrides one, and \`<name>-slim.png\` is drawn on the thin model.
- An instance that is already running can be launched a second time, from the button beside Stop on its page. The game starts as whichever account is signed in now, so switching accounts between the two is how somebody ends up in the same world twice.
- The running instances list names the account each copy is signed in as.
- Each copy of a running instance has a console of its own on the Logs tab, side by side and headed by the account it is signed in as. One copy, or none, is the page as it was.
- Micropatches: a release can be rebuilt and published again under the number it already went out as, and installs already on that version take the update. Add a build number to \`VERSION\` (\`0.18.2\` becomes \`0.18.2+1\`), then run the release workflow on the release's own tag with Micropatch ticked.

### Changed
- The empty home screen says Noctrinth and shows the fork's own mark in the accent colour, instead of Modrinth's name and icon.
- Synced with upstream Modrinth (0.18.0 → 0.18.2): the pagination ellipsis jumps to a page, compact mode for instances in the library, a drag handle to resize "Jump in", icon editing from an instance's context menu, sorting by loader and game version, appearance and behaviour settings synced across devices, depends-on and included-content search filters, and fixes for installed state, "Jump in" cards, tab bars, a Discover memory leak and \`.mrpack\` exports.
- Noctrinth's translations moved to \`src/locales-noctrinth/\` and are applied over upstream's catalogues instead of being edited into them.
- The changelog is one file again; screenshots are still published with the changelog site rather than shipped in the installer.
- The in-app changelog is no longer fetched or cached at runtime — it is the one the build shipped with.
- Collections and Notifications are laid out in the cards, surfaces and spacing the rest of the launcher uses, rather than as a page inside one large box.
- The Collections / Notifications tab row is gone from both pages; the account menu is how you get to either.
- Launching on an Ely.by account no longer waits on Ely.by. The agent is handed the address of the account system's API and what it says about itself, which it used to go and fetch twice before the game started, and a token Ely.by vouched for in the last ten minutes is taken at its word. Both are also what a launch without a connection had been tripping over.

### Fixed
- The launcher could die on a cold start with "should be initialized when used", and open fine the next time.
- Signing in with an Ely.by account left the checklist's "Sign in to Minecraft" step outstanding.
- Closing the app panicked with "there is no reactor running" instead of exiting.
- DNS failures on Windows were logged as errors on every background poll.
- The right-hand sidebar, promo cards, dialog dimmer and the far end of the loading bar stayed purple whatever accent was chosen.
- The empty-server page's tiles and selected backups on the hosting page were drawn in Modrinth green.
- The launcher refused to start after the sync: upstream and Noctrinth had both shipped a migration numbered 20260818120000. Upstream's is renumbered, and a duplicate version now fails at startup naming both files, and in CI.
- Players signed in with a different account system than yours — an Ely.by player seen from a licensed client, or the other way about — stayed Steve on offline-mode servers.
- The by-name skin lookup did nothing at all on Minecraft 1.20.2 and newer, where the game no longer asks for a profile's textures but for the property they are packed in.
- On those versions an Ely.by skin was dropped even when the server did send it, because the game keeps a list of the domains a skin may be hosted on and Ely.by is not on it.
- Licensed players stayed Steve whenever Ely.by's proxy to Mojang did not answer, which is often; Mojang is now asked directly for the names Ely.by has never heard of.
- A player who changed their skin and rejoined kept the old one for another ten minutes. A skin is now current for fifteen seconds, and looked at again in the background after that.
- Everyone came back as Steve for a moment whenever they walked back into view, and everyone at once after a death, while the game waited on a fresh lookup. A skin already known is now handed over immediately.
- Every session started by looking everyone up over again; what was found is written down now and read back at the next launch.
- A skin system that would not answer could hold a frame up for ten seconds while its timeouts ran out one after another. The game is made to wait five at the most, and the lookup finishes on its own.
- The player's own arm was Steve's for the first moment after spawning: their own skin is now looked up while the game is still starting.
- A licensed player's skin cost two requests to Mojang every time it was looked at; the id behind the name is now remembered.
- Names an offline server can hand out but Mojang would not issue, such as one with a dash in it, were never looked up.
- On Minecraft 1.20.2 and newer, a skin looked up by name was never drawn at all: the client asks for a player's skin as trusted or not, and puts Steve in the place of one it does not count as trusted.
- The console of the copy that started first stayed empty: the two commands a per-copy console calls were never declared, so the app refused them and the pane had nothing to show.
- An Ely.by session token stood in an instance's log in full, and went out with it whenever a log was shared: only Microsoft tokens were ever removed. Ely.by's are too now, and the session line the game writes for itself is blanked whatever account it belongs to.
- Clearing the live log left it empty for good — clearing the output also forgot which copies of the instance were running, so nothing could be read back until the game was started again.
- A release's notes on GitHub stayed as GitHub generated them: the workflow was still looking for the changelog in the per-release Markdown files it had moved out of.
- The app's icon files still carried the old mark, so the installer, the taskbar and the dock showed it.`,
	},
	{
		version: '0.18.0',
		date: '2026-08-19T00:00:00+00:00',
		body: `### Added
- Ely.by skins are managed inside the launcher: the account's catalogue is browsable in the same grid the Microsoft account uses, with preview, Reset and Apply.
![Ely Skins](/changelog/screenshots/0.18.0_elybyskin.png)
- Skins can be uploaded to Ely.by and removed from it without leaving the app.
- Skins for every player on offline-mode servers, looked up by name from Ely.by with a Mojang fallback. No mods needed; turn it off under Settings → Default instance options. [BETA]
- One-click modern Java for 1.7.10, installing [lwjgl3ify](https://github.com/GTNewHorizons/lwjgl3ify) or [Cleanroom](https://github.com/CleanroomMC/Cleanroom) along with the launcher-side patches. Disabling puts the instance back. [BETA]
- A graphics adapter picker for each Java runtime, on Windows, carried over when that runtime updates.
![Java Graphs](/changelog/screenshots/0.18.0_javagraphicsadapter.png)
- A Java runtime manager listing what is installed, with the option to remove it.
- Changelog entries render as markdown, so links and screenshots work.
- A notification for update downloads, carrying the version, how much has arrived and how large it is.

### Changed
- Synced with upstream Modrinth (0.17.10 → 0.18.0): a new Play page replacing Home and Library, an instance icon creator, a "Getting started" checklist, a clearer create-instance menu that can search projects, skin previews before signing in, "Managed content" renamed to "Provided content", and fixes for hooks and environment variables, instance icons, unlinked instance updates, the project gallery viewer, content-tab flickering and macOS LAN play.
![Lib](/changelog/screenshots/0.18.0_mainpage.png)
- Instance content search moved onto upstream's new library toolbar; the query language is unchanged and the content index is only read when a query needs it.
- The Java settings page matches the rest of the app, and the runtime selector is laid out as a column.
- The graphics adapter row is hidden on machines with one adapter instead of shown disabled.
- The fork's own strings speak Russian: modern Java, the Ely.by panel, the library search language, the graphics adapter picker and the Minecraft options screen.
- Reworked the Minecraft options screen: one value per slider, aligned controls, and version constraints written out in words.
- The instance Play button is brand-coloured, like the rows beside it in "Jump in".
- The version is written once, in the \`VERSION\` file at the root; a hand-edited copy fails CI.

### Fixed
- Applying an Ely.by skin appeared to hang and then opened the website; both applying and uploading now wait for the worn skin to change before assuming a missing session.
- The Ely.by panel waited for every texture before drawing anything; tiles appear immediately now.
- A dropped connection while polling the skin list raised a notification every time.
- An update whose files were not published yet was never retried until a restart.
- A network blip could write over a hundred \`ERROR\` lines carrying nothing but an OS error code.
- The keyboard selection in instance search suggestions was invisible on the OLED theme.
- A long server status line stretched its row in "Jump in" on longer translations.`,
	},
	{
		version: '0.17.10',
		date: '2026-08-15T00:00:00+00:00',
		body: `### Changed
- Synced with upstream Modrinth (0.17.8 → 0.17.10): pages failing to load while signed out.

### Fixed
- Sharing an instance reported "Unable to connect to shared instances API" whatever had gone wrong; the real reason is shown now.`,
	},
	{
		version: '0.17.8',
		date: '2026-08-15T00:00:00+00:00',
		body: `### Changed
- Synced with upstream Modrinth (0.17.7 → 0.17.8): Ears skins render correctly, \`.mrpack\` exports write the right environment values, and an instance linked to a deleted modpack version shows its managed content card again.

### Fixed
- Updating on Linux failed with "Permission denied (os error 13)"; Debian and RPM installs now update through the system's own privilege prompt.
- An update that cannot be installed is refused before it downloads, naming the directory, instead of failing after fetching it.

### Note for Linux
- If you installed Noctrinth from the \`.deb\`, this update cannot install itself. Install this release's \`.deb\` once by hand; updates after it work normally.`,
	},
	{
		version: '0.17.7',
		date: '2026-08-14T00:00:00+00:00',
		body: `### Added
- Search across every instance at once from the library: \`@sodium\` matches instances with a mod, \`#shader\` filters by content type, \`!outdated\` by state, terms combine, and a leading \`-\` flips one around.
- Suggestions as you type in that field: the sigils when empty, installed mods after \`@\`, types and states after \`#\` and \`!\`.
- A shared Minecraft options profile under Settings → Minecraft options, written into every instance's \`options.txt\` at launch, with per-option opt-in.
- "Copy link" in the overflow menu on project pages.

### Changed
- Synced with upstream Modrinth (0.17.6 → 0.17.7).
- Follow and Save left the project overflow menu; both have buttons in the header.

### Fixed
- The Save-to-collection panel opened in the top-left corner of the window, with a wrongly sized button.`,
	},
	{
		version: '0.17.6',
		date: '2026-08-14T00:00:00+00:00',
		body: `### Added
- Synced with upstream Modrinth (0.17.5 → 0.17.6): projects carry content disclosures — AI-generated content, ads, paid features, telemetry, derivative content and photosensitivity hazards — and can be filtered out when browsing.

### Changed
- The "Advanced" group of search filters is now "Advanced exclusions" and holds the new disclosure filters.
- Archived projects are marked by a disclosure rather than by their visibility.

### Fixed
- The "more options" menu on project pages opened empty.
- Adding a local file to an instance works for archives upstream's content inspection refuses: plain \`.jar\` and \`.litemod\` files without loader metadata, and packs zipped inside a wrapping folder.`,
	},
	{
		version: '0.17.5',
		date: '2026-08-14T00:00:00+00:00',
		body: `### Changed
- Synced with upstream Modrinth (0.17.4 → 0.17.5).
- An instance's content tab checks for updates when opened instead of waiting for a cached answer to expire.
- Config and RPO files are no longer flagged when importing a modpack from outside Modrinth, and the remaining warning explains itself better.
- Translations refreshed from upstream's Crowdin pull, without putting the Modrinth name back where Noctrinth had renamed it.

### Fixed
- Animated GIFs in project descriptions play again.
- Clicking an entry in an instance's content tab reported "Instance not found".
- The window close button is the right colour on Windows and Linux.
- Download buttons on version pages show the file name and size again.
- The buttons on the "Minecraft account required" dialog work on Windows.
- The right sidebar stays open when collapsing a friends-list section with "Hide right sidebar" on.`,
	},
	{
		version: '0.17.4',
		date: '2026-08-10T00:00:00+00:00',
		body: `### Added
- Modrinth App migration: a banner on the home page imports all of its instances or a hand-picked few, read straight out of its database, with an option to remove each one from Modrinth App once its copy has finished.
- CurseForge modpack \`.zip\` import from disk is back, on the same job pipeline as everything else. Search, browse and project pages stay removed.
- A link to the fork's author on the Changelog settings tab.
- Environment variables for an instance's pre- and post-launch hooks (from upstream).

### Changed
- Synced with upstream Modrinth (0.17.3 → 0.17.4): buttons rebuilt on a new shared component set, an "Inspecting modpack" progress bar on file import, friends lists collapsing offline members by default, better popout animations and noticeably lower memory use.
- The top-left wordmark dropped "app", matching Modrinth's own rebrand.
- The "Beta" badge on the Language settings tab is purple instead of green.
- Byte-size units (KiB/MiB/GiB/TiB) are shown in English.
- The instance Screenshots tab was rebuilt to match the project Gallery, with zoom, copy, reveal and delete in one floating control bar.
- The embedded Ely.by skin window hides instead of closing, so a sign-in lasts the session.
- Russian translations refreshed from upstream's Crowdin pull.

### Fixed
- Install links did nothing: they said \`modrinth://\` while the app answers to \`noctrinth://\`, and its own parser rejected \`noctrinth://\`.
- Install links on modrinth.com open Noctrinth: the app claims \`modrinth://\` alongside its own scheme on every launch.
- Ctrl+C and Ctrl+A failed in instance logs and consoles on non-Latin keyboard layouts.
- Uploading a profile picture rejected anything over 256 KiB; oversized images are scaled and compressed to fit.
- The instance Screenshots tab errored with "Cannot read properties of undefined".
- The account menu had a blank row above "Sign out", and Collections and Notifications had no accessible label.
- Upstream fixes from the sync: editing your profile in-app, stale Microsoft tokens signing the account out, \`.mrpack\` exports past 4 GB, export modal paths, \`.nbt\` config files in shared instances, and the version list freezing on game-version toggles.`,
	},
	{
		version: '0.17.3',
		date: '2026-07-31T00:00:00+00:00',
		body: `### Added
- Synced with upstream Modrinth (0.15.11 → 0.17.x): user profile pages in the app, shared instances with invites and updates, an instance Share tab, settings grouped into Display / Account / Instances, a new breadcrumbs system, a rebuilt project page header, instance quarantine handling, a toggle to hide installed modpacks, and a fix for search jumping back to page 1.

### Changed
- Upstream is the source of truth: the fork's custom Modrinth OAuth flow, browse page-reset patch and feature-flag redesign gave way to upstream's.
- The proxy setting and Noctrinth branding were carried into upstream's new settings layout; the changelog now lives under Display.

### Removed
- CurseForge integration: search, installs, project pages, the catalog toggle, the fingerprint and mapping helpers and the backend installer. Importing a CurseForge instance from another launcher still works.
- Modrinth's ads stay disabled, including the new consent popup and Modrinth+ upsell.

### Fixed
- The Screenshots tab listed nothing, passing an instance path where the backend expects an id.
- A byte-order mark in a shared UI component and a locale file, and a dead OAuth loopback listener.`,
	},
	{
		version: '0.15.11',
		date: '2026-07-15T00:00:00+00:00',
		body: `### Changed
- Synced with upstream Modrinth (0.15.10 → 0.15.11): malware warning modal changes, shift-click to toggle file selection, shader configs renamed on version change, dependents search in Discover, download modal fixes and new moderation keybinds.
- The Ely.by sign-in dialog matches the app's standard modals.
- The changelog tab uses the standard chip selector, and each source's "open full changelog" link points at the right site.
- The screenshot counter is plain text instead of a pill badge.

### Fixed
- Local files failed to add with "Unable to infer project type": plain \`.jar\` files without loader metadata are accepted as mods, packs zipped inside a wrapping folder are recognised, and filenames Windows forbids are sanitised.`,
	},
	{
		version: '0.15.10',
		date: '2026-07-14T00:00:00+00:00',
		body: `### Added
- Two-factor authentication (TOTP) for Ely.by sign-in.
- Failed required CurseForge dependencies are reported after an install instead of being skipped silently.
- The Versions tab of CurseForge project pages pages through the full file list instead of the 50 newest.

### Changed
- CurseForge modpack installs run through the launcher's install-job pipeline: queued, cancellable, retryable, with live per-file progress, parallel downloads and one error listing every failure.
- Synced with upstream Modrinth (0.15.1 → 0.15.10): an advanced filter category on Discover, redesigned version pages, download modal and modpack export modal, the "Chaos Cubed" skin pack, better install error handling with a "Copy details" button and a three-job queue, and connect and read timeouts on all launcher HTTP requests.
- Upstream fixes from the sync: freezes when opening instance pages, instance edits not appearing saved, content desync on enabling or removing mods, a 10-minute search cache, environment filters in Discover, links keeping track of the instance you came from, and a Files tab memory leak.
- "Update all" for CurseForge checks several mods at a time and skips disabled files.
- CurseForge install notifications on project pages are translated.
- Noctrinth's CurseForge search filters, install pipeline and proxy support were re-integrated on upstream's reworked search and version page.

### Fixed
- Importing a CurseForge modpack from a local \`.zip\` reported "no CurseForge API key is available" with a key configured, and the preview now shows the pack's real name, version and loader.
- A flaky connection or an Ely.by outage no longer signs you out; credentials are dropped only when both the token and its refresh are rejected.
- Updating a CurseForge mod deleted the old file before the new one had downloaded.
- Install buttons were disabled for files whose author hid the download URL.
- Links inside CurseForge descriptions pointing to other CurseForge pages opened a dead app-origin URL.
- The Oculus entry in the cross-platform mapping used a slug instead of a Modrinth project id.`,
	},
	{
		version: '0.15.1',
		date: '2026-06-27T00:00:00+00:00',
		body: `### Added
- CurseForge modpack installation on the new 0.15 content pipeline: the instance is created from the manifest, files are resolved and downloaded with a CDN fallback, overrides applied and Minecraft installed.
- A manual download fallback for files whose author disabled third-party distribution: an embedded CurseForge window where one download is captured and imported automatically.

### Changed
- Synced with upstream Modrinth (0.14.8 → 0.15.1): the backend was rewritten, instances and their content are managed through a new content pipeline, the old "profile" model is gone, and installs run as background jobs.
- Ely.by account support was re-ported onto the new backend.
- CurseForge files are tracked natively in the launcher database, so installed state and updates survive restarts and backups.
- A required CurseForge dependency that also exists on Modrinth is installed from Modrinth.
- "Update all" is provider-aware: CurseForge content updates through CurseForge, Modrinth content through Modrinth.

### Fixed
- Discover jumped back to the first page when installing a mod with "hide installed" on.
- CurseForge installs crashed with "Unknown instance" from every entry point.
- The CurseForge version and loader filter was ignored when browsing inside an instance.`,
	},
	{
		version: '0.14.8',
		date: '2026-06-20T00:00:00+00:00',
		body: `### Changed
- CurseForge search is off by default; turn it on in Settings → Appearance.
- Synced with upstream Modrinth (0.14.6 → 0.14.8): reworked content dependency handling, more reliable update matching, game-version search fixes, Maven version-range matching, modpack export filtering, Babric mods no longer misdetected as Fabric, the mobile floating action bar, comboboxes closing on scrollbar clicks, and billing and analytics fixes.

### Fixed
- CurseForge browse failed on later pages; the page size is clamped to the 10,000-result window.
- Switching catalogs resets to the first page instead of landing on a blank one.
- CurseForge project pages opened from Discover keep a working breadcrumb.
- Restored the Pride Fundraiser banner.
- Russian translations for CurseForge pages and the remaining settings, instance, world and friend-request strings.`,
	},
	{
		version: '0.14.6',
		date: '2026-06-12T00:00:00+00:00',
		body: `### Added
- Ely.by skin management in the app: "Change skin" opens an embedded Ely.by window, and the preview shows the account's cape, detects slim or classic arms and refreshes on demand.
- A proxy setting in Settings → Resource management, routing all launcher traffic through one \`http\`, \`https\`, \`socks5\` or \`socks5h\` URL. Applies after a restart.
- A CurseForge master toggle in Settings → Appearance covering the catalog toggle, search results and project pages.

### Changed
- Modrinth and CurseForge are fully separate catalogs: the toggle picks exactly one, including for text searches, and the sidebar shows only filters CurseForge supports.
- CurseForge downloads are verified against SHA1 checksums, retried, and given a CDN mirror fallback; modpack installs no longer skip such files silently.
- Synced with upstream Modrinth (0.14.5 → 0.14.6): drag-and-drop skin reordering, draggable Mojang defaults, skin outer-layer translucency, offline handling on the Skins page, live system-theme updates, new Microsoft sign-in error cases, and fixes for notifications over modals and skin editing.

### Fixed
- The Ely.by skin preview hung on "Loading", and the account picker avatar did not update after a skin change.
- Russian translations for the new settings and the Ely.by skin page.
- Startup logged a spurious "User is not logged in" error with an Ely.by account active.`,
	},
	{
		version: '0.14.5',
		date: '2026-06-09T00:00:00+00:00',
		body: `### Changed
- Synced with upstream Modrinth (0.14.3 → 0.14.5): a "release channels" instance setting, a reworked updater UI with the action in the top-right bar and pop-ups only after 24 hours, uncommon plugin loaders behind "Show more", and analytics and translation updates.
- The update pop-up's "Changelog" button opens the Noctrinth GitHub page.
- Changelog website timestamps recalculate on every visit instead of being frozen at build time, with the exact date on hover.

### Fixed
- Drag-and-drop file upload in the instance Files tab.
- The expanded console in the Logs tab rendered behind other elements.
- The updater restarted the app when no restart was requested.`,
	},
	{
		version: '0.14.3',
		date: '2026-06-07T00:00:00+00:00',
		body: `### Added
- A Screenshots tab for instances: browse, zoom, copy, reveal in explorer and delete.

### Changed
- Synced with upstream Modrinth (0.14.1 → 0.14.3): a play-time toggle in the instance header, the Mr. Pack default skin, the Pride Fundraiser banner, tooltip and card styling, a large \`.mrpack\` import crash fix, a content-update modal fix, a Windows path separator fix in the Files tab, and translation updates.
- The update popup's changelog button links to the Noctrinth GitHub page.
- Download notification icons match Noctrinth's purple.

### Fixed
- Russian locale updated with the missing skin selector, collections, notifications and screenshots strings.`,
	},
	{
		version: '0.14.0',
		date: '2026-05-29T00:00:00+00:00',
		body: `### Added
- CurseForge install detection through the fingerprint API, catching mods dropped into the folder by hand, installed before this version, or restored from a backup.
- A cross-platform mapping of Modrinth ↔ CurseForge ids, so a mod on both platforms is one card in unified search and installing one marks the other installed.
- A CurseForge dependency resolver: embedded and tool dependencies are skipped, incompatible mods warned about, optional ones offered rather than installed, and required ones kept only when every ancestor was required too.
- CurseForge categories for the shader, resource pack and datapack tabs.
- Clicking the "…" in pagination turns it into a page input.
- A CurseForge "Alpha" badge on the source toggle.
- Russian translations for the account dropdown, Collections, Notifications, sidebar tooltips, the Ely.by login modal, the Changelog tab and the dashboard nav.
- Search and sort for the Followed-projects view and collections, remembered across reloads.

### Changed
- Synced with upstream Modrinth 0.14.0: the new skin selector out of beta, skin preview improvements, project analytics performance, Daedalus manifest uploads and CSP changes.
- The skin preview tilts around its pitch axis as well as yaw, and the shadow follows the model.
- The CurseForge content sidebar uses a curated Modrinth-style category list.
- Compact number suffixes are Latin (\`k\`, \`m\`, \`b\`) while the numbers stay locale-correct.

### Fixed
- Importing an unrecognised modpack file deleted the instance; the error now comes before anything is created.
- Modpacks shipping filenames Windows forbids crashed the install with \`ERROR_INVALID_NAME\`.
- CurseForge categories were swapped.
- Vanilla instances can install CurseForge resource packs, shaders and datapacks; informational tags were being read as loaders.`,
	},
	{
		version: '0.13.24',
		date: '2026-05-26T00:00:00+00:00',
		body: `### Changed
- Synced with upstream Modrinth (0.13.22 → 0.13.24): default Minecraft memory raised from 2 GB to 4 GB, an improved Java installation UI, an "Enabled" sort button in content lists, compact log handling to survive log spam, and locale updates.

### Fixed
- Non-string values when decorating download URLs.
- Pages failed to load when an auth cookie was invalid.`,
	},
	{
		version: '0.13.21',
		date: '2026-05-22T00:00:00+00:00',
		body: `### Added
- CurseForge in Discover: search spans both catalogs, merged and de-duplicated, with a logo on each card showing where a mod is hosted.
- A catalog source toggle in the controls bar, remembered across navigation and restarts.
- CurseForge project pages reusing Modrinth's layout, including the Versions and Gallery tabs.
- Installing CurseForge mods into an existing instance or through the instance picker.
- Installing CurseForge modpacks from Discover, and importing them from a \`.zip\`.
- An optional setting recolouring the Discover accent to match the active catalog. Off by default.

### Changed
- Synced with upstream Modrinth (0.13.21): Kyros upload sessions for hosting, project analytics events and moderation queue tooling.`,
	},
	{
		version: '0.13.20',
		date: '2026-05-21T00:00:00+00:00',
		body: `### Changed
- Synced with upstream Modrinth (0.13.20): content management improvements, a new date picker, Intercom bubble positioning, macOS window occlusion checks and routing fixes.`,
	},
	{
		version: '0.13.19',
		date: '2026-05-20T00:00:00+00:00',
		body: `### Changed
- Synced with upstream Modrinth (0.13.19).

### Fixed
- The right sidebar slides in and out with a transform instead of animating the grid column, so content no longer reflows while it moves.`,
	},
	{
		version: '0.13.18',
		date: '2026-05-18T00:00:00+00:00',
		body: `### Changed
- Synced with upstream Modrinth (0.13.18).
- Discord Rich Presence runs on Noctrinth's own Discord application and artwork.

### Fixed
- Signing in with an Ely.by account and launching the game with it work reliably.`,
	},
	{
		version: '0.13.17',
		date: '2026-05-17T00:00:00+00:00',
		body: `### Added
- Ely.by as a second account provider, authenticating against its Yggdrasil server.
- Launching Minecraft with Ely.by accounts, through the authlib-injector Java agent.
- A Collections section for browsing, creating, editing and deleting Modrinth collections.
- A "Followed projects" view.
- A "Save to collection" button on project pages.
- An in-app Notifications page.
- Native desktop notifications for downloads and updates.
- A Changelog tab in settings showing both the Noctrinth and Modrinth App changelogs.
- Signed application updates delivered through GitHub Releases.

### Changed
- Rebranded to Noctrinth: new logo, generated app icons and the \`com.noctrinth.app\` bundle identifier.
- The embedded sign-in WebView was replaced with a loopback HTTP redirect for the Modrinth OAuth flow.
- Exactly one account is active across the Microsoft and Ely.by providers.
- Recoloured the interface to a purple brand scheme.
- The Feature Flags settings show readable names and descriptions instead of raw keys.
- The sidebar slides in and out when toggled.
- Extended the requested Modrinth OAuth scopes.
`,
	},
]
