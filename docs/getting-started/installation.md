---
title: Installation
description: Install, upgrade, verify, or uninstall the Branchcut command.
icon: Download
---

Branchcut installs as one user-level command. Cargo is needed to compile or install it, but the resulting executable has **zero third-party runtime dependencies** and can be copied or invoked on its own.

## Platform support

| Platform | Installer | Executable | Automated installation check |
|---|---|---|---|
| Windows 10/11 | PowerShell | `branchcut.exe` | `windows-latest` |
| Linux | POSIX `sh` | `branchcut` | `ubuntu-latest` |
| macOS | POSIX `sh` | `branchcut` | `macos-latest` |

The repository's [install matrix](https://github.com/codex-mohan/branchcut/actions/workflows/install-matrix.yml) is configured to exercise the same lifecycle on all three systems:

```text
install into an isolated user root
-> run branchcut --version
-> execute a real filesystem query
-> uninstall
-> verify the executable is gone
```

The CI checkout itself uses Git and the hosted Rust toolchain; it does not add any Branchcut runtime dependency.

## Choose an installation method

| Method | Best for | Requires a checkout? | Upgrade method |
|---|---|:---:|---|
| `cargo install --git` | Most users | No | Re-run with `--force` |
| Platform installer | Users already in the repository | Yes | Re-run the installer |
| `cargo build --release` | Testing or portable/manual copies | Yes | Rebuild and replace manually |

Prebuilt release archives are not required by any of these methods. Each method compiles for the operating system and CPU on which Cargo is running.

## Prerequisites

Install a current stable Rust toolchain from [rustup.rs](https://rustup.rs/), then open a new terminal and confirm both commands are available:

```bash
rustc --version
cargo --version
```

Branchcut uses Rust edition 2024. Development and release gates have been exercised with Rust 1.96.0; the hackathon reference toolchain is Rust 1.98.0.

If `cargo` is not recognized after installing Rust, restart the terminal before changing `PATH` manually.

## Recommended: install directly from GitHub

This method does not require cloning the repository:

```bash
cargo install --locked --git https://github.com/codex-mohan/branchcut.git
```

`--locked` makes Cargo use the repository's committed lockfile. Because Branchcut's dependency table is empty, the build compiles only Branchcut itself and the Rust standard library.

Cargo writes the executable and its installation receipt beneath the active Cargo home:

| Platform | Default executable |
|---|---|
| Windows | `%USERPROFILE%\.cargo\bin\branchcut.exe` |
| Linux | `~/.cargo/bin/branchcut` |
| macOS | `~/.cargo/bin/branchcut` |

If `CARGO_HOME` is set, replace `~/.cargo` or `%USERPROFILE%\.cargo` with that value.

### Install a specific revision

For a reproducible source selection, replace `COMMIT` with a full Git commit hash:

```bash
cargo install --locked \
  --git https://github.com/codex-mohan/branchcut.git \
  --rev COMMIT
```

This pins the source revision. It does not claim the optional byte-for-byte reproducible-build bonus; Windows linker metadata may still differ between builds.

## Install from a checkout

Clone or download the repository, enter its root, and use the native installer. Both installers:

1. confirm that Cargo is available;
2. resolve the installation root to an absolute path;
3. run `cargo install --locked --force` against the checkout;
4. verify that the expected executable exists;
5. run `branchcut --version` through that exact executable;
6. report its location and the matching uninstall command.

They do **not** silently modify `PATH`.

### Windows PowerShell

From the repository root:

```powershell
.\scripts\install.ps1
```

If local script execution is disabled, bypass the policy for this process only:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\install.ps1
```

Install beneath a custom Cargo root:

```powershell
.\scripts\install.ps1 -InstallRoot "$HOME\.branchcut"
```

The custom executable will be `$HOME\.branchcut\bin\branchcut.exe`.

### Linux

From the repository root:

```bash
sh scripts/install.sh
```

Install beneath a custom root:

```bash
BRANCHCUT_INSTALL_ROOT="$HOME/.branchcut" sh scripts/install.sh
```

### macOS

The macOS flow uses the same POSIX script and builds for the current Apple target:

```bash
sh scripts/install.sh
```

Install beneath a custom root:

```bash
BRANCHCUT_INSTALL_ROOT="$HOME/.branchcut" sh scripts/install.sh
```

The Unix script deliberately runs through `sh`; executable permission on the script file is not required.

## PATH setup

The default Cargo binary directory is normally added to `PATH` by rustup. The installers verify the binary by absolute path, so installation can succeed even when your current shell cannot yet resolve the short `branchcut` command.

### Inspect the installed command

Windows PowerShell:

```powershell
Get-Command branchcut -All
```

Linux and macOS:

```bash
command -v branchcut
```

### Add a custom root for the current terminal

Windows PowerShell:

```powershell
$env:Path = "$HOME\.branchcut\bin;$env:Path"
```

Linux or macOS:

```bash
export PATH="$HOME/.branchcut/bin:$PATH"
```

To keep that Unix setting, place the export in the startup file for the shell you actually use, such as `~/.bashrc` for Bash or `~/.zshrc` for Zsh. On Windows, add the custom `bin` directory to the user-level `Path` through **System Properties → Environment Variables**, then open a new terminal.

## Verify the installation

Run these commands outside the repository so a build artifact cannot be mistaken for the installed command:

```bash
branchcut --version
branchcut --help
branchcut --cwd /path/to/project --glob '**/*.rs' --first
```

PowerShell example:

```powershell
branchcut --cwd C:\path\to\project --glob "**/*.rs" --first
```

The version command should print `branchcut 0.1.0`. The final command should print at most one matching Rust path and exit successfully.

## Upgrade

Upgrade a direct GitHub installation to the current repository revision:

```bash
cargo install --locked --force \
  --git https://github.com/codex-mohan/branchcut.git
```

For an installation made from a checkout, update the checkout and re-run the matching installer. The scripts already pass `--force`.

An upgrade replaces the executable and updates Cargo's receipt. It does not remove the repository checkout or alter shell configuration.

## Uninstall

### Default Cargo installation

```bash
cargo uninstall branchcut
```

### Windows checkout uninstaller

```powershell
.\scripts\uninstall.ps1
```

For a custom root:

```powershell
.\scripts\uninstall.ps1 -InstallRoot "$HOME\.branchcut"
```

### Linux and macOS checkout uninstaller

```bash
sh scripts/uninstall.sh
```

For a custom root:

```bash
BRANCHCUT_INSTALL_ROOT="$HOME/.branchcut" sh scripts/uninstall.sh
```

Pass the **same root** used during installation. The uninstallers remove Branchcut through Cargo and verify that its executable is gone. If Cargo itself is no longer available, they can remove the exact expected Branchcut binary as a fallback.

Uninstalling Branchcut does not remove:

- Rust or Cargo;
- another package in the Cargo root;
- your source checkout;
- shell startup files or `PATH` entries;
- files that Branchcut previously queried.

An empty custom root directory may remain. Remove that directory yourself only after confirming that it contains nothing else you need.

## Build without installing

From the repository root:

```bash
cargo build --release
```

The release profile enables optimization level 3, fat link-time optimization, one code-generation unit, symbol stripping, and abort-on-panic.

| Platform | Build output |
|---|---|
| Windows | `target/release/branchcut.exe` |
| Linux and macOS | `target/release/branchcut` |

You may copy that single executable to a directory already on `PATH`. A manually copied executable is not recorded by Cargo, so `cargo uninstall branchcut` will not know about that copy; remove it from the location to which you copied it.

## Troubleshooting

### `cargo` is not recognized

Install Rust through rustup, close every terminal using the old environment, and open a new one. Confirm `cargo --version` before retrying Branchcut installation.

### Cargo succeeds but `branchcut` is not found

Read the exact install path printed by Cargo or the platform installer. Add its containing `bin` directory to `PATH`, then start a new terminal. If `CARGO_HOME` is set, use `$CARGO_HOME/bin`, not the default location.

### The wrong Branchcut executable runs

Look for multiple copies:

```powershell
Get-Command branchcut -All
```

```bash
type -a branchcut
```

Remove or reorder older entries in `PATH`, then verify `branchcut --version` again.

### PowerShell blocks the installer

Use the process-scoped bypass command shown in the Windows section, or use the direct `cargo install --git` method. The installer does not require administrator privileges.

### Permission denied during installation

Use the default user Cargo root or a custom directory owned by your user account. Administrator or `sudo` installation is neither required nor recommended for the documented flow.

### GitHub access is restricted

Download or clone the repository through an approved route, then use the checkout installer or `cargo install --locked --path .`. Branchcut itself does not require network access after installation.

### Uninstall says the package is not installed

The executable may have been copied manually or installed under a different Cargo root. Locate it using `Get-Command branchcut -All` or `type -a branchcut`, then use the uninstaller with the original custom root or remove the known manual copy.

## Verify the zero-crate build

From a checkout:

```bash
cargo metadata --no-deps --format-version 1
```

The root package should report `"dependencies":[]`. The Fumadocs application under `website/` is an independently deployed documentation project; it is not part of the Rust binary and does not alter `Cargo.toml`.

## Development build

For quick local iteration:

```bash
cargo run -- --glob 'src/**/*.rs'
```

Use the release build for performance measurements. Debug builds intentionally prioritize compilation speed and diagnostics over traversal throughput.
