#!/usr/bin/env node
// Official-CLI comparison. Lower-level walker probes are intentionally excluded.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { createRequire } from 'node:module';
import os from 'node:os';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { access, mkdtemp, mkdir, readFile, readdir, rm, utimes, writeFile } from 'node:fs/promises';
import { performance } from 'node:perf_hooks';

const hash = (data) => createHash('sha256').update(data).digest('hex');
const NEW_TIME = new Date('2024-01-01T00:00:00Z');

function usage() {
  return 'node scripts/compare-cli.mjs --binary PATH --zlob PATH --oracle-dir PATH --output PATH [--sizes 100000] [--warmups 2] [--iterations 7] [--memory-samples 0]\n';
}

function parseArgs(argv) {
  const options = { sizes: [100_000], warmups: 2, iterations: 7, memory_samples: 0 };
  for (let index = 0; index < argv.length; index++) {
    const option = argv[index];
    if (option === '--help' || option === '-h') {
      process.stdout.write(usage());
      process.exit(0);
    }
    if (!['--binary', '--zlob', '--oracle-dir', '--output', '--sizes', '--warmups', '--iterations', '--memory-samples'].includes(option)) {
      throw new Error(`unknown option ${option}\n${usage()}`);
    }
    const value = argv[++index];
    if (!value || value.startsWith('--')) throw new Error(`missing value for ${option}`);
    const key = option.slice(2).replaceAll('-', '_');
    if (key === 'sizes') options.sizes = value.split(',').map(Number);
    else if (key === 'warmups' || key === 'iterations' || key === 'memory_samples') options[key] = Number(value);
    else options[key] = path.resolve(value);
  }
  for (const key of ['binary', 'zlob', 'oracle_dir', 'output']) {
    if (!options[key]) throw new Error(`missing --${key.replaceAll('_', '-')}`);
  }
  if (!Number.isSafeInteger(options.iterations) || options.iterations < 1) throw new Error('iterations must be positive');
  if (!Number.isSafeInteger(options.warmups) || options.warmups < 0) throw new Error('warmups cannot be negative');
  if (!Number.isSafeInteger(options.memory_samples) || options.memory_samples < 0) throw new Error('memory-samples cannot be negative');
  if (options.memory_samples && process.platform !== 'win32') throw new Error('memory-samples currently requires Windows peak working-set measurement');
  if (!options.sizes.length || options.sizes.some((size) => !Number.isSafeInteger(size) || size < 160 || size % 20 !== 0)) {
    throw new Error('sizes must be positive multiples of 20');
  }
  return options;
}

function run(program, args, extra = {}) {
  const result = spawnSync(program, args, {
    encoding: 'utf8',
    windowsHide: true,
    maxBuffer: 256 * 1024 * 1024,
    ...extra,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${program} ${args.join(' ')} exited ${result.status}:\n${result.stderr}`);
  return result;
}

function normalize(values) {
  return values.map(normalizeOne).sort();
}

function normalizeOne(value) {
  return value.replaceAll('\\', '/').replace(/^\.\//, '').replace(/\/+$/, '');
}

function readPaths(stdout, outputMode = 'paths') {
  return stdout
    .replace(/\r/g, '')
    .split(outputMode === 'nul' ? '\0' : '\n')
    .filter(Boolean);
}

function percentile(values, fraction) {
  const sorted = [...values].sort((left, right) => left - right);
  return sorted[Math.ceil((sorted.length - 1) * fraction)];
}

function distribution(samples) {
  const values = samples.map((sample) => sample.elapsed_ms);
  const mean = values.reduce((total, value) => total + value, 0) / values.length;
  const deviation = Math.sqrt(values.reduce((total, value) => total + (value - mean) ** 2, 0) / values.length);
  return {
    median_ms: percentile(values, 0.5),
    p90_ms: percentile(values, 0.9),
    p95_ms: percentile(values, 0.95),
    standard_deviation_ms: deviation,
    dispersion_percent: mean === 0 ? 0 : (deviation / mean) * 100,
  };
}

function parentDirs(relative) {
  const parts = relative.split('/');
  const dirs = [];
  let current = '';
  for (const part of parts.slice(0, -1)) {
    current = current ? `${current}/${part}` : part;
    dirs.push(current);
  }
  return dirs;
}

function hidden(relative) {
  return relative.split('/').some((part) => part.startsWith('.'));
}

async function addFile(root, records, dirs, relative, contents, hiddenEntry = hidden(relative)) {
  const destination = path.join(root, relative);
  await mkdir(path.dirname(destination), { recursive: true });
  for (const directory of parentDirs(relative)) dirs.add(directory);
  await writeFile(destination, contents);
  await utimes(destination, NEW_TIME, NEW_TIME);
  records.push({ path: relative, kind: 'file', hidden: hiddenEntry });
}

async function makeCorpus(root, totalFiles) {
  const records = [];
  const dirs = new Set(['packages']);
  const packages = 20;
  const perPackage = totalFiles / packages;
  const locations = ['src', 'src/nested/deep', 'src', 'docs', 'target/debug', 'node_modules/dep', 'src/upper', 'src'];
  const extensions = ['rs', 'rs', 'toml', 'md', 'rs', 'ts', 'txt', 'log'];

  for (let packageIndex = 0; packageIndex < packages; packageIndex++) {
    const packageName = `p${String(packageIndex).padStart(2, '0')}`;
    const base = `packages/${packageName}`;
    dirs.add(base);
    for (const location of new Set(locations)) {
      const directory = `${base}/${location}`;
      await mkdir(path.join(root, directory), { recursive: true });
      for (const parent of parentDirs(`${directory}/x`)) dirs.add(parent);
    }
    for (let start = 0; start < perPackage; start += 128) {
      await Promise.all(Array.from({ length: Math.min(128, perPackage - start) }, async (_, offset) => {
        const index = start + offset;
        const relative = `${base}/${locations[index % locations.length]}/f${String(index).padStart(5, '0')}.${extensions[index % extensions.length]}`;
        const destination = path.join(root, relative);
        await writeFile(destination, Buffer.alloc(32, 65));
        await utimes(destination, NEW_TIME, NEW_TIME);
        records.push({ path: relative, kind: 'file', hidden: false });
      }));
    }
  }

  await addFile(root, records, dirs, 'ignored/drop.rs', Buffer.alloc(32, 65));
  await addFile(root, records, dirs, 'packages/p00/src/.hidden.rs', Buffer.alloc(32, 65), true);
  await addFile(root, records, dirs, 'packages/p01/.hidden/secret.rs', Buffer.alloc(32, 65), true);
  await addFile(
    root,
    records,
    dirs,
    '.gitignore',
    Buffer.from('ignored/\ntarget/\nnode_modules/\npackages/p06/docs/generated.md\n'),
    true,
  );
  await addFile(root, records, dirs, 'packages/p06/docs/generated.md', Buffer.alloc(32, 65));

  records.sort((left, right) => left.path.localeCompare(right.path));
  return {
    records,
    dirs: [...dirs].filter((directory) => !hidden(directory)).sort(),
  };
}

function paths(records) {
  return records.map((record) => record.path).sort();
}

function makeCases(corpus) {
  const files = corpus.records.filter((record) => record.kind === 'file' && !record.hidden);
  const rust = files.filter((record) => record.path.endsWith('.rs'));
  const hiddenRust = corpus.records.filter((record) => record.kind === 'file' && record.path.endsWith('.rs'));
  const gitignoreRust = rust.filter((record) =>
    !record.path.startsWith('ignored/') &&
    !record.path.includes('/target/') &&
    !record.path.includes('/node_modules/'));
  const cleanRustToml = files.filter((record) =>
    /\.(rs|toml)$/.test(record.path) &&
    !record.path.includes('/target/') &&
    !record.path.includes('/node_modules/'));

  return [
    {
      name: 'broad-files',
      patterns: ['**/*'],
      excludes: [],
      expected: paths(files),
      branchcut: (root) => ['--cwd', root, '--glob', '**/*'],
      rg: () => ['--files', '--no-ignore', '--no-hidden', '--glob', '**/*', '--glob', '!**/.*', '--glob', '!.*', '.'],
      fd: () => ['--type', 'file', '--no-ignore', '--no-hidden', '--glob', '*'],
      zlob: (root) => ['-a', '-G', '**/*', root],
      fastglob: { patterns: ['**/*'], options: { onlyFiles: true } },
      matrix: true,
    },
    {
      name: 'fixed-prefix-rust',
      patterns: ['packages/p00/src/**/*.rs'],
      excludes: [],
      expected: paths(rust.filter((record) => record.path.startsWith('packages/p00/src/'))),
      branchcut: (root) => ['--cwd', root, '--glob', 'packages/p00/src/**/*.rs'],
      rg: () => ['--files', '--no-ignore', '--no-hidden', '--glob', 'packages/p00/src/**/*.rs', '--glob', '!**/.*', '--glob', '!.*', '.'],
      fd: () => ['--type', 'file', '--no-ignore', '--no-hidden', '--glob', '*.rs', 'packages/p00/src'],
      zlob: (root) => ['-a', '-G', 'packages/p00/src/**/*.rs', root],
      fastglob: { patterns: ['packages/p00/src/**/*.rs'], options: { onlyFiles: true } },
      matrix: true,
    },
    {
      name: 'rust-toml-exclusions',
      patterns: ['**/*.{rs,toml}'],
      excludes: ['**/target/**', '**/node_modules/**'],
      expected: paths(cleanRustToml),
      branchcut: (root) => ['--cwd', root, '--glob', '**/*.{rs,toml}', '--exclude', '**/target/**', '--exclude', '**/node_modules/**'],
      rg: () => ['--files', '--no-ignore', '--no-hidden', '--glob', '**/*.rs', '--glob', '**/*.toml', '--glob', '!**/target/**', '--glob', '!**/node_modules/**', '--glob', '!**/.*', '--glob', '!.*', '.'],
      fd: () => ['--type', 'file', '--no-ignore', '--no-hidden', '--extension', 'rs', '--extension', 'toml', '--exclude', 'target', '--exclude', 'node_modules'],
      zlob: null,
      fastglob: { patterns: ['**/*.{rs,toml}'], ignore: ['**/target/**', '**/node_modules/**'], options: { onlyFiles: true } },
      matrix: true,
    },
    {
      name: 'hidden-rust',
      patterns: ['**/*.rs'],
      excludes: [],
      expected: paths(hiddenRust),
      branchcut: (root) => ['--cwd', root, '--glob', '**/*.rs', '--hidden'],
      rg: () => ['--files', '--no-ignore', '--hidden', '--glob', '**/*.rs', '.'],
      fd: () => ['--type', 'file', '--no-ignore', '--hidden', '--glob', '*.rs'],
      zlob: (root) => ['-a', '-G', '-H', '**/*.rs', root],
      fastglob: { patterns: ['**/*.rs'], options: { onlyFiles: true, dot: true } },
      matrix: false,
    },
    {
      name: 'gitignore-rust',
      patterns: ['**/*.rs'],
      excludes: [],
      expected: paths(gitignoreRust),
      branchcut: (root) => ['--cwd', root, '--glob', '**/*.rs', '--gitignore'],
      rg: () => ['--files', '--no-require-git', '--no-hidden', '--glob', '**/*.rs', '--glob', '!**/.*', '--glob', '!.*', '.'],
      fd: () => ['--type', 'file', '--no-hidden', '--no-require-git', '--glob', '*.rs'],
      zlob: (root) => ['-a', '**/*.rs', root],
      fastglob: null,
      matrix: false,
    },
    {
      name: 'directories',
      patterns: ['**/*'],
      excludes: [],
      expected: corpus.dirs,
      branchcut: (root) => ['--cwd', root, '--glob', '**/*', '--type', 'dir'],
      rg: null,
      fd: () => ['--type', 'directory', '--no-ignore', '--no-hidden', '--glob', '*'],
      zlob: (root) => ['-a', '-G', '-d', '**/*', root],
      fastglob: { patterns: ['**/*'], options: { onlyDirectories: true } },
      matrix: false,
    },
    {
      name: 'sorted-rust',
      patterns: ['**/*.rs'],
      excludes: [],
      expected: paths(rust),
      branchcut: (root) => ['--cwd', root, '--glob', '**/*.rs', '--sort'],
      rg: () => ['--files', '--no-ignore', '--no-hidden', '--sort', 'path', '--glob', '**/*.rs', '--glob', '!**/.*', '--glob', '!.*', '.'],
      fd: null,
      zlob: (root) => ['-a', '-G', '-s', '**/*.rs', root],
      fastglob: null,
      matrix: false,
    },
    {
      name: 'limit-10-rust',
      patterns: ['**/*.rs'],
      excludes: [],
      expected: paths(rust),
      limit: 10,
      unordered: true,
      branchcut: (root) => ['--cwd', root, '--glob', '**/*.rs', '--limit', '10'],
      rg: null,
      fd: () => ['--type', 'file', '--no-ignore', '--no-hidden', '--max-results', '10', '--glob', '*.rs'],
      zlob: (root) => ['-n', '10', '-G', '**/*.rs', root],
      fastglob: null,
      matrix: false,
    },
    {
      name: 'broad-files-nul',
      patterns: ['**/*'],
      excludes: [],
      expected: paths(files),
      outputMode: 'nul',
      branchcut: (root) => ['--cwd', root, '--glob', '**/*', '--print0'],
      rg: () => ['--files', '--no-ignore', '--no-hidden', '--null', '--glob', '**/*', '--glob', '!**/.*', '--glob', '!.*', '.'],
      fd: () => ['--type', 'file', '--no-ignore', '--no-hidden', '--print0', '--glob', '*'],
      zlob: null,
      fastglob: null,
      matrix: false,
    },
    {
      name: 'recursive-basename-rust',
      patterns: ['*.rs'],
      excludes: [],
      expected: paths(rust),
      branchcut: (root) => ['--cwd', root, '--glob', '**/*.rs'],
      rg: () => ['--files', '--no-ignore', '--no-hidden', '--glob', '*.rs', '--glob', '!**/.*', '--glob', '!.*', '.'],
      fd: () => ['--type', 'file', '--no-ignore', '--no-hidden', '--glob', '*.rs'],
      zlob: (root) => ['-a', '-G', '-b', '*.rs', root],
      fastglob: { patterns: ['**/*.rs'], options: { onlyFiles: true } },
      matrix: false,
    },
  ];
}

function nodeWorker() {
  return `const path = require('node:path');
const { createRequire } = require('node:module');
const req = createRequire(path.join(process.argv[1], 'branchcut-oracle-loader.cjs'));
const spec = JSON.parse(process.argv[2]);
const loaded = req(spec.library);
const glob = spec.library === 'fast-glob' ? loaded : loaded.glob;
glob(spec.patterns, spec.options)
  .then((paths) => process.stdout.write(paths.join('\\n') + (paths.length ? '\\n' : '')))
  .catch((error) => { process.stderr.write(String(error.stack || error)); process.exitCode = 1; });`;
}

function commandSet(test, fixture, tools, oracleDir) {
  const command = (program, args, extra = {}) => Object.assign(
    () => run(program, args, extra),
    { invocation: { binary: program, arguments: args, cwd: extra.cwd ?? process.cwd() } },
  );
  const fastOptions = {
    cwd: fixture,
    dot: Boolean(test.fastglob?.options?.dot),
    onlyFiles: test.fastglob?.options?.onlyFiles ?? true,
    onlyDirectories: Boolean(test.fastglob?.options?.onlyDirectories),
    unique: true,
    followSymbolicLinks: false,
    ignore: test.fastglob?.ignore ?? [],
  };
  const nodeCommand = (library) => command(process.execPath, ['--eval', nodeWorker(), oracleDir, JSON.stringify({
    library,
    patterns: test.fastglob.patterns,
    options: fastOptions,
  })]);
  const commands = {
    branchcut: command(tools.branchcut, test.branchcut(fixture)),
    rg: test.rg ? command(tools.rg, test.rg(), { cwd: fixture }) : null,
    fd: test.fd ? command(tools.fd, test.fd(), { cwd: fixture }) : null,
    zlob: test.zlob
      ? command(
            tools.zlob,
            test.zlob(fixture).map((value) => (value === fixture ? '.' : value)),
            { cwd: fixture },
          )
      : null,
    fast_glob: test.fastglob ? nodeCommand('fast-glob') : null,
    tinyglobby: test.fastglob ? nodeCommand('tinyglobby') : null,
  };
  if (!test.limit || test.unordered) {
    commands.branchcut_4 = command(tools.branchcut, [...test.branchcut(fixture), '--threads', '4']);
    commands.branchcut_auto = command(tools.branchcut, [...test.branchcut(fixture), '--threads', '0']);
  }
  if (test.matrix) {
    commands.rg_1 = command(tools.rg, [...test.rg(), '--threads', '1'], { cwd: fixture });
    commands.rg_4 = command(tools.rg, [...test.rg(), '--threads', '4'], { cwd: fixture });
    commands.fd_1 = command(tools.fd, [...test.fd(), '--threads', '1'], { cwd: fixture });
    commands.fd_4 = command(tools.fd, [...test.fd(), '--threads', '4'], { cwd: fixture });
  }
  return Object.fromEntries(Object.entries(commands).filter(([, command]) => command));
}

function describeCommands(test, fixture) {
  const redact = (args) => args.map((value) => (value === fixture ? '$FIXTURE' : value));
  const descriptions = {
    branchcut: { cwd: '$FIXTURE', argv: ['branchcut', ...redact(test.branchcut(fixture))] },
  };
  if (!test.limit || test.unordered) {
    descriptions.branchcut_4 = {
      cwd: '$FIXTURE',
      argv: ['branchcut', ...redact(test.branchcut(fixture)), '--threads', '4'],
    };
    descriptions.branchcut_auto = {
      cwd: '$FIXTURE',
      argv: ['branchcut', ...redact(test.branchcut(fixture)), '--threads', '0'],
    };
  }
  if (test.rg) descriptions.rg = { cwd: '$FIXTURE', argv: ['rg', ...test.rg()] };
  if (test.fd) descriptions.fd = { cwd: '$FIXTURE', argv: ['fd', ...test.fd()] };
  if (test.zlob) {
    descriptions.zlob = {
      cwd: '$FIXTURE',
      argv: ['zlob', ...test.zlob(fixture).map((value) => (value === fixture ? '.' : value))],
    };
  }
  if (test.fastglob) {
    const options = {
      cwd: '$FIXTURE',
      dot: Boolean(test.fastglob.options?.dot),
      onlyFiles: test.fastglob.options?.onlyFiles ?? true,
      onlyDirectories: Boolean(test.fastglob.options?.onlyDirectories),
      unique: true,
      followSymbolicLinks: false,
      ignore: test.fastglob.ignore ?? [],
    };
    descriptions.fast_glob = { api: 'fast-glob', patterns: test.fastglob.patterns, options };
    descriptions.tinyglobby = { api: 'tinyglobby', patterns: test.fastglob.patterns, options };
  }
  if (test.matrix) {
    descriptions.rg_1 = { cwd: '$FIXTURE', argv: ['rg', ...test.rg(), '--threads', '1'] };
    descriptions.rg_4 = { cwd: '$FIXTURE', argv: ['rg', ...test.rg(), '--threads', '4'] };
    descriptions.fd_1 = { cwd: '$FIXTURE', argv: ['fd', ...test.fd(), '--threads', '1'] };
    descriptions.fd_4 = { cwd: '$FIXTURE', argv: ['fd', ...test.fd(), '--threads', '4'] };
  }
  return descriptions;
}

function validate(name, expected, actual, test) {
  const normalized = normalize(actual);
  assert.equal(new Set(normalized).size, normalized.length, `${name} returned duplicate paths`);
  if (test.limit && test.unordered) {
    assert.equal(normalized.length, test.limit, `${name} did not return ${test.limit} paths`);
    const allowed = new Set(normalize(expected));
    assert.ok(normalized.every((entry) => allowed.has(entry)), `${name} returned an invalid limited path`);
  } else {
    assert.deepEqual(normalized, normalize(expected), `${name} returned the wrong path set`);
  }
}

async function sourceIdentity() {
  const files = ['Cargo.toml', 'Cargo.lock', 'scripts/compare-cli.mjs', 'scripts/measure-process.ps1'];
  async function visit(directory) {
    for (const entry of (await readdir(directory, { withFileTypes: true })).sort((left, right) => left.name.localeCompare(right.name))) {
      const child = path.join(directory, entry.name);
      if (entry.isDirectory()) await visit(child);
      else if (child.endsWith('.rs')) files.push(child);
    }
  }
  await visit('src');
  const manifest = [];
  for (const file of files.sort()) manifest.push({ file: file.replaceAll('\\', '/'), sha256: hash(await readFile(file)) });
  return {
    commit: run('git', ['rev-parse', 'HEAD']).stdout.trim(),
    dirty: Boolean(run('git', ['status', '--porcelain']).stdout.trim()),
    source_manifest: manifest,
    source_manifest_sha256: hash(JSON.stringify(manifest)),
  };
}

async function executablePath(name) {
  for (const directory of (process.env.PATH || '').split(path.delimiter)) {
    const candidate = path.join(directory, name + (process.platform === 'win32' ? '.exe' : ''));
    try {
      await access(candidate);
      return candidate;
    } catch {
      // Try the next PATH entry.
    }
  }
  throw new Error(`cannot resolve ${name}`);
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  const oracleRequire = createRequire(pathToFileURL(path.join(options.oracle_dir, 'branchcut-oracle-loader.cjs')));
  const fastGlobPackage = oracleRequire('fast-glob/package.json');
  const tinyGlobbyPackage = JSON.parse(await readFile(path.join(options.oracle_dir, 'node_modules/tinyglobby/package.json'), 'utf8'));
  if (fastGlobPackage.version !== '3.3.3') throw new Error(`expected fast-glob 3.3.3, found ${fastGlobPackage.version}`);
  if (tinyGlobbyPackage.version !== '0.2.14') throw new Error(`expected tinyglobby 0.2.14, found ${tinyGlobbyPackage.version}`);
  const tools = {
    branchcut: options.binary,
    zlob: options.zlob,
    rg: await executablePath('rg'),
    fd: await executablePath('fd'),
  };
  const memoryTool = options.memory_samples ? await executablePath('pwsh') : null;
  const record = {
    schema_version: 1,
    recorded_at: new Date().toISOString(),
    scope: 'Official CLI/API comparison. No lower-level zlob walker adapter is used.',
    method: 'Fresh process per sample; every tool searches the same fixture and serializes matching paths. Filesystem cache state is uncontrolled. Combinations without an implemented equivalent invocation in this runner are not measured; this is not a feature-absence claim. ripgrep is measured only in --files mode, not content search.',
    correctness: 'Complete normalized path sets are checked against a deterministic fixture manifest before timing and after every sample. Limited selections are verified for exact count, membership, and uniqueness.',
    system: {
      os: `${os.type()} ${os.release()} ${os.arch()}`,
      cpu: os.cpus()[0]?.model,
      logical_cpus: os.cpus().length,
      ram_bytes: os.totalmem(),
      node: process.version,
      rustc: run('rustc', ['--version']).stdout.trim(),
    },
    tools: {
      branchcut: run(tools.branchcut, ['--version']).stdout.trim(),
      zlob: run(tools.zlob, ['--version']).stdout.trim(),
      rg: run(tools.rg, ['--version']).stdout.trim().split(/\r?\n/)[0],
      fd: run(tools.fd, ['--version']).stdout.trim().split(/\r?\n/)[0],
      fast_glob: fastGlobPackage.version,
      tinyglobby: tinyGlobbyPackage.version,
    },
    executable_sha256: Object.fromEntries(await Promise.all(Object.entries(tools).map(async ([name, executable]) => [name, hash(await readFile(executable))]))),
    source: await sourceIdentity(),
    sizes: options.sizes,
    warmups: options.warmups,
    iterations: options.iterations,
    memory_samples_per_broad_configuration: options.memory_samples,
    memory_method: options.memory_samples ? 'Separate fresh-process runs of the same invocations. Windows peak working set sampled every 1 ms; captures are result-checked. Includes Node runtime for JavaScript APIs; excludes the PowerShell measurement parent. Not Unix peak RSS and not used for wall-clock ranking.' : null,
    raw_workloads: [],
  };
  const root = await mkdtemp(path.join(os.tmpdir(), 'branchcut-cli-compare-'));
  const save = async () => {
    await mkdir(path.dirname(options.output), { recursive: true });
    await writeFile(options.output, JSON.stringify(record, null, 2) + '\n');
  };

  try {
    for (const size of options.sizes) {
      const fixture = path.join(root, String(size));
      await mkdir(fixture);
      process.stdout.write(`Generating ${size} files\n`);
      const corpus = await makeCorpus(fixture, size);
      for (const test of makeCases(corpus)) {
        const commands = commandSet(test, fixture, tools, options.oracle_dir);
        for (const [name, command] of Object.entries(commands)) {
          validate(`${name}/${size}/${test.name}`, test.expected, readPaths(command().stdout, test.outputMode), test);
        }
        const workload = {
          name: test.name,
          corpus_files: size,
          expected_matches: test.limit ?? test.expected.length,
          output_mode: test.outputMode ?? 'paths',
          queries: describeCommands(test, fixture),
          not_covered: {
            zlob: test.zlob ? null : 'equivalent invocation not implemented in this runner',
            rg: test.rg ? null : 'equivalent file-list invocation not implemented in this runner',
            fd: test.fd ? null : 'equivalent invocation not implemented in this runner',
            fast_glob: test.fastglob ? null : 'equivalent package invocation not implemented in this runner',
            tinyglobby: test.fastglob ? null : 'equivalent package invocation not implemented in this runner',
          },
          raw_samples: Object.fromEntries(Object.keys(commands).map((name) => [name, []])),
        };
        const names = Object.keys(commands);
        for (let sample = 0; sample < options.warmups + options.iterations; sample++) {
          const rotated = [...names.slice(sample % names.length), ...names.slice(0, sample % names.length)];
          for (const name of rotated) {
            const started = performance.now();
            const output = commands[name]();
            const elapsed_ms = performance.now() - started;
            validate(`${name}/${size}/${test.name}/sample-${sample}`, test.expected, readPaths(output.stdout, test.outputMode), test);
            if (sample >= options.warmups) {
              workload.raw_samples[name].push({ elapsed_ms, stdout_bytes: Buffer.byteLength(output.stdout) });
            }
          }
        }
        workload.summary = Object.fromEntries(Object.entries(workload.raw_samples).map(([name, samples]) => [name, distribution(samples)]));
        if (memoryTool && test.name === 'broad-files') {
          workload.memory_samples = {};
          const measurementRoot = path.join(root, 'memory');
          await mkdir(measurementRoot, { recursive: true });
          for (const [name, command] of Object.entries(commands)) {
            const samples = [];
            for (let index = 0; index < options.memory_samples; index++) {
              const requestPath = path.join(measurementRoot, 'request.json');
              const stdoutPath = path.join(measurementRoot, 'stdout.txt');
              await writeFile(requestPath, JSON.stringify({ ...command.invocation, stdout_path: stdoutPath }));
              const measured = JSON.parse(run(memoryTool, ['-NoProfile', '-File', path.resolve('scripts/measure-process.ps1'), '-RequestPath', requestPath]).stdout);
              assert.equal(measured.exit_code, 0, `${name} failed during memory measurement: ${measured.stderr}`);
              const stdout = await readFile(stdoutPath, 'utf8');
              validate(`${name}/${size}/${test.name}/memory-${index}`, test.expected, readPaths(stdout, test.outputMode), test);
              assert.ok(measured.observed_peak_working_set_bytes > 0, `${name} peak memory was not observed`);
              samples.push(measured);
            }
            workload.memory_samples[name] = samples;
          }
          workload.memory_summary = Object.fromEntries(Object.entries(workload.memory_samples).map(([name, samples]) => [name, {
            median_peak_working_set_bytes: percentile(samples.map((sample) => sample.observed_peak_working_set_bytes), 0.5),
            min_peak_working_set_bytes: Math.min(...samples.map((sample) => sample.observed_peak_working_set_bytes)),
            max_peak_working_set_bytes: Math.max(...samples.map((sample) => sample.observed_peak_working_set_bytes)),
          }]));
        }
        record.raw_workloads.push(workload);
        await save();
        process.stdout.write(`${size} ${test.name}: ${Object.entries(workload.summary).map(([name, value]) => `${name}=${value.median_ms.toFixed(2)}ms`).join(' | ')}\n`);
      }
    }
    record.status = 'complete';
    await save();
  } catch (error) {
    record.status = 'failed';
    record.error = String(error.stack ?? error);
    await save();
    throw error;
  } finally {
    const temporary = path.resolve(os.tmpdir());
    const candidate = path.resolve(root);
    const relative = path.relative(temporary, candidate);
    if (!relative || relative.startsWith('..') || path.isAbsolute(relative) || !path.basename(candidate).startsWith('branchcut-cli-compare-')) {
      throw new Error(`unsafe cleanup path: ${candidate}`);
    }
    await rm(candidate, { recursive: true, force: true });
  }
}

main().catch((error) => {
  process.stderr.write(`cli comparison: ${error.stack ?? error}\n`);
  process.exitCode = 1;
});
