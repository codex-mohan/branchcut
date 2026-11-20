---
title: Compatibility
description: Supported behavior and honest gaps relative to mature glob tools.
icon: BadgeCheck
---

Branchcut intentionally implements a focused filesystem-query surface. It is not a drop-in replacement for the full `fast-glob` API.

## Supported

- `*`, `?`, component-level `**`;
- byte classes, ranges, and negated classes;
- nested/multiple brace groups and bounded stepped integer/ASCII-letter ranges;
- component-local positive extglobs and whole-component negation;
- shared multiple positive patterns;
- explicit subtree exclusions;
- file, directory, and symlink result types;
- repeatable extension filters;
- exact decimal size predicates and modification-time predicates;
- byte matching by default, with opt-in Unicode scalar matching and simple lowercase case matching;
- literal filename or cwd-relative path search;
- minimum and maximum depth from the query root;
- hidden-path control and layered Git, `.ignore`, and explicit ignore-file sources;
- sequential streaming and early stop;
- optional global sorting, counting, JSON Lines, and command execution;
- native NUL-delimited output, absolute path output, and bounded batch command execution;
- bounded parallel traversal;
- symlink following with cycle detection, Unix one-filesystem traversal, contents-first directory output, and bounded open-directory handling;
- explicit named saved-query profiles;
- planner explanations, candidate-path explanations with ignore provenance, and real traversal statistics.

## Deliberate gaps

- prefix/suffix negated extglobs and every shell-specific repetition edge case;
- captures and watch mode;
- full Unicode case folding or normalization (the opt-in Unicode mode uses simple lowercase conversion);
- every advanced Git ignore escaping edge case;
- grapheme matching; `--unicode` counts Unicode scalar values for wildcard matching, while default matching is byte-oriented;
- full API and option parity with `fast-glob`, tinyglobby, or zlob.

## Paths, sorting, and commands

- Unix JSON Lines adds `path_bytes` (lowercase hexadecimal) when the complete relative path is not UTF-8. Windows JSON Lines adds `path_utf16` (UTF-16 code units) when a path cannot be represented as UTF-8. The ordinary `path` field is for display in those cases.
- Unix plain output writes native bytes followed by a newline; filenames containing newlines are ambiguous in this format.
- `--sort --limit N` retains at most N paths and returns the first N from global native-path order; it must finish traversal before output. Unlimited sorting collects all matches. Count-only queries can stop after reaching the count limit.
- `--exec` substitutes native path values and runs with the query root as its working directory. Sorted execution processes the final sorted, limited result set.
- `--size` applies only when selecting regular files. Multiple size expressions are conjunctive. Modification-time checks use each entry's own metadata; directory mtimes do not prune children.
- Decimal size fractions are parsed without floats. Values such as `1.5MiB` are accepted only if conversion gives an integer byte threshold; sub-byte thresholds such as `0.1B` are errors.
- `--modified-within` uses an inclusive `mtime >= now - age` comparison. `--modified-before` uses strict `mtime < cutoff`; an age cutoff is resolved once at the start of each execution.
- Absolute `--modified-before` accepts UTC dates, RFC3339 `Z` or numeric offsets with up to nine fractional digits, and signed Unix timestamps such as `@-0.5`. Calendar date forms are limited to 1970–9999; representable Unix timestamps can be before the epoch.
- `--full-path` and `--filename` change only literal search-term matching. Glob patterns continue to match path components.
- Depth is measured from `--cwd` (root depth 0), even when a literal prefix narrows the physical traversal root. A maximum depth stops before opening deeper child directories.
- `--why PATH` interprets relative candidates from `--cwd` and accepts an absolute candidate only when it is textually below the query root. It reads the candidate and relevant ignore files without traversing unrelated directories. A correctly excluded candidate exits successfully.
- Saved profiles are independent and do not inherit `default`. Their config directory is the default query root. Lists from a profile and the command line append; command-line scalar options override profile values. Output and execution settings are command-line only.
- Default matching is byte-oriented with ASCII-only case-insensitive matching. `--unicode` uses Unicode scalar matching for valid UTF-8 names and simple lowercase conversion; it does not normalize text or apply full case-fold expansions. Invalid native byte names use byte matching. Smart-case inspects positive globs and literal search terms; exclusions do not switch the mode. Ignore rules remain case-sensitive.
- `--gitignore` loads Git ignore sources, including repository/global excludes and hierarchical `.gitignore`; `--ignore` additionally loads hierarchical `.ignore`; repeatable `--ignore-file` sources override both. Precedence is Git sources < `.ignore` < explicit files. `GIT_CONFIG_GLOBAL` and `GIT_CONFIG_NOSYSTEM` are honored; `GIT_CONFIG_SYSTEM` is read only when explicitly set. Git `-c` and `GIT_CONFIG_COUNT` overrides are not read. Unsupported `includeIf hasconfig:` conditions are reported. An implicit Windows system config path is not guessed.
- `--one-file-system` is Unix-only. Following directory symlinks and contents-first output use sequential traversal. At the `--max-open` limit, the walker buffers an ancestor's pending entries and closes its handle. It later resumes from those buffered entries.
- The sequential walker uses explicit directory frames, tested at 600 components. Operating-system handle/path limits still apply; parallel queue-overflow fallback remains recursive.

See [feature status and replacements](./feature-status.md) for the full implemented surface and remaining gaps. The supported ignore subset includes excluded-parent semantics, checked against Git; advanced Git configuration and escaping behavior remains outside the compatibility claim.

## Comparison principle

A feature is claimed compatible only where documented tests compare normalized result sets:

```text
Branchcut − oracle = {}
oracle − Branchcut = {}
```

Ordering is not compared unless deterministic sorting is explicitly selected.
