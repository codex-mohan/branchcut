#!/usr/bin/env node
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { createRequire } from 'node:module';
import os from 'node:os';
import path from 'node:path';
import { performance } from 'node:perf_hooks';
import { pathToFileURL } from 'node:url';
import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';

const EXPECTED_FAST_GLOB = '3.3.3';
const PACKAGE_COUNT = 20;
const FILES_PER_KIND_PER_PACKAGE = 250;
const DISTRACTOR_FILES_PER_PACKAGE = 150;
const GLOBS = ['**/*.rs', '**/*.toml'];
const EXCLUDES = ['**/target/**', '**/node_modules/**'];
function usage() {
  return `Usage: node scripts/benchmark.mjs --binary PATH --oracle-dir PATH [--iterations N] [--warmups N] [--output PATH]\n`;
}

function parseArgs(argv) {
  const options = { iterations: 10, warmups: 2 };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === '--help' || arg === '-h') {
      process.stdout.write(usage());
      process.exit(0);
    }
    if (!['--binary', '--oracle-dir', '--iterations', '--warmups', '--output'].includes(arg)) {
      throw new Error(`Unknown option: ${arg}\n${usage()}`);
    }
    const value = argv[++i];
    if (!value || value.startsWith('--')) throw new Error(`Missing value for ${arg}`);
    const name = arg.slice(2).replaceAll('-', '_');
    options[name] = name === 'iterations' || name === 'warmups'
      ? Number.parseInt(value, 10)
      : path.resolve(value);
    if ((name === 'iterations' || name === 'warmups') && (!Number.isSafeInteger(options[name]) || options[name] < 0)) {
      throw new Error(`${arg} must be a nonnegative integer`);
    }
  }
  if (!options.binary || !options.oracle_dir) throw new Error(usage());
  if (options.iterations < 1) throw new Error('--iterations must be greater than zero');
  return options;
}

function run(program, args, { cwd } = {}) {
  const result = spawnSync(program, args, {
    cwd,
    encoding: 'utf8',
    windowsHide: true,
    maxBuffer: 64 * 1024 * 1024,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    throw new Error(`${path.basename(program)} exited ${result.status}:\n${result.stderr}`);
  }
  return result;
}

async function createCorpus() {
  const root = await mkdtemp(path.join(os.tmpdir(), 'branchcut-bench-'));
  try {
    for (let packageIndex = 0; packageIndex < PACKAGE_COUNT; packageIndex += 1) {
      const packageName = `package-${String(packageIndex).padStart(2, '0')}`;
      const packageRoot = path.join(root, 'packages', packageName);
      const sources = path.join(packageRoot, 'src');
      const target = path.join(packageRoot, 'target', 'debug');
      const modules = path.join(packageRoot, 'node_modules', 'dep');
      await Promise.all([
        mkdir(path.join(sources, 'unit', 'deep'), { recursive: true }),
        mkdir(path.join(sources, 'integration'), { recursive: true }),
        mkdir(target, { recursive: true }),
        mkdir(modules, { recursive: true }),
      ]);
      const writes = [];
      for (let fileIndex = 0; fileIndex < FILES_PER_KIND_PER_PACKAGE; fileIndex += 1) {
        const number = String(fileIndex).padStart(4, '0');
        const directory = fileIndex % 4 === 0
          ? path.join(sources, 'unit', 'deep')
          : fileIndex % 4 === 1
            ? path.join(sources, 'integration')
            : sources;
        const base = path.join(directory, `source-${number}`);
        writes.push(writeFile(`${base}.rs`, 'fn generated() {}\n'));
        writes.push(writeFile(`${base}.toml`, 'generated = true\n'));
      }
      for (let fileIndex = 0; fileIndex < DISTRACTOR_FILES_PER_PACKAGE; fileIndex += 1) {
        const number = String(fileIndex).padStart(4, '0');
        writes.push(writeFile(path.join(target, `generated-${number}.rs`), 'fn generated() {}\n'));
        writes.push(writeFile(path.join(modules, `dependency-${number}.ts`), 'export {};\n'));
      }
      await Promise.all(writes);
    }
    const matched = PACKAGE_COUNT * FILES_PER_KIND_PER_PACKAGE * 2;
    const total = PACKAGE_COUNT * (FILES_PER_KIND_PER_PACKAGE * 2 + DISTRACTOR_FILES_PER_PACKAGE * 2);
    return { root, matched, total };
  } catch (error) {
    const resolved = path.resolve(root);
    if (resolved.startsWith(path.resolve(os.tmpdir()) + path.sep)) {
      await rm(resolved, { recursive: true, force: true });
    }
    throw error;
  }
}

function normalized(paths) {
  return paths.map((value) => value.replaceAll('\\', '/')).sort();
}

function compareResults(branchcutPaths, oraclePaths) {
  const branchcut = normalized(branchcutPaths);
  const oracle = normalized(oraclePaths);
  if (new Set(branchcut).size !== branchcut.length) throw new Error('Branchcut returned duplicate paths');
  if (new Set(oracle).size !== oracle.length) throw new Error('fast-glob returned duplicate paths');
  if (branchcut.length !== oracle.length || branchcut.some((value, index) => value !== oracle[index])) {
    const actual = new Set(branchcut);
    const expected = new Set(oracle);
    const missing = oracle.filter((value) => !actual.has(value));
    const extra = branchcut.filter((value) => !expected.has(value));
    throw new Error(`Correctness check failed before timing.\nMissing: ${missing.slice(0, 20).join(', ') || '(none)'}\nExtra: ${extra.slice(0, 20).join(', ') || '(none)'}`);
  }
  return branchcut.length;
}

function parseStats(stderr) {
  const stats = {};
  const fields = {
    matched: 'matches',
    'directories considered': 'directories_considered',
    'directories opened': 'directories_opened',
    'directories pruned': 'directories_pruned',
    'entries inspected': 'entries_inspected',
    'candidate files': 'candidate_files',
    'metadata calls': 'metadata_calls',
    'filesystem errors': 'filesystem_errors',
  };
  for (const [label, field] of Object.entries(fields)) {
    const found = stderr.match(new RegExp(`^${label}\\s+(\\d+)\\s*$`, 'm'));
    if (found) stats[field] = Number(found[1]);
  }
  const elapsed = stderr.match(/^elapsed\s+([\d.]+)ms\s*$/m);
  if (elapsed) stats.branchcut_internal_elapsed_ms = Number(elapsed[1]);
  return stats;
}

function distribution(samples) {
  const sorted = [...samples].sort((a, b) => a - b);
  const percentile = (p) => {
    const position = (sorted.length - 1) * p;
    const lower = Math.floor(position);
    const upper = Math.ceil(position);
    return sorted[lower] + ((sorted[upper] - sorted[lower]) * (position - lower));
  };
  const mean = sorted.reduce((sum, item) => sum + item, 0) / sorted.length;
  const variance = sorted.reduce((sum, item) => sum + ((item - mean) ** 2), 0) / sorted.length;
  return {
    median_ms: percentile(0.5),
    p90_ms: percentile(0.9),
    p95_ms: percentile(0.95),
    mean_ms: mean,
    standard_deviation_ms: Math.sqrt(variance),
    dispersion_percent: mean === 0 ? 0 : (Math.sqrt(variance) / mean) * 100,
  };
}

function systemInfo() {
  const cpu = os.cpus()[0];
  return {
    os: `${os.type()} ${os.release()} (${os.arch()})`,
    cpu_model: cpu?.model ?? 'unknown',
    logical_cpu_count: os.cpus().length,
    ram_bytes: os.totalmem(),
    node: process.version,
    rustc: run('rustc', ['--version']).stdout.trim(),
  };
}

function gitInfo() {
  const commit = run('git', ['rev-parse', 'HEAD']).stdout.trim();
  const dirty = run('git', ['status', '--porcelain', '--', 'src', 'Cargo.toml', 'Cargo.lock']).stdout.trim().length > 0;
  return { commit, tracked_source_dirty: dirty };
}

function oracleWorkerSource() {
  return `const { createRequire } = require('node:module');\n` +
    `const path = require('node:path');\n` +
    `const req = createRequire(path.join(process.argv[1], 'branchcut-oracle-loader.cjs'));\n` +
    `const fg = req('fast-glob');\n` +
    `fg([${JSON.stringify(GLOBS[0])}, ${JSON.stringify(GLOBS[1])}], {\n` +
    `  cwd: process.argv[2], ignore: ${JSON.stringify(EXCLUDES)}, dot: false,\n` +
    `  onlyFiles: true, unique: true, followSymbolicLinks: false\n` +
    `}).then((paths) => process.stdout.write(String(paths.length) + '\\n'))\n` +
    `.catch((error) => { process.stderr.write(String(error.stack || error)); process.exitCode = 1; });`;
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  const requireOracle = createRequire(pathToFileURL(path.join(options.oracle_dir, 'branchcut-oracle-loader.cjs')));
  const oraclePackage = requireOracle('fast-glob/package.json');
  if (oraclePackage.version !== EXPECTED_FAST_GLOB) {
    throw new Error(`Expected fast-glob@${EXPECTED_FAST_GLOB}, found ${oraclePackage.version}`);
  }
  const fastGlob = requireOracle('fast-glob');
  const corpus = await createCorpus();
  try {
    const oracleOptions = {
      cwd: corpus.root,
      ignore: EXCLUDES,
      dot: false,
      onlyFiles: true,
      unique: true,
      followSymbolicLinks: false,
    };
    const oraclePaths = await fastGlob(GLOBS, oracleOptions);
    const correctnessArgs = [
      '--cwd', corpus.root, '--glob', GLOBS[0], '--glob', GLOBS[1],
      '--exclude', EXCLUDES[0], '--exclude', EXCLUDES[1],
      '--type', 'file', '--sort', '--json',
    ];
    const output = run(options.binary, correctnessArgs).stdout.trimEnd();
    const branchcutPaths = output ? output.split(/\r?\n/).map((line) => JSON.parse(line).path) : [];
    const resultCount = compareResults(branchcutPaths, oraclePaths);
    if (resultCount !== corpus.matched) {
      throw new Error(`Expected ${corpus.matched} source/config matches, got ${resultCount}`);
    }

    const queryArgs = [
      '--cwd', corpus.root, '--glob', GLOBS[0], '--glob', GLOBS[1],
      '--exclude', EXCLUDES[0], '--exclude', EXCLUDES[1], '--type', 'file', '--count', '--stats',
    ];
    const worker = oracleWorkerSource();
    const branchcutSamples = [];
    const oracleSamples = [];
    let finalStats = {};

    for (let iteration = 0; iteration < options.warmups + options.iterations; iteration += 1) {
      const timed = iteration >= options.warmups;
      const branchcutStart = performance.now();
      const branchcutResult = run(options.binary, queryArgs);
      const branchcutElapsed = performance.now() - branchcutStart;
      const count = Number(branchcutResult.stdout.trim());
      if (count !== resultCount) throw new Error(`Branchcut count changed before/after correctness check: ${count} != ${resultCount}`);
      finalStats = parseStats(branchcutResult.stderr);

      const oracleStart = performance.now();
      const oracleResult = run(process.execPath, ['--eval', worker, options.oracle_dir, corpus.root]);
      const oracleElapsed = performance.now() - oracleStart;
      const oracleCount = Number(oracleResult.stdout.trim());
      if (oracleCount !== resultCount) throw new Error(`fast-glob count changed: ${oracleCount} != ${resultCount}`);

      if (timed) {
        branchcutSamples.push(Number(branchcutElapsed.toFixed(3)));
        oracleSamples.push(Number(oracleElapsed.toFixed(3)));
      }
    }

    const metadata = {
      schema_version: 1,
      measurement: 'fresh process per sample; includes startup, query, and count output; filesystem cache state is uncontrolled',
      correctness: { passed: true, result_count: resultCount, comparison: 'complete sorted path sets before timing' },
      git: gitInfo(),
      system: systemInfo(),
      fast_glob: { version: oraclePackage.version },
      branchcut: {
        binary_name: path.basename(options.binary),
        binary_sha256: createHash('sha256').update(await readFile(options.binary)).digest('hex'),
        query: [
          'branchcut', '--cwd', '$FIXTURE', '--glob', GLOBS[0], '--glob', GLOBS[1],
          '--exclude', EXCLUDES[0], '--exclude', EXCLUDES[1], '--type', 'file', '--count', '--stats',
        ],
        stats_from_last_sample: finalStats,
      },
      fast_glob_query: {
        process: 'node --eval <worker>',
        patterns: GLOBS,
        options: { cwd: '$FIXTURE', ignore: EXCLUDES, dot: false, onlyFiles: true, unique: true, followSymbolicLinks: false },
      },
      corpus: {
        shape: '20 packages; per package: src/{250 .rs,250 .toml}, target/debug/150 .rs, node_modules/dep/150 .ts',
        total_files: corpus.total,
        expected_matches: corpus.matched,
      },
      warmup_iterations: options.warmups,
      measured_iterations: options.iterations,
      output_mode: 'count only for both tools; all matching paths enumerated; no path serialization during timing',
      results: {
        branchcut: { samples_ms: branchcutSamples, ...distribution(branchcutSamples) },
        fast_glob: { samples_ms: oracleSamples, ...distribution(oracleSamples) },
      },
    };

    const json = `${JSON.stringify(metadata, null, 2)}\n`;
    if (options.output) {
      const destination = path.resolve(options.output);
      await mkdir(path.dirname(destination), { recursive: true });
      await writeFile(destination, json);
      process.stdout.write(`Raw benchmark record: ${destination}\n`);
    } else {
      process.stdout.write(json);
    }
  } finally {
    const resolved = path.resolve(corpus.root);
    if (!resolved.startsWith(path.resolve(os.tmpdir()) + path.sep)) {
      throw new Error(`Refusing to remove corpus outside temporary directory: ${resolved}`);
    }
    await rm(resolved, { recursive: true, force: true });
  }
}

main().catch((error) => {
  process.stderr.write(`benchmark: ${error.stack ?? error}\n`);
  process.exitCode = 1;
});
