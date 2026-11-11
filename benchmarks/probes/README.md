# Native walker probes

These development-only wrappers are separately built benchmark tools. They do not add dependencies to Branchcut's core or ordinary CLI and contain no copied competitor implementation.

## zlob 1.7.0 native walker

`zlob_walk.c` calls the public `zlob_walk` and `zlob_walk_collect` C APIs from the official zlob library. These APIs exercise the new walker; they are distinct from the older glob/match path used by zlob's command-line executable.

Pinned official reference: [`dmtrKovalenko/zlob` at `5979979703ba21c2b47645aced50aac59ab1bca9`](https://github.com/dmtrKovalenko/zlob/blob/5979979703ba21c2b47645aced50aac59ab1bca9/src/walker/walker.zig). The exact commit, source hash, compiler, backend, and binary hashes belong in retained records.

Windows build, using Zig 0.16.0 (replace the executable path if necessary):

```powershell
$probeZig = 'C:/path/to/zig-0.16.0/zig.exe'
Push-Location target/perf-reference/zlob
& $probeZig build -Doptimize=ReleaseFast -Dskip-bench=true
Pop-Location
& $probeZig cc -O3 -std=c11 -Wall -Wextra -Werror -I target/perf-reference/zlob/include benchmarks/probes/zlob_walk.c target/perf-reference/zlob/zig-out/lib/zlob.lib -o target/perf-reference/zlob/zig-out/bin/zlob-walk-probe.exe
```

The generated `zlob.lib` is an import library; leave `zlob.dll` next to the probe executable. The official library's compile-time backend selection chooses `windows_ntdll` on this Windows build and reaches `NtQueryDirectoryFile`; the probe does not implement or modify platform enumeration.

```powershell
target/perf-reference/zlob/zig-out/bin/zlob-walk-probe.exe ROOT '**/*.rs' 1 paths
target/perf-reference/zlob/zig-out/bin/zlob-walk-probe.exe ROOT '**/*.rs' 1 count
target/perf-reference/zlob/zig-out/bin/zlob-walk-probe.exe ROOT '**/*.rs' 4 collect-count
```

- `paths` collects using the public walker and writes regular-file paths relative to ROOT, one per line, without sorting.
- `count` uses the streaming walker and padded per-worker counters; callback state is partitioned by the public `worker_id` contract. It serializes only the final count.
- `collect-count` uses the collecting API and serializes only the regular-file count. This mode measures collection overhead as well as traversal.
- A pattern of `-` disables the glob filter. Other patterns use the library's default brace and recursive-globstar flags.
- Worker count `0` selects the library default; explicit values 1–1,024 select its supported explicit range. Compare explicit equal worker counts when testing traversal throughput.

All modes disable ignores, exclude dot-hidden entries/subtrees, do not follow symlinks, request no optional metadata, omit directories, filter other kinds to regular files, and abort on filesystem errors. The walker may still deliver symlink/other non-directory entries internally before the wrapper's regular-file check. Clone the official `v1.7.0` tag into ignored `target/perf-reference/zlob` and verify commit `5979979703ba21c2b47645aced50aac59ab1bca9` before building.

Verify complete path sets before timing. Newline-delimited probe output is intended for controlled ASCII fixtures; unusual filenames and Unicode command-line roots have not been validated. The public walker currently uses a 4,096-byte path buffer on Windows. `paths` is a collecting path and `count` is streaming, so do not present those modes as equivalent memory or output workloads. No speed claim follows from building or smoke-testing the probe.

The comparison runner optionally includes this probe for workloads it can express:

```powershell
node scripts/compare-current.mjs --binary target/release/branchcut.exe --oracle-dir target/oracle --sizes 100000 --warmups 2 --iterations 7 --thread-matrix --zlob-probe target/perf-reference/zlob/zig-out/bin/zlob-walk-probe.exe --output benchmarks/results/local-zlob-native.json
```

It validates full output sets and retains executable/DLL hashes. Workloads requiring explicit exclusions are skipped for the probe rather than timing unequal selections.
