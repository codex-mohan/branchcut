---
title: Verification
description: Build, test, lint, and inspect the zero-dependency constraints.
icon: ListChecks
---

## Rust gates

Run before submitting implementation changes:

```bash
cargo fmt -- --check
cargo test
cargo clippy -- -D warnings
```

## Constraint proof

```bash
cargo metadata --no-deps --format-version 1
```

Confirm that `Cargo.toml` has an empty dependency table and that `src/main.rs` is the only Rust implementation file.

## Real queries

```bash
branchcut --glob 'src/**/*.rs' --type file
branchcut --glob 'src/**/*.{rs,toml}' --exclude '**/target/**' --sort
branchcut --glob '**/*.rs' --first
branchcut --glob '**/*.rs' --count --stats
branchcut --glob 'packages/**/src/**/*.rs' --exclude '**/node_modules/**' --explain
branchcut --glob '**/*' --gitignore --sort
branchcut --glob '**/*.rs' --json
branchcut --glob '**/*.rs' --threads 4 --count --stats
```

## Documentation site gates

From `website/`:

```bash
npm run lint
npm run types:check
npm run build
```

The build automatically synchronizes canonical files from root `docs/` into generated Fumadocs content.
