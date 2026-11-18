# Branchcut documentation site

This directory contains the Fumadocs/Next.js presentation layer for Branchcut's documentation. The canonical documentation is maintained in [`../docs`](../docs/README.md); do not author pages in `.content/`.

## Local development

Requirements:

- Node.js 22 or newer
- npm

Run:

```bash
npm install
npm run dev
```

Open <http://localhost:3000>. The `predev` script copies the canonical Markdown and MDX files from `../docs` into the generated `.content/docs` directory before Fumadocs compiles them.

After changing documentation while the development server is already running, refresh the generated content with:

```bash
npm run sync:docs
```

## Production verification

```bash
npm run lint
npm run types:check
npm run build
```

`prebuild` runs the same documentation sync, so the production build always consumes the root documentation tree.

## Deploy to GitHub Pages

The site is a static Next.js export. The [`pages.yml`](../.github/workflows/pages.yml)
workflow builds it on every relevant push to `master` and deploys
`website/out` through GitHub Pages.

For this repository's project URL, set:

```text
NEXT_PUBLIC_BASE_PATH=/branchcut
NEXT_PUBLIC_SITE_URL=https://codex-mohan.github.io/branchcut
```

The workflow already supplies both values. In the repository settings, choose
**GitHub Actions** as the Pages source. A custom domain deployment should clear
`NEXT_PUBLIC_BASE_PATH` and set `NEXT_PUBLIC_SITE_URL` to that origin.

To reproduce the deployment build locally:

```powershell
$env:NEXT_PUBLIC_BASE_PATH = "/branchcut"
$env:NEXT_PUBLIC_SITE_URL = "https://codex-mohan.github.io/branchcut"
npm run build
```

The static output is written to `website/out`.

## Deploy to Vercel

Import the repository into Vercel and use these settings:

| Setting | Value |
|---|---|
| Root Directory | `website` |
| Framework Preset | Next.js |
| Node.js | 22 or newer |
| Install Command | `npm install` |
| Build Command | `npm run build` |

Set `NEXT_PUBLIC_SITE_URL` to the production origin, for example `https://branchcut.example.com`. This produces correct canonical metadata and social-card URLs. Leave `NEXT_PUBLIC_BASE_PATH` unset for a root-domain deployment.

The included `vercel.json` supplies the framework, install, and build defaults. Vercel still needs the project Root Directory set to `website` so it finds this application.

## Content architecture

```text
repository/
├── docs/                 canonical Markdown and MDX
├── README.md             project overview and docs entry point
└── website/
    ├── scripts/sync-docs.mjs
    ├── .content/docs/    generated; never edit
    └── src/              Fumadocs UI and routes
```

The site also exposes static Fumadocs search, per-page Open Graph images, and
`llms.txt`/`llms-full.txt` routes. Visual styling is intentionally
Branchcut-specific: dark-first forest surfaces, signal green and chartreuse,
orange pruning signals, square graph-paper geometry, and the horizontal
Branchcut icon. Every visible surface uses a 0px border radius.
