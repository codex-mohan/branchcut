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
| `--first` / `--limit` | Supported | Stops after enough discovered matches when streaming |
| `--sort` | Supported | Sorts the complete result set globally; a sorted limit selects from that full ordering |
| `--stats` | Supported | Real traversal counters |
| `--explain` | Supported | Planner decisions and strategy |
| Positional pattern | Supported | Arguments containing glob syntax are treated as glob patterns |
| Positional search | Supported | Plain text is literal, case-sensitive filename containment |
| `--strict` | Supported | Exit code 2 on filesystem errors |
| `--count` | Supported | Emits the match count without path serialization |
| `--gitignore` | Supported | Root and nested rules, ordered overrides, directory rules, and conservative re-inclusion traversal |
| `--json` | Supported | Streaming JSON Lines output |
| `--exec` | Supported | Shell-free command templates; native path substitution; child cwd is the query root |
| `--threads N` | Supported | Bounded parallel traversal with buffered results; incompatible with `--limit` and `--exec` |

## Deliberate gaps

- Advanced `.gitignore` escaping and every Git edge case are not claimed.
- No extglobs, including `+(...)`, `?(...)`, `*(...)`, `@(...)`, and `!(...)`.
- No nested brace expansion or brace ranges.
- No full `fast-glob` options/API compatibility.
- No captures, metadata predicates, or watch mode.
- No case-insensitive or smart-case mode.
- Wildcard matching is byte-oriented rather than Unicode-scalar-oriented.
- Matching is byte-oriented on native platform encodings, not Unicode-scalar or grapheme-oriented. On Windows the matcher uses `OsStr::as_encoded_bytes` rather than a lossy UTF-16 conversion.
- Human-readable Windows output can replace unpaired UTF-16 surrogates. JSON Lines includes `path_utf16` for a non-UTF-8-encodable path; Unix JSON Lines includes `path_bytes` as lowercase hex. In both cases the companion field is the lossless representation.
- Newline-delimited plain output cannot unambiguously represent a filename containing a newline. Use JSON Lines when Unix paths may contain non-UTF-8 bytes or control characters.

Compatibility claims apply only to the supported table and are not a claim of drop-in replacement behavior.
