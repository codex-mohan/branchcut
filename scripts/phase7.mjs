#!/usr/bin/env node
// Development-only before/after runner. No third-party Node packages.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { spawn, spawnSync } from 'node:child_process';
import { mkdtemp, mkdir, readdir, readFile, rm, statfs, utimes, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { performance } from 'node:perf_hooks';

const hash = (data) => createHash('sha256').update(data).digest('hex');
const run = (binary, args) => {
  const result = spawnSync(binary, args, { encoding: 'utf8', windowsHide: true, maxBuffer: 128 * 1024 * 1024 });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${binary} exited ${result.status}: ${result.stderr}`);
  return result;
};
const options = { iterations: 9, warmups: 2, sizes: '10000,100000' };
for (let i = 2; i < process.argv.length; i++) {
  const name = process.argv[i];
  if (name === '--engine-stream') { options.engine_stream = true; continue; }
  if (name === '--paired-engines') { options.paired_engines = true; continue; }
  if (name === '--help') {
    console.log('node scripts/phase7.mjs --before FILE --after FILE --before-engine FILE --after-engine FILE --before-source FILE --output FILE [--sizes 10000,100000] [--iterations 9] [--warmups 2] [--snapshot FILE] [--only NAME]');
    process.exit(0);
  }
  if (!/^--(before|after|before-engine|after-engine|before-source|output|sizes|iterations|warmups|snapshot|only|only-regex)$/.test(name)) throw new Error(`unknown option ${name}`);
  const value = process.argv[++i];
  if (!value || value.startsWith('--')) throw new Error(`missing value for ${name}`);
  options[name.slice(2)] = value;
}

async function sourceIdentity() {
  const files = ['Cargo.toml', 'Cargo.lock', 'examples/measure_engine.rs'];
  async function visit(directory) {
    for (const entry of (await readdir(directory, { withFileTypes: true })).sort((a, b) => a.name.localeCompare(b.name))) {
      const file = `${directory}/${entry.name}`;
      if (entry.isDirectory()) await visit(file);
      else if (file.endsWith('.rs')) files.push(file);
    }
  }
  await visit('src');
  const manifest = [];
  for (const file of files.sort()) manifest.push({ path: file, sha256: hash(await readFile(file)) });
  return {
    commit: run('git', ['rev-parse', 'HEAD']).stdout.trim(),
    dirty: !!run('git', ['status', '--porcelain']).stdout.trim(),
    source_manifest: manifest,
    source_sha256: hash(JSON.stringify(manifest)),
  };
}
if (options.snapshot) {
  await writeFile(options.snapshot, JSON.stringify(await sourceIdentity(), null, 2) + '\n');
  process.exit(0);
}
for (const key of ['before', 'after', 'before-engine', 'after-engine', 'before-source', 'output']) {
  if (!options[key]) throw new Error(`missing --${key}`);
}
for (const key of ['iterations', 'warmups']) {
  if (!/^\d+$/.test(String(options[key])) || !Number.isSafeInteger(Number(options[key]))) throw new Error(`invalid ${key}`);
  options[key] = Number(options[key]);
}
if (options.iterations < 1) throw new Error('iterations must be positive');
const sizes = options.sizes.split(',').map(Number);
if (sizes.some((n) => !Number.isSafeInteger(n) || n < 160 || n % 160 !== 0 && ![10000, 100000].includes(n))) throw new Error('sizes must be 10000, 100000, or multiples of 160');

function distribution(values) {
  const sorted = [...values].sort((a, b) => a - b);
  const p = (fraction) => sorted[Math.ceil((sorted.length - 1) * fraction)];
  const mean = values.reduce((a, b) => a + b, 0) / values.length;
  const stddev = Math.sqrt(values.reduce((total, value) => total + (value - mean) ** 2, 0) / values.length);
  return { median_ms: p(0.5), p90_ms: p(0.9), p95_ms: p(0.95), standard_deviation_ms: stddev, dispersion_percent: stddev / mean * 100 };
}
function stats(text) {
  const result = {};
  for (const line of text.trim().split(/\r?\n/)) {
    const match = line.match(/^([a-z ]+?)\s+(\d+(?:\.\d+)?)(ms)?$/);
    if (match) result[match[1].trim().replaceAll(' ', '_')] = Number(match[2]);
  }
  return result;
}
async function createCorpus(root, count) {
  const records = [];
  const perPackage = count / 20;
  const branches = ['src', 'src/nested/deep', 'src', 'docs', 'target/debug', 'node_modules/dep', 'src/UPPER', 'src'];
  const extensions = ['rs', 'rs', 'toml', 'md', 'rs', 'ts', 'RS', 'log'];
  for (let p = 0; p < 20; p++) {
    const base = `packages/p${String(p).padStart(2, '0')}`;
    for (const branch of new Set(branches)) await mkdir(path.join(root, base, branch), { recursive: true });
    for (let start = 0; start < perPackage; start += 128) {
      await Promise.all(Array.from({ length: Math.min(128, perPackage - start) }, async (_, offset) => {
        const index = start + offset;
        const file = `${base}/${branches[index % 8]}/f${String(index).padStart(5, '0')}.${extensions[index % 8]}`;
        const size = index % 16 === 0 ? 4096 : 32;
        const old = index % 2 === 1;
        await writeFile(path.join(root, file), Buffer.alloc(size, 65));
        const mtime = new Date(old ? '2020-01-01T00:00:00Z' : '2024-01-01T00:00:00Z');
        await utimes(path.join(root, file), mtime, mtime);
        records.push({ path: file, size, old, index });
      }));
    }
  }
  // Explicit recursive forms keep before/after result sets equivalent even
  // when the baseline predates the slashless-directory-rule regression fix.
  await writeFile(path.join(root, '.gitignore'), '**/target/\n**/node_modules/\n');
  await writeFile(path.join(root, 'packages/p00/src/.gitignore'), '*.log\n');
  records.sort((a, b) => a.path.localeCompare(b.path));
  return records;
}
function workloads(records) {
  const item = (name, args, predicate, extra = {}) => ({ name, args, expected: records.filter(predicate).map((r) => r.path).sort(), ...extra });
  const rs = (r) => r.path.endsWith('.rs');
  const clean = (r) => !r.path.includes('/target/') && !r.path.includes('/node_modules/');
  const list = [
    item('broad-count', ['--glob', '**/*'], () => true),
    item('broad-output', ['--glob', '**/*'], () => true, { output: 'paths' }),
    item('narrow-prefix', ['--glob', 'packages/p00/src/**/*.rs'], (r) => r.path.startsWith('packages/p00/src/') && rs(r)),
    item('insensitive-prefix', ['--glob', 'packages/p00/src/**/*.rs', '--ignore-case'], (r) => r.path.toLowerCase().startsWith('packages/p00/src/') && r.path.toLowerCase().endsWith('.rs')),
    item('exclusions', ['--glob', '**/*.rs', '--exclude', '**/target/**'], (r) => rs(r) && clean(r)),
    item('ignored', ['--glob', '**/*', '--gitignore'], (r) => clean(r) && !(r.path.startsWith('packages/p00/src/') && r.path.endsWith('.log'))),
    item('size-candidates', ['--glob', '**/*.rs', '--size', '>=1KiB'], (r) => rs(r) && r.size >= 1024),
    item('mtime-candidates', ['--glob', '**/*.rs', '--modified-within', '36500d'], rs),
    item('first-10', ['--glob', '**/*.rs', '--limit', '10'], rs, { limit: 10, unordered: true }),
    item('sort-all', ['--glob', '**/*', '--sort'], () => true, { output: 'paths', engine: false }),
    item('sorted-10', ['--glob', '**/*', '--sort', '--limit', '10'], () => true, { output: 'paths', limit: 10, engine: false }),
    item('sorted-half', ['--glob', '**/*', '--sort', '--limit', String(records.length / 2)], () => true, { output: 'paths', limit: records.length / 2, engine: false }),
    item('json-output', ['--glob', '**/*', '--json'], () => true, { output: 'json', engine: false }),
    item('nul-output', ['--glob', '**/*', '--print0'], () => true, { output: 'nul', engine: false }),
  ];
  for (const threads of [2, 4, 8]) list.push(item(`parallel-${threads}`, ['--glob', '**/*', '--threads', String(threads)], () => true));
  list.push(item('parallel-4-count', ['--glob', '**/*', '--threads', '4'], () => true));
  for (const n of [1, 10, 100, 1000]) {
    const shared = Array.from({ length: n }, (_, i) => `**/src/**/f${String(i).padStart(5, '0')}.rs`);
    list.push(item(`shared-${n}`, shared.flatMap((pattern) => ['--glob', pattern]), (r) => r.index < n && r.path.includes('/src/') && rs(r)));
    const unrelated = ['packages/p00/src/**/*.rs', ...Array.from({ length: n - 1 }, (_, i) => `u${String(i).padStart(4, '0')}/**/*.rs`)];
    list.push(item(`unrelated-${n}`, unrelated.flatMap((pattern) => ['--glob', pattern]), (r) => r.path.startsWith('packages/p00/src/') && rs(r)));
  }
  return list;
}
function pathsFrom(stdout, mode) {
  if (mode === 'json') return stdout.trim().split(/\r?\n/).filter(Boolean).map((line) => JSON.parse(line).path);
  return stdout.split(mode === 'nul' ? '\0' : /\r?\n/).filter(Boolean).map((file) => file.replaceAll('\\', '/'));
}
async function timed(binary, args) {
  const started = performance.now();
  const child = spawn(binary, args, { windowsHide: true });
  let firstOutput = null;
  const stdout = [];
  const stderr = [];
  child.stdout.on('data', (data) => { firstOutput ??= performance.now() - started; stdout.push(data); });
  child.stderr.on('data', (data) => stderr.push(data));
  const code = await new Promise((resolve, reject) => { child.once('error', reject); child.once('close', resolve); });
  const elapsed_ms = performance.now() - started;
  const error = Buffer.concat(stderr).toString();
  if (code !== 0) throw new Error(`${binary} exited ${code}: ${error}`);
  return { elapsed_ms, first_output_chunk_ms: firstOutput, stdout: Buffer.concat(stdout).toString(), stats: stats(error) };
}
async function resources(binary, args, root) {
  if (process.platform !== 'win32') return { peak_rss_bytes: null, cpu_ms: null, reason: 'Windows resource probe only; no memory claim on this platform' };
  const request = path.join(root, 'process-request.json');
  await writeFile(request, JSON.stringify({ binary: path.resolve(binary), arguments: args }));
  const result = JSON.parse(run('pwsh', ['-NoProfile', '-NonInteractive', '-File', 'scripts/measure-process.ps1', '-RequestPath', request]).stdout);
  assert.equal(result.exit_code, 0, result.stderr);
  return result;
}

// Keep both compiled queries resident and alternate individual executions.
// The probes wait on stdin between runs; command/JSON I/O is outside their
// internal timers, avoiding the earlier before-block/after-block bias.
async function pairedEngines(variants, query) {
  const probes = {};
  const results = {};
  const count = options.warmups + options.iterations;
  try {
    for (const label of ['before', 'after']) {
      const child = spawn(variants[label].engine, [...query, ...(options.engine_stream ? ['--stream'] : []), '--interactive', '--iterations', String(count), '--warmups', '0'], { windowsHide: true });
      let buffered = '', stderr = '', failure = null, closed = false;
      const queue = [], waiting = [];
      const fail = (error) => { failure = error; while (waiting.length) waiting.shift().reject(error); };
      child.stdout.setEncoding('utf8');
      child.stderr.setEncoding('utf8');
      child.stderr.on('data', (data) => { stderr += data; });
      child.stdout.on('data', (data) => {
        buffered += data;
        let end;
        while ((end = buffered.indexOf('\n')) >= 0) {
          const line = buffered.slice(0, end).trim();
          buffered = buffered.slice(end + 1);
          if (!line) continue;
          try {
            const value = JSON.parse(line);
            if (waiting.length) waiting.shift().resolve(value);
            else queue.push(value);
          } catch (error) { fail(error); child.kill(); }
        }
      });
      child.on('error', fail);
      child.stdin.on('error', fail);
      const completion = new Promise((resolve) => child.on('close', (code) => {
        closed = true;
        if (code !== 0) fail(new Error(`engine probe exited ${code}: ${stderr}`));
        else if (waiting.length) fail(new Error('engine probe ended before a sample'));
        resolve(code);
      }));
      const next = () => queue.length ? Promise.resolve(queue.shift()) : failure ? Promise.reject(failure) : closed ? Promise.reject(new Error('engine probe already ended')) : new Promise((resolve, reject) => waiting.push({ resolve, reject }));
      probes[label] = { child, next, completion };
      results[label] = { compile: await next(), engine_samples: [] };
    }
    for (let iteration = 0; iteration < count; iteration++) {
      for (const label of iteration % 2 ? ['after', 'before'] : ['before', 'after']) {
        const probe = probes[label];
        await new Promise((resolve, reject) => probe.child.stdin.write('run\n', (error) => error ? reject(error) : resolve()));
        const sample = await probe.next();
        if (iteration >= options.warmups) results[label].engine_samples.push(sample);
      }
    }
    for (const probe of Object.values(probes)) assert.equal(await probe.completion, 0);
    return results;
  } finally {
    for (const probe of Object.values(probes)) if (probe.child.exitCode === null) probe.child.kill();
  }
}

async function main() {
  const root = await mkdtemp(path.join(os.tmpdir(), 'branchcut-phase7-'));
  const variants = {};
  for (const label of ['before', 'after']) variants[label] = {
    binary: path.resolve(options[label]),
    engine: path.resolve(options[`${label}-engine`]),
    binary_sha256: hash(await readFile(options[label])),
    engine_sha256: hash(await readFile(options[`${label}-engine`])),
    source: label === 'before' ? JSON.parse(await readFile(options['before-source'], 'utf8')) : await sourceIdentity(),
  };
  // Strip local absolute executable paths from the retained record.
  const record = {
    schema_version: 1, recorded_at: new Date().toISOString(),
    method: 'CLI: fresh process, stdout pipe drained and retained, alternating variant order; engine: compiled plan reused in one process, visitor count or explicit parallel delivery. Filesystem cache uncontrolled; warmups are not cold-disk control.',
    parallel_engine_delivery: options.engine_stream ? 'bounded streaming visitor' : 'collected paths',
    engine_order: options.paired_engines ? 'two resident probes; individual executions alternate order each iteration' : 'separate before and after blocks, order reversed by workload',
    correctness: 'Complete CLI path sets checked against deterministic generator manifest before timing; every timed output/count checked. Engine counts checked against the same manifest.',
    limitations: ['Engine variants run in separate blocks, reversing order between workloads.', 'First output chunk includes buffering and startup; it is not engine time to first match.', 'Windows resources are separate untimed samples; OS peak working set is sampled every 1 ms while alive and may miss the final peak. It is not allocator usage.', 'No external competitor speed claim; generated files are not a public-repository benchmark.', 'Batch child-process work is not timed by this suite.'],
    system: { os: `${os.type()} ${os.release()} ${os.arch()}`, cpu: os.cpus()[0]?.model, logical_cpus: os.cpus().length, ram_bytes: os.totalmem(), node: process.version, rustc: run('rustc', ['--version']).stdout.trim(), filesystem_type: Number((await statfs(root)).type) },
    harness_sha256: hash(await readFile('scripts/phase7.mjs')),
    resource_probe_sha256: hash(await readFile('scripts/measure-process.ps1')),
    variants: Object.fromEntries(Object.entries(variants).map(([name, { binary, engine, ...identity }]) => [name, identity])),
    iterations: options.iterations, warmups: options.warmups, workloads: [],
  };
  const save = async () => { await mkdir(path.dirname(path.resolve(options.output)), { recursive: true }); await writeFile(options.output, JSON.stringify(record, null, 2) + '\n'); };
  try {
    for (const size of sizes) {
      const corpus = path.join(root, String(size));
      await mkdir(corpus);
      console.log(`Generating ${size} files`);
      const files = await createCorpus(corpus, size);
      const cases = workloads(files).filter((item) => (!options.only || item.name.includes(options.only)) && (!options['only-regex'] || new RegExp(options['only-regex']).test(item.name)));
      for (let index = 0; index < cases.length; index++) {
        const test = cases[index];
        const expected = test.limit ? test.expected.slice(0, test.limit) : test.expected;
        const query = ['--cwd', corpus, ...test.args];
        const check = (actual) => {
          assert.equal(new Set(actual).size, actual.length, `${test.name}: duplicate paths`);
          if (test.unordered) {
            assert.equal(actual.length, expected.length);
            const allowed = new Set(test.expected);
            assert.ok(actual.every((file) => allowed.has(file)), 'limited selection contains invalid path');
          } else assert.deepEqual([...actual].sort(), expected, test.name);
        };
        for (const variant of Object.values(variants)) {
          check(pathsFrom(run(variant.binary, test.output ? query : [...query, '--json']).stdout, test.output || 'json'));
        }
        const result = { name: test.name, corpus_files: size, query: test.args, expected_matches: expected.length, complete_selection_sha256: hash(JSON.stringify(test.expected)), variants: {} };
        for (const label of ['before', 'after']) result.variants[label] = { cli_samples: [] };
        const args = [...query, ...(test.output ? [] : ['--count']), '--stats'];
        for (let iteration = 0; iteration < options.iterations + options.warmups; iteration++) {
          for (const label of iteration % 2 ? ['after', 'before'] : ['before', 'after']) {
            const sample = await timed(variants[label].binary, args);
            if (test.output) check(pathsFrom(sample.stdout, test.output));
            else assert.equal(Number(sample.stdout.trim()), expected.length, test.name);
            assert.equal(sample.stats.filesystem_errors, 0);
            delete sample.stdout;
            if (iteration >= options.warmups) result.variants[label].cli_samples.push(sample);
          }
        }
        for (const label of index % 2 ? ['after', 'before'] : ['before', 'after']) {
          const variant = result.variants[label];
          variant.cli_distribution = distribution(variant.cli_samples.map((sample) => sample.elapsed_ms));
          if (test.engine !== false && !options.paired_engines) {
            const output = run(variants[label].engine, [...query, ...(options.engine_stream ? ['--stream'] : []), '--iterations', String(options.iterations), '--warmups', String(options.warmups)]).stdout.trim().split(/\r?\n/).map(JSON.parse);
            variant.compile = output.shift();
            for (const sample of output) assert.equal(sample.matches, expected.length, `engine ${test.name}`);
            variant.engine_samples = output;
            variant.engine_distribution = distribution(output.map((sample) => sample.elapsed_ns / 1e6));
          }
          if (['sort-all', 'sorted-10', 'sorted-half', 'broad-count', 'parallel-4'].includes(test.name)) {
            variant.resources = [];
            for (let sample = 0; sample < 3; sample++) variant.resources.push(await resources(variants[label].binary, args, root));
          }
        }
        if (test.engine !== false && options.paired_engines) {
          const paired = await pairedEngines(variants, query);
          for (const label of ['before', 'after']) {
            for (const sample of paired[label].engine_samples) assert.equal(sample.matches, expected.length, `paired engine ${test.name}`);
            Object.assign(result.variants[label], paired[label]);
            result.variants[label].engine_distribution = distribution(paired[label].engine_samples.map((sample) => sample.elapsed_ns / 1e6));
          }
        }
        const ratio = result.variants.before.cli_distribution.median_ms / result.variants.after.cli_distribution.median_ms;
        console.log(`${size} ${test.name}: ${(ratio).toFixed(2)}x before/after; ${expected.length} matches`);
        record.workloads.push(result);
        await save();
      }
    }
  } finally {
    const relative = path.relative(path.resolve(os.tmpdir()), path.resolve(root));
    if (!relative || relative.startsWith('..') || path.isAbsolute(relative) || !path.basename(root).startsWith('branchcut-phase7-')) throw new Error('unsafe cleanup path');
    await rm(root, { recursive: true, force: true });
  }
}
main().catch((error) => { console.error(error); process.exitCode = 1; });
