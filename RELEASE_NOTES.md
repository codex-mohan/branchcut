# Branchcut v0.2.0

The first post-competition release turns Branchcut into a modular filesystem query engine with a reusable Rust library and a thin CLI. The core and ordinary CLI remain free of third-party crates.

## Highlights

- Hierarchical and inherited Git ignores, repository/global excludes, linked worktrees, `.ignore`, explicit override files and rule provenance.
- Richer braces, composed component-local extglobs, POSIX ASCII classes, opt-in pattern escaping and Unicode scalar matching.
- Size/time/depth filters, entry-type unions, empty/executable selection and Unix ownership filters.
- Symlink following with cycle checks, multi-root selection, handle budgets, post-order traversal and cooperative cancellation.
- Streaming and exact limits, metadata JSON, formatting, command previews, bounded execution and shell completions.
- Public matcher, metadata-record visitor and multi-root APIs, plus current feature and compatibility documentation.

## Downloads

Choose the archive for Windows x64, Linux x64, macOS Intel or macOS Apple Silicon. Every archive contains the executable, license and README files in English, Spanish, French, Simplified Chinese, Japanese and Hindi. Verify the downloaded archive against `SHA256SUMS`, extract it, and put the executable on your `PATH`. Running a prebuilt binary does not require Cargo.

Windows, Linux and Intel macOS builds run their native regression suites. The Apple Silicon archive is cross-built on Intel macOS using Apple's SDK; its target source/tests are compile-checked and its Mach-O architecture is verified. Native Apple Silicon execution has not been tested for this release.

Performance records identify their measured source and retain defaults, tuned configurations, memory samples and losing results. They do not establish universal superiority or full compatibility with other tools.

The historical `submission-v1` tag is preserved unchanged. This release does not add content search, indexing or a service protocol.
