---
title: Ignore and hidden paths
description: Control hidden traversal and layered Git and .ignore rules.
icon: EyeOff
---

## Hidden paths

Hidden components are excluded by default. Opt in with:

```bash
branchcut --glob '**/*' --hidden
```

The policy applies to hidden files and directories. Avoiding a hidden directory means its descendants are not inspected.

## Ignore files are explicit

Branchcut does not silently load ignore files. Choose a source set explicitly:

```bash
# .gitignore plus repository and global Git excludes
branchcut --glob '**/*' --gitignore

# Git sources plus nested .ignore files
branchcut --glob '**/*' --ignore

# Add one or more explicit Git-style rule files
branchcut --glob '**/*' --ignore-file .branchcut-ignore
```

`--gitignore` reads applicable Git ignore configuration, global excludes, repository `info/exclude`, and root/nested `.gitignore` files. `--ignore` enables those sources and root/nested `.ignore` files. `--ignore-file` adds named rule files even without discovery; each file is applied at highest priority, in command-line order. The precedence is Git sources, then `.ignore`, then explicit `--ignore-file` rules. Within a source level, later matching rules override earlier ones.

Git config include files are read where their paths can be resolved. `includeIf gitdir:`, `includeIf gitdir/i:`, and `includeIf onbranch:` are supported. `includeIf hasconfig:` is unsupported and reported as an ignore-source error. Branchcut honors `GIT_CONFIG_GLOBAL` and `GIT_CONFIG_NOSYSTEM`, and reads `GIT_CONFIG_SYSTEM` only when that path is explicitly set; it does not guess an implicit system config path on Windows. It does not read Git's command-line `-c` settings or `GIT_CONFIG_COUNT` overrides.

Rules are loaded at the query root and as nested files are encountered. Parent rules remain active while walking into a child directory.

A slashless directory rule such as `target/` applies at every depth below its ignore file; `/target/` applies only at that file's root. Directory-only rules leave regular files and symlinks named `target` eligible. Rules remain case-sensitive regardless of the query's case mode.

## Precedence and re-inclusion

Later rules override earlier rules for the same path. Git does not visit excluded directories, so a negated child rule cannot restore a path while its parent remains ignored. Explicitly re-include the parent directory before re-including selected children.

```text
# .gitignore
generated/*
!generated/keep.rs

generated/              traversed
generated/tmp.bin       ignored
generated/keep.rs       included
```

To ignore a directory's contents and restore selected children, leave the parent traversable with `generated/*`. To ignore the directory itself, use `generated/`; a later `!generated/keep.rs` cannot restore a descendant while that parent stays excluded. `!generated/` can reopen the directory, after which later child rules can select its contents.

## Compatibility boundary

The tested rule syntax covers comments, ordered rules, anchoring, directory-only rules, nested files, negation, escaped leading markers and spaces, and excluded-parent behavior. Git has additional escaping and configuration edge cases; unsupported configuration conditions are reported instead of silently accepted. See the compatibility reference for the remaining boundaries.

## Combine policies

Explicit glob exclusions, hidden policy, and enabled ignore sources all participate in the same traversal:

```bash
branchcut \
  --glob '**/*.{rs,toml}' \
  --exclude '**/target/**' \
  --ignore \
  --hidden
```

`--hidden` includes hidden paths; it does not disable explicit exclusions or `.gitignore` rules.
