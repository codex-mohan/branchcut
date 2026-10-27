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

The core library and ordinary CLI must not depend on third-party crates. Confirm the production dependency graph with `cargo tree -e normal` and inspect `Cargo.toml`. Optional integrations and development tools must be clearly separated from the core.

The one-source-file requirement belongs to the completed competition artifact. New Rust modules are expected in current development.

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

Run focused tests and the supported-semantics differential harness for query behavior changes. Check sequential and parallel results where applicable. For path-sensitive changes, run platform-specific tests on each supported operating system through CI.

## Documentation site gates

From `website/`:

```bash
npm run lint
npm run types:check
npm run build
```

The build automatically synchronizes canonical files from root `docs/` into generated Fumadocs content. Run these site checks when editing documentation or website sources.
