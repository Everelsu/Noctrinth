<img src=".github/assets/noctrinth-banner.svg" alt="Noctrinth" width="100%"/>

<div align="center">

[![Release](https://img.shields.io/github/v/release/Everelsu/Noctrinth?include_prereleases&style=for-the-badge&logo=github&logoColor=white&label=Release&labelColor=16181c&color=ac51fb)](https://github.com/Everelsu/Noctrinth/releases)
[![Downloads](https://img.shields.io/github/downloads/Everelsu/Noctrinth/total?style=for-the-badge&logo=github&logoColor=white&label=Downloads&labelColor=16181c&color=ac51fb)](https://github.com/Everelsu/Noctrinth/releases)
[![License](https://img.shields.io/badge/License-GPL--3.0-ac51fb?style=for-the-badge&logo=gnu&logoColor=white&labelColor=16181c)](apps/app/LICENSE)
[![Stars](https://img.shields.io/github/stars/Everelsu/Noctrinth?style=for-the-badge&logo=github&logoColor=white&label=Stars&labelColor=16181c&color=ac51fb)](https://github.com/Everelsu/Noctrinth/stargazers)

[English](README.md) · **Русский**

**Меняешь лаунчер? Твои инстансы переезжают вместе с тобой — в один клик, ничего не потеряется.**

🌙 Форк Modrinth App с поддержкой Ely.by, установкой CurseForge-паков и импортом инстансов из шести лаунчеров 🚀

[Список изменений](https://everelsu.github.io/Noctrinth/) · [Релизы](https://github.com/Everelsu/Noctrinth/releases) · [Issues](https://github.com/Everelsu/Noctrinth/issues) · [Обсуждения](https://github.com/Everelsu/Noctrinth/discussions) · [Оригинал](https://github.com/modrinth/code)

</div>

---

## Скриншоты

<div align="center">
<table>
<tr>
<td width="50%"><img src=".github/assets/screenshots/library.png" alt="Библиотека инстансов, лента новостей и список друзей" width="100%"/><br/><sub>Библиотека — все инстансы, рядом лента активности</sub></td>
<td width="50%"><img src=".github/assets/screenshots/ely-by-skins.png" alt="Управление скином Ely.by прямо в Noctrinth" width="100%"/><br/><sub>Скины Ely.by — без похода на сайт</sub></td>
</tr>
</table>
</div>

## Зачем нужен Noctrinth

Modrinth App — действительно хороший лаунчер, но он умеет входить только через Microsoft, ставить только `.mrpack` с самого Modrinth и показывает рекламу. Если ты играешь через Ely.by, держишь полку зипов с CurseForge или сидишь за заблокированным подключением, приходится держать второй лаунчер только ради этих пробелов.

<div align="center">

| Метрика              | Значение                              |
| -------------------- | ------------------------------------- |
| Провайдеры аккаунтов | Microsoft **+ Ely.by**                |
| Источники импорта    | **6** лаунчеров, включая Modrinth App |
| Реклама              | **отсутствует**                       |

</div>

## Что добавляет Noctrinth

- **Аккаунты Ely.by** — вход рядом с Microsoft, запуск через authlib-injector, управление скинами во встроенном окне
- **Миграция из Modrinth App** — баннер находит установленный Modrinth App и предлагает перенести инстансы: все разом или выборочно, с опцией удалить их из источника после переноса
- **Импорт модпаков CurseForge из `.zip`** — установка пака прямо с диска через тот же конвейер задач, что и всё остальное: в очереди, с возобновлением, с чистым откатом при ошибке
- **Пресеты акцента** — девять цветов для всего интерфейса, от заставки и иконки окна до полосы загрузки, с необязательной подкраской поверхностей
- **Скины для всех игроков** — на офлайн-серверах скины не передаются вовсе, и все ходят Стивами; лаунчер ищет их по нику у Ely.by и, если там не нашлось, у Mojang — без модов с обеих сторон
- **Своя папка со скинами** — положи `<ник>.png`, и игрок наденет его раньше любой скин-системы; рядом лежат `capes/` и `elytras/`
- **Две копии одной сборки** — запуск уже запущенной сборки вторым аккаунтом, у каждой копии своя консоль на вкладке «Логи»
- **Общий профиль `options.txt`** — выбери настройки Minecraft, которые дописываются в каждую сборку при запуске, и исключи те сборки, которым это не нужно
- **Коллекции и отслеживаемые проекты** — просмотр, создание и редактирование коллекций, плюс виртуальная коллекция всего, на что ты подписан
- **Уведомления** — лента уведомлений Modrinth прямо в приложении
- **Прокси** — один URL (`http://`, `https://`, `socks5://`, `socks5h://`) для всех запросов лаунчера — для регионов, где Modrinth заблокирован
- **Встроенный список изменений** — заметки о релизах Noctrinth и Modrinth рядом, в Настройки → Список изменений
- **Фиолетовый, без рекламы** — брендинг Noctrinth везде, реклама и допродажи Modrinth отключены

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
> Пре-релизные сборки (`0.19.2-beta.1` и подобные) **не** раздаются авто-обновлением. Ставь их вручную; при выходе соответствующего стабильного релиза приложение подхватит его как обычное обновление.

### Перенос инстансов

Уже на Modrinth App? Открой Noctrinth — баннер предложит импортировать всё найденное. Хочешь выбрать сам? **Создать инстанс → Импорт** покажет Modrinth App рядом с Prism, MultiMC, ATLauncher, GDLauncher и CurseForge.

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

Noctrinth синхронизируется с [modrinth/code](https://github.com/modrinth/code) и привязывает свою версию к версии оригинала один в один — если у Modrinth `0.19.1`, у Noctrinth тоже `0.19.1`. Там, где обе стороны реализуют одно и то же, побеждает вариант оригинала, а вариант форка удаляется. Код, специфичный только для форка, остаётся — но лишь там, где не конфликтует.

Быстрые патчи между релизами оригинала выходят как semver пре-релизы (`0.19.2-beta.1`), которые сортируются выше текущего стабильного релиза и ниже следующего — так тестеры сразу переходят на настоящий релиз, как только он выходит.

## Участие в разработке

Баг-репорты и pull request'ы приветствуются — начни с [открытия issue](https://github.com/Everelsu/Noctrinth/issues).

Нашёл баг, который не специфичен для Noctrinth? Ему место [в оригинале](https://github.com/modrinth/code/issues) — исправление там получат все, и оно придёт в этот форк при следующей синхронизации.

Если Noctrinth оказался полезен, [поставь звезду](https://github.com/Everelsu/Noctrinth/stargazers).

## Лицензия

Десктоп-приложение распространяется под [GPL-3.0](apps/app/LICENSE). У остальных пакетов свои лицензии — см. файл `LICENSE` в каждом из них и [COPYING.md](COPYING.md) для подробностей.

Брендинг Modrinth принадлежит Rinth, Inc. и здесь не используется — у Noctrinth свой. Noctrinth — независимый форк, не связанный с Rinth, Inc. и не одобренный ею.
