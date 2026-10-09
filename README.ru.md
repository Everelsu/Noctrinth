<img src=".github/assets/noctrinth-banner.svg" alt="Noctrinth" width="100%"/>

<div align="center">

[![Release](https://img.shields.io/github/v/release/Everelsu/Noctrinth?include_prereleases&style=for-the-badge&logo=github&logoColor=white&label=Release&labelColor=16181c&color=ac51fb)](https://github.com/Everelsu/Noctrinth/releases)
[![Downloads](https://img.shields.io/github/downloads/Everelsu/Noctrinth/total?style=for-the-badge&logo=github&logoColor=white&label=Downloads&labelColor=16181c&color=ac51fb)](https://github.com/Everelsu/Noctrinth/releases)
[![License](https://img.shields.io/badge/License-GPL--3.0-ac51fb?style=for-the-badge&logo=gnu&logoColor=white&labelColor=16181c)](apps/app/LICENSE)
[![Stars](https://img.shields.io/github/stars/Everelsu/Noctrinth?style=for-the-badge&logo=github&logoColor=white&label=Stars&labelColor=16181c&color=ac51fb)](https://github.com/Everelsu/Noctrinth/stargazers)

[English](README.md) · **Русский**

**Ely.by, офлайн-аккаунты и паки с CurseForge — в том же Modrinth App, к которому ты привык.**

🌙 Форк Modrinth App, который понимает Ely.by, ставит паки с CurseForge и возвращает скины всем игрокам на офлайн-серверах 🚀

<a href="https://github.com/Everelsu/Noctrinth/releases/latest"><img src=".github/assets/readme/download-windows.ru.svg" alt="Скачать для Windows" height="56"/></a>
<a href="https://github.com/Everelsu/Noctrinth/releases/latest"><img src=".github/assets/readme/download-macos.ru.svg" alt="Скачать для macOS" height="56"/></a>
<a href="https://github.com/Everelsu/Noctrinth/releases/latest"><img src=".github/assets/readme/download-linux.ru.svg" alt="Скачать для Linux" height="56"/></a>

[Список изменений](https://everelsu.github.io/Noctrinth/) · [Релизы](https://github.com/Everelsu/Noctrinth/releases) · [Issues](https://github.com/Everelsu/Noctrinth/issues) · [Обсуждения](https://github.com/Everelsu/Noctrinth/discussions) · [Оригинал](https://github.com/modrinth/code)

</div>

---

## Скриншоты

<div align="center">
<table>
<tr>
<td width="62%"><img src=".github/assets/screenshots/ely-by-skins.png" alt="Сохранённые скины аккаунта Ely.by в выборе скина" width="100%"/><br/><sub>Скины Ely.by — надеть и загрузить прямо из лаунчера</sub></td>
<td width="38%"><img src=".github/assets/screenshots/accounts.png" alt="Окно добавления аккаунта: Microsoft, Ely.by или офлайн" width="100%"/><br/><sub>Microsoft, Ely.by или просто ник</sub></td>
</tr>
</table>
</div>

## Зачем нужен Noctrinth

Modrinth App — действительно хороший лаунчер, но он умеет входить только через Microsoft, ставить только `.mrpack` с самого Modrinth и показывает рекламу. Если ты играешь через Ely.by или на офлайн-серверах, держишь полку зипов с CurseForge или сидишь за заблокированным подключением, приходится держать второй лаунчер ради этих пробелов. Noctrinth закрывает их в том же приложении и идёт за оригиналом релиз в релиз.

## Что добавляет Noctrinth

### Аккаунты и скины

- **Аккаунты Ely.by** — вход через страницу самого Ely.by, запуск через authlib-injector; скины можно смотреть, надевать, загружать и удалять в той же сетке, что и у аккаунта Microsoft
- **Офлайн-аккаунты** — просто ник, для одиночной игры и офлайн-серверов; на нём виден плащ, который у этого ника есть в OptiFine, LabyMod, MinecraftCapes или SkinMC
- **Скины для всех игроков** — офлайн-серверы не передают скины, и все ходят Стивами; лаунчер ищет каждого по нику у Ely.by и Mojang, без модов с обеих сторон
- **Две копии одной сборки** — запуск уже запущенной сборки вторым аккаунтом, у каждой копии своя консоль

<div align="center"><img src=".github/assets/screenshots/skin-preview.gif" alt="Превью скина поворачивается, ник остаётся над головой" width="280"/><br/><sub>Ник поворачивается вместе с моделью, как табличка</sub></div>

### Сборки и контент

- **Установка модпаков CurseForge из `.zip`** — в очереди, с возобновлением, с чистым откатом при ошибке и с иконкой самого пака
- **CurseForge-сборки приезжают целиком** — импорт докачивает моды, которых не хватает в папке, и подписывает каждый файл CurseForge настоящим названием, автором и иконкой
- **Миграция из Modrinth App** — баннер предлагает перенести сборки: все разом или выборочно, с опцией удалить их из источника
- **Язык поиска по библиотеке** — `@sodium` находит сборки с модом, `#shader` фильтрует по типу, `!outdated` по состоянию, а `-` переворачивает условие
- **Современная Java для 1.7.10** — один клик ставит lwjgl3ify или Cleanroom вместе с патчами лаунчера, ещё один всё возвращает
- **Видеокарта для каждой Java** на Windows — для ноутбуков, где игра запускается не на той GPU
- **Анимированные иконки** — GIF остаётся анимированным в библиотеке и переживает экспорт в `.mrpack`

<div align="center"><img src=".github/assets/screenshots/library-search.gif" alt="Ввод @sodium в поиск библиотеки оставляет только сборки с Sodium" width="100%"/><br/><sub><code>@sodium</code> оставляет в библиотеке только сборки с ним</sub></div>

### Вокруг приложения

- **Перевод описаний** — Google Translate или DeepL со своим ключом, с возвратом к оригиналу одной кнопкой
- **Пресеты акцента** — девять цветов для всего интерфейса, заставки, иконок окна и панели задач, с необязательной подкраской фона
- **Жесты тачпада** — два пальца вбок листают назад и вперёд, как в Chrome
- **Коллекции и отслеживаемые проекты**, а также **уведомления Modrinth** — прямо в приложении
- **Прокси** — один URL (`http://`, `https://`, `socks5://`, `socks5h://`) для всех запросов лаунчера, для регионов, где Modrinth заблокирован
- **Без рекламы** — реклама, окно согласия и допродажи Modrinth отключены

<div align="center">
<table>
<tr>
<td width="50%"><img src=".github/assets/screenshots/translate.gif" alt="Описание китайского модпака перетекает в английский и обратно" width="100%"/><br/><sub>Китайское описание — по-английски и обратно</sub></td>
<td width="50%"><img src=".github/assets/screenshots/accents.gif" alt="Переключение девяти пресетов акцента перекрашивает окно настроек" width="100%"/><br/><sub>Девять акцентов, применяются сразу по клику</sub></td>
</tr>
</table>
</div>

## Начало работы

### Установка

Возьми установщик для своей платформы из [последнего релиза](https://github.com/Everelsu/Noctrinth/releases/latest):

| Платформа | Примечание                            |
| --------- | ------------------------------------- |
| Windows   | Установщик (NSIS)                     |
| macOS     | Универсальный — Intel и Apple Silicon |
| Linux     | Собран на Ubuntu 22.04                |

Обновления подписаны и доставляются автоматически через GitHub Releases — переустанавливать не нужно.

> [!NOTE]
> Пре-релизные сборки (`0.21.10-beta.1` и подобные) **не** раздаются авто-обновлением. Ставь их вручную; при выходе соответствующего стабильного релиза приложение подхватит его как обычное обновление.

### Перенос инстансов

Уже на Modrinth App? Открой Noctrinth, и баннер предложит импортировать всё найденное. Хочешь выбрать сам? **Создать инстанс → Импорт** покажет Modrinth App рядом с Prism, MultiMC, ATLauncher, GDLauncher и CurseForge.

## Сборка из исходников

Нужны [Node.js](https://nodejs.org/) ≥ 24.15, [pnpm](https://pnpm.io/), [Rust](https://www.rust-lang.org/tools/install) и [зависимости Tauri](https://v2.tauri.app/start/prerequisites/).

```bash
pnpm install
```

Скопируй шаблон окружения в `packages/app-lib/`, затем запусти десктоп-приложение с горячей перезагрузкой:

```bash
pnpm app:dev
```

Перед тем как открыть pull request, прогони проверки фронтенда:

```bash
pnpm prepr:frontend:app
```

## Структура репозитория

Это монорепозиторий оригинального Modrinth, поэтому в нём куда больше, чем лаунчер — здесь же сайт, backend API и общие библиотеки. Noctrinth поставляет именно десктоп-приложение.

| Путь                                | Что это                                                   |
| ----------------------------------- | --------------------------------------------------------- |
| `apps/app`                          | Оболочка Tauri — Rust-команды, окно и настройки апдейтера |
| `apps/app-frontend`                 | UI лаунчера (Vue 3), список изменений и локали Noctrinth  |
| `packages/app-lib`                  | Ядро лаунчера — аккаунты, инстансы, установки, импорт     |
| `packages/ui`                       | Общая библиотека Vue-компонентов                          |
| `apps/frontend`, `apps/labrinth`, … | Сайт и backend Modrinth, перенесены из оригинала          |

Для архитектуры и инфраструктуры, не специфичной для форка, справочником остаётся [оригинальный репозиторий](https://github.com/modrinth/code).

## Отношения с оригиналом

Noctrinth синхронизируется с [modrinth/code](https://github.com/modrinth/code) и привязывает свою версию к версии оригинала один в один: если у Modrinth `0.21.9`, у Noctrinth тоже `0.21.9`. Там, где обе стороны реализуют одно и то же, побеждает вариант оригинала, а вариант форка удаляется. Так ушли общий профиль `options.txt`, своя вкладка скриншотов и свой кэш загрузок: синхронизация настроек, страница «Скриншоты» и хранилище контента из оригинала делают то же самое лучше.

Исправление между релизами оригинала выходит микропатчем (`0.21.6+1`) — установки на этой версии получают его как обычное обновление. Тестовые сборки выходят пре-релизами (`0.21.10-beta.1`), которые сортируются выше текущего стабильного релиза и ниже следующего, так что тестеры сразу переходят на настоящий релиз, как только он выходит.

## Участие в разработке

Баг-репорты и pull request'ы приветствуются — начни с [открытия issue](https://github.com/Everelsu/Noctrinth/issues).

Нашёл баг, который не специфичен для Noctrinth? Ему место [в оригинале](https://github.com/modrinth/code/issues) — исправление там получат все, и оно придёт в этот форк при следующей синхронизации.

Если Noctrinth оказался полезен, [поставь звезду](https://github.com/Everelsu/Noctrinth/stargazers).

## Лицензия

Десктоп-приложение распространяется под [GPL-3.0](apps/app/LICENSE). У остальных пакетов свои лицензии — см. файл `LICENSE` в каждом из них и [COPYING.md](COPYING.md) для подробностей.

Брендинг Modrinth принадлежит Rinth, Inc. и здесь не используется — у Noctrinth свой. Noctrinth — независимый форк, не связанный с Rinth, Inc. и не одобренный ею.
