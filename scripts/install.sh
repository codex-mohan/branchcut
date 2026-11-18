#!/bin/sh
set -eu

repository=${BRANCHCUT_REPOSITORY:-https://github.com/codex-mohan/branchcut.git}
branch=${BRANCHCUT_BRANCH:-master}
install_root=${BRANCHCUT_INSTALL_ROOT:-${CARGO_HOME:-"$HOME/.cargo"}}
repository_root=''

if [ -n "${0:-}" ] && [ -f "$0" ]; then
    script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" 2>/dev/null && pwd || pwd)
    if [ -f "$script_dir/../Cargo.toml" ]; then
        repository_root=$(CDPATH= cd -- "$script_dir/.." 2>/dev/null && pwd || printf '%s' '')
    fi
fi

if ! command -v cargo >/dev/null 2>&1; then
    if ! command -v curl >/dev/null 2>&1; then
        printf '%s\n' 'error: curl is required to install Rust automatically.' >&2
        exit 1
    fi

    printf '%s\n' 'Cargo was not found. Installing a minimal stable Rust toolchain with rustup...'
    rustup_installer=$(mktemp)
    curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs -o "$rustup_installer"
    sh "$rustup_installer" -y --profile minimal --default-toolchain stable
    rm -f "$rustup_installer"

    cargo_env="${CARGO_HOME:-$HOME/.cargo}/env"
    if [ -f "$cargo_env" ]; then
        . "$cargo_env"
    fi
fi

if ! command -v cargo >/dev/null 2>&1; then
    printf '%s\n' 'error: Cargo is still unavailable after rustup completed. Open a new terminal and run this installer again.' >&2
    exit 1
fi

if [ -n "$repository_root" ]; then
    printf 'Installing Branchcut from %s\n' "$repository_root"
    cargo install --locked --force --root "$install_root" --path "$repository_root"
else
    printf 'Installing Branchcut from %s (%s)\n' "$repository" "$branch"
    cargo install --locked --force --root "$install_root" --git "$repository" --branch "$branch"
fi

binary="$install_root/bin/branchcut"
if [ ! -x "$binary" ]; then
    printf 'error: Cargo completed but the Branchcut executable was not found at %s\n' "$binary" >&2
    exit 1
fi

"$binary" --version

case ":$PATH:" in
    *:"$install_root/bin":*) ;;
    *) printf 'warning: %s is not on PATH. Add it to run branchcut from any directory.\n' "$install_root/bin" >&2 ;;
esac

printf 'Installed: %s\n' "$binary"
printf 'Uninstall: cargo uninstall branchcut --root "%s"\n' "$install_root"
