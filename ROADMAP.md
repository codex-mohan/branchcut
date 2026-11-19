# Branchcut post-competition roadmap

Branchcut is evolving from a one-file hackathon submission into a maintainable filesystem query tool. The core remains third-party-crate-free. Everyday search should stay approachable; advanced filters and query composition should be available without burdening simple use.

This roadmap is a plan, not a statement that listed features are implemented. The preserved competition submission is tagged `submission-v1`; current development must not rewrite that history. See the [baseline record](docs/evidence/post-competition-baseline.md) for the verified starting point and limitations.

The [18 November native validation matrix](https://github.com/codex-mohan/branchcut/actions/runs/44349839187) passes formatting, tests, lint, supported-semantics differential checks and install/query/uninstall on Windows, Linux and Intel macOS. Tagged release builds still validate the separate artifact targets, including macOS ARM.

## Phase 1 — Establish and verify the baseline

**Status: implemented and locally verified.** The starting revision and original submission tag are recorded. Deterministic oracle and benchmark runners and raw before/after records are checked in.

- Preserve the competition tag and record the exact starting revision.
- Capture current compiler, platform, build, test, and benchmark context.
- Distinguish existing published competition measurements from measurements future contributors can reproduce.
- Keep an explicit defect list; a passing pre-existing test suite is not proof of correctness.

**Exit gate:** a clean checkout has repeatable build and test commands, a recorded baseline, and benchmark methodology that checks result equality before timing.

## Phase 2 — Make queries and paths trustworthy

**Status: implemented; Windows, Linux and Intel macOS CI passes.** Matching, sorted limits, ancestor ignores, native paths, command roots, and parallel best-effort/strict error handling have regression coverage. Sequential traversal and parallel overflow use iterative frames; deep-tree fixtures exercise 600 levels on Windows/Linux and fit the macOS path budget.

- Add regressions and fix shared-pattern globstar state leakage.
- Make `--sort --limit` implement global sorted selection as documented.
- Preserve applicable ancestor `.gitignore` rules after traversal-root narrowing.
- Keep filesystem paths native through matching and command invocation; define lossless machine output separately from human display.
- Align `--cwd`, relative paths, and child-command working-directory behavior.
- Check deep-tree behavior, strict/best-effort filesystem errors, and sequential/parallel result parity.
- Extend continuous integration to format, lint, test, and exercise platform-specific behavior.

**Exit gate:** focused regressions and supported-semantics differential checks pass; filesystem and path behavior is documented for supported platforms.

## Phase 3 — Extract a maintainable library

**Status: implemented; Windows, Linux and Intel macOS CI passes.** `src/main.rs` is a three-line entry point. The public library, focused core modules, CLI adapter, and external-consumer tests are in place. See [library and modules](docs/concepts/library-and-modules.md).

- Create `src/lib.rs` and keep `src/main.rs` as a thin executable entry point.
- Separate the typed query and compiled plan, pattern parser/program, sequential and parallel traversal, ignore rules, filters, native path handling, statistics, errors, and CLI.
- Keep the engine independent of terminal formatting, sorting policy, and command-line parsing.
- Move and expand tests into focused unit and CLI integration suites.
- Compare representative behavior and performance before and after extraction.

Start inside one Cargo package. Keep core and CLI dependencies empty. Add workspace packages only to support a real product need.

**Exit gate:** the library and CLI preserve verified behavior; test, lint, install, and cross-platform gates pass; any material performance change is explained.

## Phase 4 — Complete everyday search

**Status: implemented; Windows, Linux and Intel macOS CI passes.** Size, modification-time, case, path-search, and depth filters are exposed through both the CLI and library. Candidate-first metadata evaluation and the existing no-metadata fast path have focused regression coverage.

Add high-value, composable file-size and modification-time predicates, explicit case behavior and smart case for simple search, depth limits, and clear filename versus full-path matching. Decide units, comparison boundaries, time interpretation, and cross-platform case behavior before implementation. Query planning must fetch metadata only when a predicate needs it.

**Exit gate:** documented recipes can find recent documents, large files, and project files within a depth boundary on supported platforms; ordinary name-only searches retain metadata avoidance.

## Phase 5 — Explain and reuse selections

**Status: implemented; Windows, Linux and Intel macOS CI passes.** `--why PATH` explains the ordered candidate checks and identifies the decisive `.gitignore` rule and source line. Named profiles in explicit config files load before command-line options, and `--explain` prints the fully resolved query.

Extend the existing plan explanation with `--why PATH` and ignore-rule provenance. Then consider small saved project queries with explicit precedence and a command that prints the fully resolved query. Keep configuration behavior visible and stable.

**Exit gate:** a user can reproduce a saved selection and understand why a candidate matched or was excluded.

## Phase 6 — Improve selected-file workflows

**Status: implemented; Windows, Linux and Intel macOS CI passes.** Native NUL output, resolved absolute paths, and bounded shell-free batch execution now cover pipeline and external-tool handoff workflows. Batch composition respects both a path-count limit and a conservative command-size budget.

Add NUL-delimited output and explicit path forms for dependable pipelines. Improve command invocation with native argument handling, well-defined working-directory semantics, safe batching, and clear failure reporting. Command preview now shows planned native arguments without launching the child.

**Exit gate:** integration recipes handle whitespace and unusual paths correctly, and report command or traversal failures without ambiguity.

## Phase 7 — Optimize measured bottlenecks

**Status: two measured passes implemented and verified on Windows.** The [48-workload Phase 7 record](docs/evidence/phase7.md) covers 10k/100k trees and retains losses. A later [52-workload Rust-level pass](docs/evidence/rust-optimizations.md) adds pathless counts, borrowed streaming paths, reusable matcher scratch, shared ignore contexts, universal-plan short-circuiting, metadata-path avoidance, and parallel count batching. Sorted limits retain at most N native paths while preserving full-scan semantics. Public-repository, batch-execution, and Linux/macOS measurements remain open.

Expand reproducible workloads before choosing optimization work. Measure shared and unrelated pattern scaling, narrow and broad queries, ignored trees, metadata filters, early limits, and parallel traversal. Track elapsed time, memory, time to first result, opened/pruned directories, inspected entries, and metadata calls. Investigate multiple traversal roots, reusable matching state, shared ignore context, count-only memory, parallel streaming, and bounded sorted selection only where measurements identify a bottleneck.

**Exit gate:** every accepted performance change has result-equivalence checks and before/after evidence across representative workloads, including regressions or losses.

## Phase 8 — Improve adoption and release quality

The owner also authorized closing filesystem-search gaps. Bounded richer braces, component-local extglobs, Git-oracle ignore corrections, explicit sequential frames, and bounded parallel streaming have been implemented after the initial Phase 7 snapshot. See [feature status](docs/reference/feature-status.md) for remaining gaps; these additions do not imply full Unicode or Git compatibility. Performance records identify their measured source manifests rather than attributing old results to later builds.

**Status: v0.2.1 published.** The [filesystem gap-closure record](docs/evidence/gap-closure.md) tracks the ignore, matcher, traversal, filter, workflow and library work. Shell completions and source installers are available. The [GitHub release](https://github.com/codex-mohan/branchcut/releases/tag/v0.2.1) provides four prebuilt archives with SHA-256 checksums. The Apple Silicon archive was cross-built on Intel macOS and passed architecture and signature checks.

Validate everyday, developer and power-user workflows with users. Package-manager publication and native Apple Silicon execution testing remain open.

**Exit gate:** a new user can install, run a useful search, interpret its result, and upgrade without needing to build the project from source.

## Scope guardrails

- Preserve the zero-third-party-crate core and the std-only ordinary CLI.
- Keep optional integration dependencies outside the core dependency graph.
- Ship one package and a library boundary before considering a multi-package workspace.
- Do not combine semantic changes with the library extraction.
- Treat global sorting as a full-result operation unless an algorithm proves the requested top-N result; document its cost.
- Keep advanced glob compatibility, watchers, a GUI, and specialized integrations demand-driven.
- Keep documentation, compatibility claims, and published benchmarks tied to tested behavior.
