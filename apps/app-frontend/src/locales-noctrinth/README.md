# Noctrinth translations

Upstream's catalogues live in `../locales/<code>/index.json` and are pulled
from Modrinth's Crowdin. Nothing of the fork's belongs in them: upstream
rewrites those files wholesale, so every fork edit inside one is a merge
conflict on the next sync. This folder is the fork's side, and it is applied
over upstream's at startup — see `../helpers/noctrinth-locales.ts`.

Each locale has up to two files, differing only in who wins:

| File            | Holds                                                                                           | Wins over upstream                        |
| --------------- | ----------------------------------------------------------------------------------------------- | ----------------------------------------- |
| `messages.json` | The fork's own strings, and upstream strings Noctrinth words differently (its own name, mostly) | yes                                       |
| `fallback.json` | Translations of _upstream's_ strings for a locale Crowdin hasn't finished                       | no — only used where upstream has nothing |

So a string only ever needs writing once: put a Noctrinth string in
`messages.json`, and a translation that stands in for a missing upstream one in
`fallback.json`. When Crowdin does translate it, upstream's wins on the next
sync and nothing here has to be cleaned up.

Both files use the same shape as upstream's, so an entry can be moved between
them by cutting and pasting:

```json
{
	"app.some.message": { "message": "Some message" }
}
```

Only locales the app already carries are extended; a folder for a locale that
is not in `LOCALES` is ignored.
