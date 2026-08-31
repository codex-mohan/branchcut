---
title: Installation
description: Build or install the optimized Branchcut executable.
icon: Download
---

Branchcut requires a Rust toolchain with Cargo. The project uses Rust edition 2024 and has no third-party Rust dependencies.

## Build from source

Clone or download the repository, enter its root, and build the optimized executable:

```bash
cargo build --release
```

The release profile enables optimization level 3, fat link-time optimization, one code-generation unit, symbol stripping, and abort-on-panic.

| Platform | Output |
|---|---|
| Unix | `target/release/branchcut` |
| Windows | `target/release/branchcut.exe` |

## Install from the checkout

To place Branchcut in Cargo's binary directory:

```bash
cargo install --path .
```

Confirm the executable is available:

```bash
branchcut --version
branchcut --help
```

## Verify the zero-crate build

```bash
cargo metadata --no-deps --format-version 1
```

The root package should report an empty dependency list. The Fumadocs application under `website/` is an independently deployed documentation project; it is not part of the Rust binary and does not alter `Cargo.toml`.

## Development build

For quick local iteration:

```bash
cargo run -- --glob 'src/**/*.rs'
```

Use the release build for performance measurements. Debug builds intentionally prioritize compilation speed and diagnostics over traversal throughput.
