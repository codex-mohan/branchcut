---
title: Verification
description: Build, test, lint, and inspect the zero-dependency constraints.
icon: ListChecks
---

## Rust gates

Run before submitting implementation changes:

```bash
cargo fmt -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

## Optional regex integration gates

Run the separate package gates for changes to regex matching or shared content APIs.

```sh
cargo fmt --manifest-path integrations/content-regex/Cargo.toml -- --check
cargo test --locked --manifest-path integrations/content-regex/Cargo.toml
cargo clippy --locked --manifest-path integrations/content-regex/Cargo.toml --all-targets -- -D warnings
cargo tree --locked -e normal --manifest-path integrations/content-regex/Cargo.toml
```

The optional package declares its own dependency graph and lockfile. Verify filename/path and content regex through the executable, together with literal content, encoding, context, input, and serializer regressions in the ordinary package.

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

The oracle is a development-only tool installed outside the production package:

```bash
npm install --prefix target/oracle --ignore-scripts --no-audit --no-fund fast-glob@3.3.3
cargo build --release --locked
node scripts/verify.mjs --binary target/release/branchcut --oracle-dir target/oracle
```

On Windows, use `target/release/branchcut.exe`. The runner rejects a different oracle version and compares complete result sets in both sequential and parallel modes. The CI matrix runs these gates on Windows, Linux, and macOS.

## Documentation site gates

From `website/`:

```bash
npm run lint
npm run types:check
npm run build
```

The build automatically synchronizes canonical files from root `docs/` into generated Fumadocs content. Run these site checks when editing documentation or website sources.
