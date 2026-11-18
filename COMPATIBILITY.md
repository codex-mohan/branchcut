# Compatibility

Branchcut intentionally supports a focused filesystem-query surface rather than the full `fast-glob` API.

Composed negated extglobs use component-local language complement, consistent with Bash in the retained `a!(a).rs` case: `a.rs` and `aaa.rs` match while `aa.rs` does not. fast-glob/micromatch excludes `aaa.rs` in that case. The differential runner preserves and checks this dialect difference separately from result-equivalent cases.

## Supported

| Syntax or option | Status | Notes |
|---|---|---|
| `*` | Supported | Matches bytes within one path component |
| `?` | Supported | One byte within one path component |
| `**` | Supported | Matches zero or more path components |
| `[abc]` | Supported | Byte classes |
| `[a-z]` | Supported | Inclusive byte ranges |
| `[!abc]` / `[^abc]` | Supported | Negated classes |
| Braces and ranges | Supported | Multiple/nested alternatives; inclusive stepped integer/ASCII-letter ranges; bounded expansion |
| Extglobs | Supported subset | Component-local `@`, `?`, `+`, `*`, and whole-component `!`; prefix/suffix negation rejected |
| Multiple `--glob` | Supported | Compiled into one shared program |
| `--exclude` | Supported | Trailing `/**` exclusions prune subtrees |
| `--type file` | Supported | Default type |
| `--type dir` | Supported | Directories are emitted only when explicitly requested |
| `--type symlink` | Supported | Symlinks are not followed |
| `-e` | Supported | Suffix filter follows the selected case mode |
| `-i`, `-s`, `--smart-case` | Supported | ASCII-only folding; exclusions do not influence smart-case; ignore files remain case-sensitive |
| `--filename`, `--full-path` | Supported | Applies to literal search terms; glob matching is unchanged |
| `--min-depth`, `--max-depth` | Supported | Depth starts at cwd (0); literal-prefix roots do not reset it |
| `--size EXPR` | Supported | Repeatable AND predicates; regular files only; decimal and binary units |
| `--modified-within AGE` | Supported | Inclusive lower time boundary; execution clock is captured per run |
| `--modified-before WHEN` | Supported | Strict age cutoff, UTC/RFC3339 offset timestamp with nanoseconds, or signed Unix `@seconds` |
| `--hidden` | Supported | Hidden components excluded by default |
| `--cwd` | Supported | Relative output paths |
| `--first` / `--limit` | Supported | Stops after enough discovered matches when streaming |
| `--sort` | Supported | Global native-path order; sorted limits scan all candidates and retain at most N paths |
| `--stats` | Supported | Real traversal counters, including depth-pruned directories |
| `--explain` | Supported | Planner decisions and strategy |
| `--why PATH` | Supported | Candidate decision with filter outcomes and decisive `.gitignore` source/line; no tree walk |
| `--config`, `--profile` | Supported | Dependency-free named saved queries; CLI scalar options override loaded values |
| Positional pattern | Supported | Arguments containing glob syntax are treated as glob patterns |
| Positional search | Supported | Plain text is literal filename containment; case mode is configurable |
| `--strict` | Supported | Exit code 2 on filesystem errors |
| `--count` | Supported | Emits the match count without path serialization |
| `--gitignore` | Supported subset | Root/nested rules, ordered overrides, quoting, directory-only rules, and excluded-parent semantics verified against Git |
| `--ignore` | Supported subset | Git sources plus nested `.ignore`; precedence is Git sources < `.ignore` < explicit ignore files |
| `--ignore-file FILE` | Supported subset | Explicit Git-style rule files; repeatable and highest priority |
| `--unicode` | Supported subset | Opt-in Unicode scalar matching for valid UTF-8 names, with simple lowercase matching |
| `--follow` | Supported subset | Follows directory symlinks with cycle detection; traversal uses sequential fallback |
| `--one-file-system` | Supported on Unix | Prevents descent across device boundaries |
| `--contents-first` | Supported | Emits matched directories after descendants; sequential traversal |
| `--max-open N` | Supported | Bounds open directory handles; spilled frames retain pending entries in memory and reopen as needed |
| `--json` | Supported | Streaming JSON Lines output |
| `-0`, `--print0` | Supported | Native path output followed by NUL; incompatible with JSON and count |
| `--absolute` | Supported | Resolved absolute paths for output and command substitution |
| `--exec` | Supported | Shell-free command templates; native path substitution; child cwd is the query root |
| `--exec-batch`, `--batch-size` | Supported | Sorted or traversal-order batches with native argv expansion and a fixed size budget |
| `--threads N` | Supported | Adaptive bounded parallel traversal by default; `1` forces sequential; explicit counts above one are incompatible with CLI `--limit` and `--exec` |

## Deliberate gaps

- Advanced `.gitignore` escaping and every Git edge case are not claimed.
- Prefix/suffix negated extglobs such as `!(foo).rs` are rejected. Positive repetition uses component-local language semantics, not a claim of every shell/micromatch dialect edge case.
- Brace expansion is capped at 256 results, eight expansion levels, and 1 MiB. Extended-glob components are capped at 4,096 bytes, 512 nodes, and 16 nesting levels.
- No full `fast-glob` options/API compatibility.
- No captures or watch mode.
- Without `--unicode`, matching is byte-oriented. `--unicode` opts valid UTF-8 names into Unicode scalar wildcard matching and simple lowercase case matching; it does not implement full Unicode case folding, normalization, or grapheme matching. Invalid native byte names retain byte matching. Smart-case examines positive patterns and literal search terms, not exclusions. Ignore-file rules remain case-sensitive.
- `--size` accepts exact decimal fractions when the unit conversion resolves to an integral byte threshold; sub-byte thresholds are rejected. `K`, `M`, `G`, `T` and `kB`/`KB` etc. are decimal; `KiB`, `MiB`, `GiB`, and `TiB` are binary. Size filters require regular-file selection.
- Relative durations use fixed units `ms`, `s`, `m`, `h`, `d`, and `w`; calendar months and years are unsupported. `--modified-within` includes the exact cutoff, while `--modified-before` is strict. Absolute cutoffs are UTC dates (`YYYY-MM-DD`), RFC3339 timestamps with `Z` or numeric offsets and up to nine fractional digits, or signed Unix seconds using `@` notation. Calendar date forms are limited to years 1970–9999; Unix instants may precede the epoch when representable.
- Modification-time predicates inspect the entry's own timestamp, including symlinks. Directory time filters affect whether that directory is emitted; they do not prune its descendants.
- The `--ignore` source set does not evaluate Git's command-line `-c` values or `GIT_CONFIG_COUNT` overrides. `includeIf hasconfig:` is unsupported and reported; the implementation does not guess an implicit Windows system config path.
- `--one-file-system` is currently available on Unix only. Following directory symlinks or requesting contents-first output forces sequential traversal. If the open-directory limit is reached, pending ancestor entries spill to memory; this bounds handles but may increase retained memory and I/O.
- Without `--unicode`, matching is byte-oriented on native platform encodings. On Windows the matcher uses `OsStr::as_encoded_bytes` rather than a lossy UTF-16 conversion.
- Human-readable Windows output can replace unpaired UTF-16 surrogates. JSON Lines includes `path_utf16` for a non-UTF-8-encodable path; Unix JSON Lines includes `path_bytes` as lowercase hex. In both cases the companion field is the lossless representation.
- Newline-delimited plain output cannot unambiguously represent a filename containing a newline. Use JSON Lines when Unix paths may contain non-UTF-8 bytes or control characters.
- Saved profiles are independent and do not inherit `default`. Their config directory is the default query root. Lists from a profile and the command line append; command-line scalar options override profile values. Output and execution settings are command-line only.

Compatibility claims apply only to the supported table and are not a claim of drop-in replacement behavior.

See [feature status and replacements](docs/reference/feature-status.md) for the full implemented surface and remaining gaps. Ignore loading is opt-in; repository/global Git sources are included by `--gitignore` and `--ignore`, though not every Git config or escaping edge case is implemented.
