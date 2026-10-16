# Noctrinth news

Articles for the news page at https://everelsu.github.io/Noctrinth/news/ and for the
News sidebar in the launcher, which reads them from `news/feed/articles.json` and shows
them next to Modrinth's. Release notes do not go here: those are the changelog
(`apps/app-frontend/src/helpers/noctrinth-changelog.ts`).

One folder per article. The folder name is the article's address
(`/news/article/<folder>/`).

```
news/
  my-article/
    article.md       English, required
    article.ru.md    Russian, optional; falls back to English
    thumbnail.svg    the card picture, 1920×1080 (also .webp, .png or .jpg)
    picture.png      anything the text shows, referenced as ![](picture.png)
```

Each `article.md` starts with:

```markdown
---
title: What the card says
summary: One sentence under the title.
date: 2026-10-15T12:00:00+09:00
---

The text, in Markdown: ## headings, - lists, **bold**, _italic_, `code`,
[links](https://example.com), > quotes and ![pictures](picture.png).
```

`article.ru.md` has the same three fields in Russian. Add `unlisted: true` to keep an
article off the site while it is being written.

Thumbnails are drawn in Modrinth's blog style, in purple: a simplified picture of the
thing over isometric blocks. `scripts/news-thumbnail.mjs` draws them; give a new article
a scene of its own there (copy `new-site`) and run:

```bash
node scripts/news-thumbnail.mjs new-site news/new-site/thumbnail.svg
```

The site rebuilds itself when anything here changes on `main`. To look at it locally:

```bash
node scripts/build-changelog-site.mjs target/site-preview
```
