---
title: Ignore and hidden paths
description: Control hidden traversal and hierarchical .gitignore behavior.
icon: EyeOff
---

## Hidden paths

Hidden components are excluded by default. Opt in with:

```bash
branchcut --glob '**/*' --hidden
```

The policy applies to hidden files and directories. Avoiding a hidden directory means its descendants are not inspected.

## Gitignore is explicit

Branchcut does not silently load `.gitignore` files. Enable hierarchical rules with:

```bash
branchcut --glob '**/*' --gitignore
```

Rules are loaded at root and as nested `.gitignore` files are encountered.

## Precedence and re-inclusion

Later rules override earlier rules. Negated rules can re-include descendants, so Branchcut traverses conservatively when an ignored directory may contain a later re-inclusion.

```text
# .gitignore
generated/
!generated/keep.rs

generated/              ignored
generated/tmp.bin       ignored
generated/keep.rs       included
```

This is an important pruning boundary: an ignored directory is pruned only when doing so cannot hide a valid re-included result.

## Compatibility boundary

The implementation covers comments, ordered rules, directory rules, nested files, and negation. It does not claim every advanced Git escaping rule or platform-specific edge case. Use the compatibility reference when exact Git parity is required.

## Combine policies

Explicit exclusions, hidden policy, and gitignore state all participate in the same traversal:

```bash
branchcut \
  --glob '**/*.{rs,toml}' \
  --exclude '**/target/**' \
  --gitignore \
  --hidden
```

`--hidden` includes hidden paths; it does not disable explicit exclusions or `.gitignore` rules.
