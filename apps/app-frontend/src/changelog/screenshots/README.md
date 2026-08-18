# Changelog screenshots

Images referenced from the entries next to this folder. The reference is the
published path, which mirrors this one:

```markdown
![Modern Java](/changelog/screenshots/modern-java.png)
```

`scripts/build-changelog-site.mjs` publishes this folder as
`/changelog/screenshots/` on the site, and the app resolves the path against
it — so a screenshot is downloaded once by whoever scrolls to it, instead of
being carried inside every installer. Nothing here is bundled into the app:
these files sit in `src/` but no module imports them.

A screenshot only appears once the site carries it, so an entry written for an
unreleased version shows a broken image until the Pages workflow has run on
`main`. Build the site locally to see it before then:

```bash
node scripts/build-changelog-site.mjs /tmp/site
```

The build fails if an entry points at a file that is not here, or at a path
outside `/changelog/screenshots/`.
