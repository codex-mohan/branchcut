---
title: FAQ
description: Short answers to common product, scope, safety, and integration questions.
icon: CircleAlert
---

## What is Branchcut?

A filesystem query engine. It compiles path patterns, exclusions, ignore policy, filters, and termination into one traversal plan, then streams matching native paths.

## Is it a glob library?

It supports globs, but that is only one input. Branchcut also owns traversal, ignore policy, metadata predicates, output, diagnostics, and selected-file commands.

## Does zero crates mean zero runtime requirements?

The core engine and ordinary CLI depend only on Rust's standard library. The compiled executable has no third-party runtime library requirement. Cargo is needed to build or install it from source.

## Does Branchcut search inside files?

No. Use a content-search tool such as ripgrep. A useful split is Branchcut for choosing paths and ripgrep or another program for reading their contents.

## Why is `.gitignore` support opt-in?

Ignore behavior is explicit so queries remain understandable and profiles can choose their own policy. `--gitignore` enables Git ignore sources, including repository/global excludes and nested `.gitignore`; `--ignore` adds nested `.ignore` files. Use `--ignore-file` for explicit highest-priority rule files.

## Can scripts and coding agents call it?

Yes. Branchcut is a normal executable with streaming stdout, stderr counters and diagnostics, stable exit codes, JSON Lines, NUL-delimited paths, and shell-free command execution. No service protocol is required for local filesystem work.

## How do I preserve unusual paths?

- Unix pipelines: prefer `--print0`.
- JSON Lines: consume `path_bytes` or `path_utf16` when the readable `path` cannot preserve the native name.
- Rust consumers: use the library's native `Path` values.
- Commands: pass native path arguments through `--exec` or `--exec-batch`.

## Why does a limit default to one worker?

Without an explicit worker count, `--first` and `--limit` select sequential traversal so results stop in filesystem traversal order. An explicit `--threads N` is honored; in parallel mode the delivered first N matches are unordered, and other workers may already have inspected additional entries. Use `--sort --limit N` for the globally first N paths; this scans the full selection. Per-match `--exec` with multiple traversal workers requires `--exec-jobs`; values greater than one run commands concurrently without an ordering guarantee.

## Why does `--stats` show metadata calls for a name-only query?

The adjective “name-only” describes per-entry selection. Root preparation, ancestor ignore validation, and other setup may still inspect filesystem metadata. Statistics count explicit metadata requests, not every call hidden inside the standard library.

## Which shell syntax do command templates support?

None. `--exec` and `--exec-batch` parse an argv template and call a process directly. Shell pipes, redirects, substitutions, and expansion are not interpreted. Invoke an explicit shell or wrapper only when shell behavior is actually required.

## Why can a no-match query succeed?

No matches is a valid completed selection and exits `0`. Invalid input and enforced filesystem or child-command failures exit `2`.

## Is Windows, Linux, and macOS supported equally?

The source and CI matrix cover all three. `--one-file-system` currently requires Unix device identifiers and is therefore unavailable on Windows. Public performance evidence is still platform-specific; do not generalize a Windows result to every filesystem or operating system.

## Can I embed it in a Rust application?

Yes. The library exposes typed queries, reusable compiled plans, standalone `PathMatcher`s, multi-root queries, path and metadata-record visitors, cooperative cancellation, filesystem diagnostics, and completion reports. See [Use Branchcut from Rust](../tutorials/rust-library).

## Does it ship shell completion, a GUI, or an index?

It can print completions for bash, zsh, fish, and PowerShell with `--completions SHELL`. There is no GUI, content index, or file watcher; those are outside the current scope. The command, machine output, and library are the supported surfaces.
