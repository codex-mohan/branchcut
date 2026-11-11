#!/usr/bin/env node
// Cold-process comparison against available filesystem enumeration tools.
// Results are meaningful only after complete path-set equivalence succeeds.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { createRequire } from 'node:module';
import os from 'node:os';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { access, mkdtemp, mkdir, readFile, rm, statfs, writeFile } from 'node:fs/promises';
import { performance } from 'node:perf_hooks';

const PINNED_FAST_GLOB = '3.3.3';
const hash = (data) => createHash('sha256').update(data).digest('hex');
const normalize = (values) => values
  .map((value) => value.replaceAll('\\', '/').replace(/^\.\//, ''))
  .sort();

function usage() {
  return 'node scripts/compare-current.mjs --binary PATH --oracle-dir PATH --output PATH [--sizes 10000,100000] [--warmups 2] [--iterations 9] [--thread-matrix] [--zlob-probe PATH]\n';
}

function parseArgs(args) {
  const options = { sizes: [10_000, 100_000], warmups: 2, iterations: 9 };
  for (let i = 0; i < args.length; i++) {
    const key = args[i];
    if (key === '--help' || key === '-h') {
      process.stdout.write(usage());
      process.exit(0);
    }
    if (key === '--thread-matrix') { options.thread_matrix = true; continue; }
    if (!['--binary', '--oracle-dir', '--output', '--sizes', '--warmups', '--iterations', '--zlob-probe'].includes(key)) {
      throw new Error(`Unknown option: ${key}\n${usage()}`);
    }
    const value = args[++i];
    if (!value || value.startsWith('--')) throw new Error(`Missing value for ${key}`);
    const name = key.slice(2).replaceAll('-', '_');
    if (name === 'sizes') options.sizes = value.split(',').map(Number);
    else if (name === 'warmups' || name === 'iterations') options[name] = Number(value);
    else options[name] = path.resolve(value);
  }
  for (const key of ['binary', 'oracle_dir', 'output']) if (!options[key]) throw new Error(`Missing --${key}`);
  if (!Number.isSafeInteger(options.iterations) || options.iterations < 1) throw new Error('--iterations must be positive');
  if (!Number.isSafeInteger(options.warmups) || options.warmups < 0) throw new Error('--warmups must be nonnegative');
  if (options.sizes.length === 0 || options.sizes.some((count) => !Number.isSafeInteger(count) || count < 160 || count % 20 !== 0)) {
    throw new Error('--sizes must be comma-separated positive multiples of 20');
  }
  return options;
}

function run(program, args, extra = {}) {
  const result = spawnSync(program, args, {
    encoding: 'utf8',
    windowsHide: true,
    maxBuffer: 128 * 1024 * 1024,
    ...extra,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${program} ${args.join(' ')} exited ${result.status}:\n${result.stderr}`);
  return result;
}

function percentile(values, p) {
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.ceil((sorted.length - 1) * p)];
}

function distribution(samples) {
  const values = samples.map((sample) => sample.elapsed_ms);
  const mean = values.reduce((sum, value) => sum + value, 0) / values.length;
  const variance = values.reduce((sum, value) => sum + (value - mean) ** 2, 0) / values.length;
  const deviation = Math.sqrt(variance);
  return {
    median_ms: percentile(values, 0.5),
    p90_ms: percentile(values, 0.9),
    p95_ms: percentile(values, 0.95),
    standard_deviation_ms: deviation,
    dispersion_percent: mean === 0 ? 0 : deviation / mean * 100,
  };
}

async function makeCorpus(root, totalFiles) {
  const records = [];
  const perPackage = totalFiles / 20;
  const locations = ['src', 'src/nested/deep', 'src', 'docs', 'target/debug', 'node_modules/dep', 'src/upper', 'src'];
  const extensions = ['rs', 'rs', 'toml', 'md', 'rs', 'toml', 'txt', 'txt'];
  for (let packageIndex = 0; packageIndex < 20; packageIndex++) {
    const packageName = `p${String(packageIndex).padStart(2, '0')}`;
    for (const directory of new Set(locations)) {
      await mkdir(path.join(root, 'packages', packageName, directory), { recursive: true });
    }
    for (let start = 0; start < perPackage; start += 128) {
      await Promise.all(Array.from({ length: Math.min(128, perPackage - start) }, async (_, offset) => {
        const index = start + offset;
        const relative = `packages/${packageName}/${locations[index % locations.length]}/f${String(index).padStart(5, '0')}.${extensions[index % extensions.length]}`;
        await writeFile(path.join(root, relative), 'generated benchmark file\n');
        records.push(relative);
      }));
    }
  }
  return records.sort();
}

function makeWorkloads(allPaths) {
  const cases = [
    {
      name: 'broad-files',
      patterns: ['**/*'],
      excludes: [],
      branchcut: ['--glob', '**/*'],
      rg: ['--glob', '**/*'],
      fd: ['--glob', '*'],
      expected: allPaths,
    },
    {
      name: 'fixed-prefix-rust',
      patterns: ['packages/p00/src/**/*.rs'],
      excludes: [],
      branchcut: ['--glob', 'packages/p00/src/**/*.rs'],
      rg: ['--glob', 'packages/p00/src/**/*.rs'],
      fd: ['--glob', '*.rs', 'packages/p00/src'],
      expected: allPaths.filter((file) => file.startsWith('packages/p00/src/') && file.endsWith('.rs')),
    },
    {
      name: 'rust-and-toml-excluding-build-and-dependencies',
      patterns: ['**/*.rs', '**/*.toml'],
      excludes: ['**/target/**', '**/node_modules/**'],
      branchcut: ['--glob', '**/*.{rs,toml}', '--exclude', '**/target/**', '--exclude', '**/node_modules/**'],
      rg: ['--glob', '**/*.rs', '--glob', '**/*.toml', '--glob', '!**/target/**', '--glob', '!**/node_modules/**'],
      fd: ['--extension', 'rs', '--extension', 'toml', '--exclude', 'target', '--exclude', 'node_modules'],
      expected: allPaths.filter((file) => /\.(rs|toml)$/.test(file) && !file.includes('/target/') && !file.includes('/node_modules/')),
    },
  ];
  return cases.map((testCase) => ({
    ...testCase,
    expected: normalize(testCase.expected),
    expected_sha256: hash(JSON.stringify(normalize(testCase.expected))),
  }));
}

function assertSelection(label, expected, actual) {
  const paths = normalize(actual);
  assert.equal(new Set(paths).size, paths.length, `${label} returned duplicate paths`);
  assert.deepEqual(paths, expected, `${label} did not match the deterministic fixture manifest`);
}

function readPaths(stdout) {
  return stdout.replace(/\r/g, '').split('\n').filter(Boolean);
}

function oracleWorker() {
  return `const path = require('node:path');
const { createRequire } = require('node:module');
const req = createRequire(path.join(process.argv[1], 'branchcut-oracle-loader.cjs'));
const fg = req('fast-glob');
const patterns = JSON.parse(process.argv[2]);
const ignore = JSON.parse(process.argv[3]);
fg(patterns, { cwd: process.argv[4], ignore, dot: false, onlyFiles: true, unique: true, followSymbolicLinks: false })
  .then((paths) => process.stdout.write(paths.join('\\n') + (paths.length ? '\\n' : '')))
  .catch((error) => { process.stderr.write(String(error.stack || error)); process.exitCode = 1; });`;
}

async function shellDiscovery() {
  if (process.platform === 'win32') {
    try {
      const result = run('pwsh', ['-NoProfile', '-NonInteractive', '-Command',
        "$found = Get-Command rg,fd -ErrorAction SilentlyContinue | Select-Object Name,Source; ConvertTo-Json -InputObject @($found) -Compress"]);
      return { method: 'Get-Command rg,fd', tools: JSON.parse(result.stdout || '[]') };
    } catch (error) {
      return { method: 'Get-Command rg,fd', error: String(error) };
    }
  }
  return { method: 'PATH lookup and --version', tools: [] };
}

async function fileHash(file) {
  return hash(await readFile(file));
}

async function executablePath(name, discovery) {
  const discovered = discovery.tools.find((candidate) => candidate.Name?.toLowerCase() === `${name}.exe`)?.Source;
  if (discovered) return discovered;
  for (const directory of (process.env.PATH || '').split(path.delimiter)) {
    const candidate = path.join(directory, name + (process.platform === 'win32' ? '.exe' : ''));
    try { await access(candidate); return candidate; } catch { /* try the next PATH entry */ }
  }
  throw new Error(`cannot resolve executable ${name}`);
}

async function sourceIdentity() {
  const files = ['Cargo.toml', 'Cargo.lock'];
  async function walk(directory) {
    for (const entry of (await (await import('node:fs/promises')).readdir(directory, { withFileTypes: true })).sort((a, b) => a.name.localeCompare(b.name))) {
      const child = path.join(directory, entry.name);
      if (entry.isDirectory()) await walk(child);
      else if (child.endsWith('.rs')) files.push(child);
    }
  }
  await walk('src');
  const manifest = [];
  for (const file of files.sort()) manifest.push({ file: file.replaceAll('\\', '/'), sha256: await fileHash(file) });
  const commit = run('git', ['rev-parse', 'HEAD']).stdout.trim();
  return {
    commit,
    dirty: !!run('git', ['status', '--porcelain']).stdout.trim(),
    source_manifest: manifest,
    source_manifest_sha256: hash(JSON.stringify(manifest)),
  };
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  const oracleRequire = createRequire(pathToFileURL(path.join(options.oracle_dir, 'branchcut-oracle-loader.cjs')));
  const oraclePackage = oracleRequire('fast-glob/package.json');
  if (oraclePackage.version !== PINNED_FAST_GLOB) throw new Error(`Expected fast-glob@${PINNED_FAST_GLOB}, found ${oraclePackage.version}`);

  const discovery = await shellDiscovery();
  const toolVersions = { branchcut: null, rg: null, fd: null, fast_glob: { version: oraclePackage.version, package_sha256: await fileHash(oracleRequire.resolve('fast-glob/package.json')) } };
  const candidates = [
    { name: 'branchcut', command: options.binary, versionArgs: ['--version'] },
    { name: 'rg', command: 'rg', versionArgs: ['--version'] },
    { name: 'fd', command: 'fd', versionArgs: ['--version'] },
  ];
  for (const tool of candidates) {
    const result = spawnSync(tool.command, tool.versionArgs, { encoding: 'utf8', windowsHide: true });
    if (result.error || result.status !== 0) {
      toolVersions[tool.name] = { available: false, reason: result.error?.message ?? result.stderr ?? `exit ${result.status}` };
      if (tool.name === 'branchcut' || tool.name === 'rg') throw new Error(`${tool.name} is required: ${toolVersions[tool.name].reason}`);
      continue;
    }
    toolVersions[tool.name] = {
      available: true,
      version: (result.stdout || result.stderr).trim().split(/\r?\n/)[0],
      executable_sha256: await fileHash(tool.name === 'branchcut' ? options.binary : await executablePath(tool.name, discovery)),
    };
  }
  if (options.zlob_probe) {
    toolVersions.zlob_native_walker = {
      version: '1.7.0',
      revision: '5979979703ba21c2b47645aced50aac59ab1bca9',
      zig: '0.16.0',
      build: 'official ReleaseFast library; own public-C-API adapter compiled -O3',
      backend: process.platform === 'win32' ? 'windows_ntdll' : 'see platform source selection',
      probe_sha256: await fileHash(options.zlob_probe),
      probe_source_sha256: await fileHash('benchmarks/probes/zlob_walk.c'),
      library_sha256: process.platform === 'win32' ? await fileHash(path.join(path.dirname(options.zlob_probe), 'zlob.dll')) : null,
      output: 'zlob_walk_collect followed by relative regular-file output; no metadata/ignore/follow; no sort',
    };
  }

  const system = {
    os: `${os.type()} ${os.release()} ${os.arch()}`,
    cpu: os.cpus()[0]?.model ?? 'unknown',
    logical_cpus: os.cpus().length,
    ram_bytes: os.totalmem(),
    node: process.version,
    rustc: run('rustc', ['--version']).stdout.trim(),
    filesystem_type: null,
  };
  const root = await mkdtemp(path.join(os.tmpdir(), 'branchcut-current-tools-'));
  if (process.platform === 'win32') {
    try {
      system.filesystem_type = run('pwsh', ['-NoProfile', '-NonInteractive', '-Command',
        "(Get-Volume -DriveLetter (Split-Path -Qualifier $env:TEMP).TrimEnd(':')).FileSystem"]).stdout.trim() || 'unknown';
    } catch { system.filesystem_type = 'unknown'; }
  } else {
    try { system.filesystem_type = `0x${Number((await statfs(root)).type).toString(16)}`; } catch { system.filesystem_type = 'unknown'; }
  }
  const record = {
    schema_version: 1,
    recorded_at: new Date().toISOString(),
    method: 'Fresh process for each sample; full relative path output is captured for all tools; includes process startup, query, traversal, and output capture. Warmups are fresh-process runs which prime uncontrolled filesystem caches. This is not a hot-engine or cold-disk comparison.',
    correctness: 'Complete sorted path sets compared to the deterministic generator manifest and to one another before timing; every timed output is checked again; duplicates are rejected.',
    limitations: ['Generated synthetic corpus only; no public repository or competitor-wide speed claim.', 'Filesystem cache state is uncontrolled.', 'No result sorting is requested; comparison normalizes order after capture.', 'Branchcut output uses line-delimited names; generated filenames contain no control characters.', 'No CPU or RSS measurements are included in this runner.'],
    source: await sourceIdentity(),
    system,
    harness_sha256: await fileHash('scripts/compare-current.mjs'),
    executable_discovery: { ...discovery, tools: discovery.tools.map(({ Name, Source }) => ({ name: Name, executable: path.basename(Source) })) },
    thread_policy: 'Branchcut default is sequential; branchcut_auto requests --threads 0 (available parallelism capped at 16). rg and fd use their own defaults. This is an out-of-box comparison, not equal worker counts.',
    thread_matrix: Boolean(options.thread_matrix),
    tools: toolVersions,
    dataset: { shape: '20 packages; entries cycle through src, nested src, docs, target/debug, node_modules/dep, and extension distractors', requested_sizes: options.sizes },
    warmups: options.warmups,
    iterations: options.iterations,
    raw_workloads: [],
  };
  const save = async () => {
    await mkdir(path.dirname(path.resolve(options.output)), { recursive: true });
    await writeFile(options.output, JSON.stringify(record, null, 2) + '\n');
  };

  try {
    for (const size of options.sizes) {
      const fixture = path.join(root, String(size));
      await mkdir(fixture);
      const allPaths = await makeCorpus(fixture, size);
      for (const testCase of makeWorkloads(allPaths)) {
        const branchcut = ['--cwd', fixture, ...testCase.branchcut];
        const rgArgs = ['--files', '--no-ignore', ...testCase.rg, '.'];
        const fdArgs = ['--type', 'file', '--no-ignore', '--no-hidden', ...testCase.fd];
        const oracleArgs = ['--eval', oracleWorker(), options.oracle_dir, JSON.stringify(testCase.patterns), JSON.stringify(testCase.excludes), fixture];
        const invocations = {
          branchcut: () => run(options.binary, branchcut),
          branchcut_auto: () => run(options.binary, [...branchcut, '--threads', '0']),
          rg: () => run('rg', rgArgs, { cwd: fixture }),
          fd: () => toolVersions.fd.available ? run('fd', fdArgs, { cwd: fixture }) : null,
          fast_glob: () => run(process.execPath, oracleArgs),
        };
        if (options.thread_matrix) {
          invocations.rg_1 = () => run('rg', [...rgArgs, '--threads', '1'], { cwd: fixture });
          invocations.rg_4 = () => run('rg', [...rgArgs, '--threads', '4'], { cwd: fixture });
          invocations.branchcut_4 = () => run(options.binary, [...branchcut, '--threads', '4']);
          if (toolVersions.fd.available) {
            invocations.fd_1 = () => run('fd', [...fdArgs, '--threads', '1'], { cwd: fixture });
            invocations.fd_4 = () => run('fd', [...fdArgs, '--threads', '4'], { cwd: fixture });
          }
        }
        const zlobPattern = testCase.patterns.length === 1 && testCase.excludes.length === 0 ? testCase.patterns[0] : null;
        if (options.zlob_probe && zlobPattern) {
          for (const threads of [1, 4, 0]) invocations[`zlob_${threads || 'auto'}`] = () => run(options.zlob_probe, [fixture, zlobPattern, String(threads), 'paths']);
        }

        const expected = testCase.expected;
        const verified = {};
        for (const name of Object.keys(invocations)) {
          if (name === 'fd' && !toolVersions.fd.available) continue;
          const output = invocations[name]();
          verified[name] = readPaths(output.stdout);
          assertSelection(`${name}/${size}/${testCase.name}`, expected, verified[name]);
        }
        const names = Object.keys(verified);
        for (let i = 1; i < names.length; i++) assert.deepEqual(normalize(verified[names[i]]), normalize(verified[names[0]]), `${names[i]} disagrees with ${names[0]}`);

        const workload = {
          name: testCase.name,
          corpus_files: size,
          queries: {
            branchcut: ['branchcut', ...branchcut.map((value) => value === fixture ? '$FIXTURE' : value)],
            branchcut_auto: ['branchcut', ...branchcut.map((value) => value === fixture ? '$FIXTURE' : value), '--threads', '0'],
            rg: ['rg', ...rgArgs],
            fd: toolVersions.fd.available ? ['fd', ...fdArgs] : null,
            fast_glob: { process: 'node --eval <worker>', patterns: testCase.patterns, options: { cwd: '$FIXTURE', ignore: testCase.excludes, dot: false, onlyFiles: true, unique: true, followSymbolicLinks: false } },
            matrix_overrides: options.thread_matrix ? { branchcut_4: '--threads 4', rg_1: '--threads 1', rg_4: '--threads 4', fd_1: '--threads 1', fd_4: '--threads 4' } : null,
            zlob: options.zlob_probe && zlobPattern ? { argv: ['zlob-walk-probe', '$FIXTURE', zlobPattern, '1|4|0', 'paths'] } : null,
          },
          expected_matches: expected.length,
          zlob_skip_reason: options.zlob_probe && !zlobPattern ? 'adapter accepts one include pattern and no explicit excludes; excluded workload not compared' : null,
          result_sha256: testCase.expected_sha256,
          raw_samples: Object.fromEntries(names.map((name) => [name, []])),
        };
        const sequence = names;
        for (let sample = 0; sample < options.warmups + options.iterations; sample++) {
          const rotated = [...sequence.slice(sample % sequence.length), ...sequence.slice(0, sample % sequence.length)];
          for (const name of rotated) {
            const started = performance.now();
            const output = invocations[name]();
            const elapsed_ms = performance.now() - started;
            const paths = readPaths(output.stdout);
            assertSelection(`${name}/${size}/${testCase.name}/sample-${sample}`, expected, paths);
            if (sample >= options.warmups) {
              workload.raw_samples[name].push({ elapsed_ms, stdout_bytes: Buffer.byteLength(output.stdout) });
            }
          }
        }
        workload.summary = Object.fromEntries(Object.entries(workload.raw_samples).map(([name, samples]) => {
          return [name, distribution(samples)];
        }));
        record.raw_workloads.push(workload);
        await save();
        const display = Object.entries(workload.summary).map(([name, result]) => `${name} ${result.median_ms.toFixed(2)}ms`).join(' | ');
        process.stdout.write(`${size} ${testCase.name}: ${expected.length} matches | ${display}\n`);
      }
    }
    record.status = 'complete';
    await save();
  } catch (error) {
    record.status = 'failed';
    record.error = String(error?.stack ?? error);
    await save();
    throw error;
  } finally {
    const temporaryRoot = path.resolve(os.tmpdir());
    const relative = path.relative(temporaryRoot, path.resolve(root));
    if (!relative || relative.startsWith('..') || path.isAbsolute(relative) || !path.basename(root).startsWith('branchcut-current-tools-')) {
      throw new Error(`Refusing unsafe temporary-tree cleanup: ${root}`);
    }
    await rm(root, { recursive: true, force: true });
  }
}

main().catch((error) => {
  process.stderr.write(`current-tool comparison: ${error.stack ?? error}\n`);
  process.exitCode = 1;
});
