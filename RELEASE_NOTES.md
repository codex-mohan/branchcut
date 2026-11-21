# Branchcut v0.3.0

Branchcut can now search file contents after narrowing the search with the same glob, ignore, and metadata filters used for filesystem queries.

```sh
branchcut config
branchcut find config src/
branchcut grep -i -C 2 'connection refused' logs/
branchcut-regex find -E '^test_.*[.]py$' tests/
branchcut-regex grep -E 'ERROR [0-9]+' logs/
```

- `find` and `grep` take the pattern first and optional input paths after it. `grep` accepts `-` for stdin and returns 0 for a match, 1 for no match, and 2 for errors.
- Search results include line and byte column, context, matched spans, per-file counts, and matching or nonmatching file lists. Use streaming JSON Lines, JSON, CSV, TSV, plain text templates, or NUL-delimited paths.
- Search BOM-marked UTF-8 and UTF-16, Latin-1, Windows-1252, raw bytes, or a preprocessor's output. Compression support uses external decompressors.
- Literal scans share work across patterns, borrow complete lines from the read buffer, and use bounded parallel file scanning where applicable.
- The ordinary executable remains dependency-free. The separately built `branchcut-regex` executable adds regular expressions for filenames, paths, and individual content lines. Regex captures and multiline matches are outside this release.

Download one of the Windows x64, Linux x64, macOS Intel, or macOS Apple Silicon archives. Each includes both executables and the optional regex build's license notices. Verify the archive with `SHA256SUMS`, extract it, and add the executables you use to your `PATH`.

If an old command searched for `grep` or `find` alongside other positional terms, put `--` before those terms to force the original filename search. A single-word search such as `branchcut grep` still searches filenames.
