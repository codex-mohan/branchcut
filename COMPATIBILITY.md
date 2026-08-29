# Compatibility

Branchcut intentionally supports a focused filesystem-query surface rather than the full `fast-glob` API.

## Supported

| Syntax or option | Status | Notes |
|---|---|---|
| `*` | Supported | Matches bytes within one path component |
| `?` | Supported | One byte within one path component |
| `**` | Supported | Matches zero or more path components |
| `[abc]` | Supported | Byte classes |
| `[a-z]` | Supported | Inclusive byte ranges |
| `[!abc]` / `[^abc]` | Supported | Negated classes |
| `*.{rs,toml}` | Supported | One flat brace group |
| Multiple `--glob` | Supported | Compiled into one shared program |
| `--exclude` | Supported | Trailing `/**` exclusions prune subtrees |
| `--type file` | Supported | Default type |
| `--type dir` | Supported | Directories are emitted only when explicitly requested |
| `--type symlink` | Supported | Symlinks are not followed |
| `-e` | Supported | Case-sensitive suffix filter |
| `--hidden` | Supported | Hidden components excluded by default |
| `--cwd` | Supported | Relative output paths |
| `--first` / `--limit` | Supported | Immediate stop in streaming mode |
| `--sort` | Supported | Collects all results, sorts globally, then truncates |
| `--stats` | Supported | Real traversal counters |
| `--explain` | Supported | Planner decisions and strategy |
| Positional search | Supported | Literal, case-sensitive filename containment |
| `--strict` | Supported | Exit code 2 on filesystem errors |

## Deliberate gaps

- No `.gitignore` parsing or hierarchical ignore precedence.
- No extglobs, including `+(...)`, `?(...)`, `*(...)`, `@(...)`, and `!(...)`.
- No nested brace expansion or brace ranges.
- No full `fast-glob` options/API compatibility.
- No captures, JSON output, command execution, metadata predicates, or watch mode.
- No case-insensitive or smart-case mode.
- No parallel traversal.
- Wildcard matching is byte-oriented rather than Unicode-scalar-oriented.
- Windows paths with characters not representable by the current lossy matching view are not fully byte-preserving.

Compatibility claims apply only to the supported table and are not a claim of drop-in replacement behavior.
