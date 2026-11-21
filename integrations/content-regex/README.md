# Branchcut regex integration

Build this optional executable to select filenames/paths and search selected file contents with Rust regular expressions. The main Branchcut package builds independently with only the standard library.

```sh
cargo install --locked --path integrations/content-regex
branchcut-regex find -E '^test_.*\.rs$'
branchcut-regex find -E '^src/.+\.rs$' --full-path
branchcut-regex grep -E --glob 'src/**/*.rs' 'TODO|FIXME'
branchcut-regex grep -E --glob '**/*.log' --json 'ERROR [0-9]+'
```

`branchcut-regex find -E PATTERN [PATH...]` selects names, or slash-separated root-relative paths with `--full-path`. `branchcut-regex grep -E PATTERN [PATH...]` searches content. Both commands accept other Branchcut filters. Patterns are literals without `-E`, and `-F` selects literals explicitly.

Each content regular expression runs on one line. The integration returns byte spans with the index of the matching pattern. Use repeatable `grep -e PATTERN` or `grep -f FILE` for several patterns and `--content-mode` to select any/all/none matching. Once patterns are explicit, remaining positionals are input paths. Patterns are UTF-8, while input can contain arbitrary bytes. Use `--unicode` for Unicode matching and case folding, or an inline flag such as `(?u:...)` for a specific expression. `grep` returns 0 for a selected result, 1 for none, and 2 for errors.

Anchors, groups, alternation, character classes, and repetition follow [Rust regex syntax](https://docs.rs/regex/latest/regex/#syntax). Expressions that can match zero bytes count as matches and may produce empty spans. Match spans describe the full expression. The integration does not report capture groups, span multiple lines, or provide PCRE lookaround and backreferences.

The optional package uses `regex` for matching and pins its resolved dependency graph in its own `Cargo.lock`. No dependency is added to the ordinary CLI or core library. The adapter accepts up to 128 expressions and applies per-expression state size, cache size, and nesting limits. Structured output reports a scan error if a line produces more than 65,536 match spans.

```sh
cargo test --locked --manifest-path integrations/content-regex/Cargo.toml
cargo tree --locked -e normal --manifest-path integrations/content-regex/Cargo.toml
```
