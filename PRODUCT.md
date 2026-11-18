# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

Branchcut serves command-line users who need to find files precisely, developers
who need a reusable Rust filesystem-selection engine, and power users who need
repeatable queries with exclusions, ignore rules, metadata filters, safe command
execution, and inspectable diagnostics.

## Product Purpose

Branchcut compiles a filesystem query into one traversal plan, avoids opening
directories that cannot contribute a result, and streams matching native paths.
Success means people can replace fragile shell pipelines with a query that is
easier to understand, verify, and explain.

## Positioning

Branchcut is not only a glob matcher or a faster directory walker. It combines
pattern compilation, exclusions, ignore policy, predicates, termination, output,
and diagnostics into one filesystem query plan and proves the work it avoided.

## Operating Context

People use Branchcut in terminals, scripts, build tooling, editor integrations,
and Rust applications on Windows, Linux, and macOS. Queries commonly target
source trees, generated-code directories, dependency directories, and large
monorepos. The ordinary CLI and core library use only Rust's standard library.

## Capabilities and Constraints

The project ships literal filename search, glob queries, shared positive
patterns, exclusions, hierarchical `.gitignore` support, type and extension
filters, depth, size and modification-time predicates, hidden-path policy,
smart case, exact limits, sorting, plain/count/JSON Lines/NUL output, native
command execution, explain/why diagnostics, statistics, and sequential and
bounded parallel traversal. The core remains free of third-party crates;
optional integrations may use dependencies only when a concrete need justifies
them. Content search, indexing, a GUI/TUI, watchers, and agent protocols remain
outside the agreed scope.

## Brand Commitments

The product name is Branchcut. The horizontal branch logo is the primary mark.
Its logo green is `#3ddc97`; the interface brightens that signal to `#57ef97`
on dark surfaces with chartreuse for decisive evidence and amber for pruned
branches. The product voice is direct, technical, and honest about limits. The
campaign page must use square corners with a 0px border radius throughout.

## Evidence on Hand

The repository contains a retained direct-CLI benchmark over a 100,000-file
Windows/NTFS fixture, documented methods and raw JSON, benchmark tables, query
plans, correctness tests, installer checks, architecture documentation, and the
original branch icon. The benchmark is workload-specific and must not be
presented as a universal ranking.

## Product Principles

1. Avoid filesystem work before accelerating it.
2. Treat correct, documented semantics as part of performance.
3. Keep simple filename search simple while exposing composed filters.
4. Show evidence and limitations instead of hiding them.
5. Keep the core portable, transparent, and dependency-free.

## Accessibility & Inclusion

The web experience must support keyboard navigation, visible focus, reduced
motion, readable contrast, responsive layouts, and honest text alternatives for
visual demonstrations.
