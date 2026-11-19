---
title: Release process
description: How version tags produce verified Branchcut release archives.
icon: PackageCheck
---

Branchcut's release workflow builds executables after an owner pushes a version tag. It checks the tag against the package version, runs formatting, tests, and Clippy on each build platform, then publishes four archives with SHA-256 checksums. No release assets are available until a version tag has been created and the workflow succeeds.

## Supported release targets

| Target | GitHub-hosted runner | Archive |
|---|---|---|
| Windows x64 | `windows-2025` | `branchcut-vVERSION-x86_64-pc-windows-msvc.zip` |
| Linux x64 (GNU) | `ubuntu-24.04` | `branchcut-vVERSION-x86_64-unknown-linux-gnu.tar.gz` |
| macOS Intel | `macos-15-intel` | `branchcut-vVERSION-x86_64-apple-darwin.tar.gz` |
| macOS Apple Silicon | `macos-15-intel` (cross-build) | `branchcut-vVERSION-aarch64-apple-darwin.tar.gz` |

Each archive contains the executable, `LICENSE`, and English and translated README files. The release also includes `SHA256SUMS`. Windows, Linux and Intel macOS builds run natively. The Apple Silicon binary is cross-built on Intel macOS using Apple's SDK, compile-checked for its target, architecture-checked with `lipo`, and ad-hoc signed. Native ARM execution is not claimed. Rust documents this toolchain in its [Apple target reference](https://doc.rust-lang.org/rustc/platform-support/apple-darwin.html). Runner labels follow [GitHub's hosted-runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).

## Release a version

Update the version in `Cargo.toml`, ensure the release commit passes the repository gates, and push a tag whose version exactly matches the package version. For example, if the package version is `0.2.1`:

```sh
git tag v0.2.1
git push origin v0.2.1
```

The workflow only runs for pushed `v*` tags. It rejects tags that do not match `Cargo.toml`, validates the native toolchain or the explicitly supported Intel-to-ARM macOS cross-build, and waits for all platform jobs to pass before publishing. The build matrix runs `cargo fmt -- --check`, `cargo test --locked`, and `cargo clippy --locked --all-targets -- -D warnings` before each release build; the ARM job also checks target-specific source and tests. The GitHub release is created with the four archives, checksums, and prepared release notes only after every build succeeds.

Do not move or reuse a version tag. A transient runner or environment failure can be retried against the same tagged revision. Source or workflow corrections require a new version and tag; rerunning an old tag does not include later source changes.

## Verify and install an archive

Download the archive for your operating system and CPU architecture together with `SHA256SUMS` from the same GitHub release.

On Linux, verify the downloaded archives with:

```sh
sha256sum --check --ignore-missing SHA256SUMS
```

On macOS:

```sh
shasum -a 256 branchcut-v0.2.1-aarch64-apple-darwin.tar.gz
```

On macOS, compare that output with the matching filename's line in `SHA256SUMS`; replace the filename above for an Intel archive. On Windows PowerShell, compare the archive hash with its matching line:

```powershell
(Get-FileHash .\branchcut-v0.2.1-x86_64-pc-windows-msvc.zip -Algorithm SHA256).Hash.ToLowerInvariant()
Get-Content .\SHA256SUMS
```

Extract the archive and place `branchcut` (or `branchcut.exe`) in a directory on your `PATH`. The archive does not install or modify `PATH` automatically. For a source-built installation, use the [installation guide](../getting-started/installation).

## Release checklist

- Update `Cargo.toml` and `Cargo.lock` together when the package version changes.
- Run the format, test, and Clippy gates locally.
- Review `RELEASE_NOTES.md` before tagging and verify the published notes afterward.
- Download an archive and its checksum file; verify the checksum and run `branchcut --version`.
- Confirm the four target assets and `SHA256SUMS` appear in the release.
