# Changelog screenshots

Drop images for changelog entries here and reference them from the root:

```markdown
![Modern Java](/changelog/modern-java.png)
```

Anything in this folder ships as-is, so the path written in the markdown is the
path the app loads — nothing bundles or rewrites it. Only paths under
`/changelog/` are allowed in an entry, so an image cannot point outward.

The entries themselves are in `src/changelog/<version>.md`.
