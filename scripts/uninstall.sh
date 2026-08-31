#!/bin/sh
set -eu

install_root=${BRANCHCUT_INSTALL_ROOT:-${CARGO_HOME:-"$HOME/.cargo"}}
binary="$install_root/bin/branchcut"

if [ -e "$binary" ]; then
    was_installed=1
else
    was_installed=0
fi

if command -v cargo >/dev/null 2>&1; then
    if ! cargo uninstall branchcut --root "$install_root" && [ "$was_installed" -eq 1 ]; then
        printf '%s\n' 'error: cargo uninstall failed' >&2
        exit 1
    fi
elif [ "$was_installed" -eq 1 ]; then
    rm -f -- "$binary"
fi

if [ -e "$binary" ]; then
    printf 'error: Branchcut is still present at %s\n' "$binary" >&2
    exit 1
fi

if [ "$was_installed" -eq 1 ]; then
    printf 'Uninstalled Branchcut from %s\n' "$install_root"
else
    printf 'Branchcut was not installed in %s\n' "$install_root"
fi
