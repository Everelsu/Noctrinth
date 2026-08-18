# Changelog screenshots

Images referenced from the entries next to this folder. Write the reference
against the root of the changelog site, not against this path:

```markdown
![Modern Java](/changelog/modern-java.png)
```

`scripts/build-changelog-site.mjs` publishes this folder as `/changelog/` on
the site, and the app resolves the path against it — so a screenshot is
downloaded once by whoever scrolls to it, instead of being carried inside every
installer. Nothing here is bundled into the app: these files sit in `src/` but
no module imports them.
