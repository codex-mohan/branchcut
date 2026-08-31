---
title: Documentation
description: Author canonical Markdown and preview it through Fumadocs.
icon: FilePenLine
---

## Source of truth

Author documentation under root `docs/`. Do not edit generated files in `website/.content/`.

```text
docs/*.md, *.mdx, meta.json
             │
             │ npm run sync:docs
             ▼
website/.content/docs/       generated, disposable
             │
             ▼
Fumadocs routes + search + llms.txt
```

## Add a page

1. Create a Markdown or MDX file with `title` and `description` frontmatter.
2. Add it to the nearest `meta.json` so navigation order is intentional.
3. Add it to `docs/README.md` when it belongs in the human-readable map.
4. Use relative Markdown links for repository browsing.
5. Run the website build to validate routes and MDX.

## ASCII diagrams

Use fenced `text` blocks. Keep lines short enough for mobile horizontal scrolling and explain the conclusion in prose.

```text
candidate directory
        │
        ├── any positive descendant possible? ── no ──╳ prune
        └── yes
             └── safely excluded subtree? ───── yes ──╳ prune
```

## Claims

Link performance claims to the Evidence section. Keep compatibility wording aligned with the executable and avoid universal superiority language.
