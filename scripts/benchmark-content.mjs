import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { performance } from 'node:perf_hooks';

const [baselinePath, branchcutPath, reportPath, filesystem, baselineSyntax = 'legacy'] = process.argv.slice(2);
if (!filesystem || !['legacy', 'grep'].includes(baselineSyntax)) {
  throw new Error('Usage: node scripts/benchmark-content.mjs BEFORE BINARY REPORT.json FILESYSTEM [legacy|grep]');
}
const before = path.resolve(baselinePath);
const branchcut = path.resolve(branchcutPath);
const fixture = path.resolve('target/content-benchmark-fixture');
const line = '0123456789abcdef'.repeat(7) + 'abcdefghijklmno\n';
const hitLine = line.slice(0, 12) + 'ERROR_BRANCHCUT' + line.slice(27);
const file = line.repeat(8192);
for (let group = 0; group < 12; group++) {
  const dir = path.join(fixture, `shard-${group}`);
  mkdirSync(dir, { recursive: true });
  for (let entry = 0; entry < 8; entry++) {
    const contents = (entry + group) % 4 === 0 ? hitLine + file.slice(128) : file;
    writeFileSync(path.join(dir, `file-${entry}.log`), contents);
  }
}

const scenarios = [
  { id: 'absent', terms: ['MISSING_BRANCHCUT'], files: 96, matches: 0 },
  { id: 'sparse', terms: ['ERROR_BRANCHCUT'], files: 96, matches: 24 },
  { id: 'eight-literals', terms: ['ERROR_BRANCHCUT', ...Array.from({ length: 7 }, (_, i) => `MISSING_${i}`)], files: 96, matches: 24 },
  { id: 'one-shard', terms: ['ERROR_BRANCHCUT'], files: 8, matches: 2, dir: 'shard-0' },
];
const configurations = [1, 8].flatMap((threads) => [
  { tool: 'branchcut-before', binary: before, threads },
  { tool: 'branchcut', binary: branchcut, threads },
  { tool: 'ripgrep', binary: 'rg', threads },
]);
const sha256 = (value) => createHash('sha256').update(value).digest('hex');

function args(scenario, { tool, threads }) {
  const workers = ['--threads', String(threads)];
  const glob = scenario.dir ? `${scenario.dir}/*.log` : '**/*.log';
  if (tool === 'ripgrep') {
    return ['--no-config', '--no-ignore', '--no-encoding', '--hidden', '--no-mmap', '--color', 'never', '--no-heading', '--with-filename', '--line-number', '--column', '-F', ...workers, '--glob', '*.log', ...scenario.terms.flatMap((term) => ['-e', term]), scenario.dir || '.'];
  }
  const legacy = tool === 'branchcut-before' && baselineSyntax === 'legacy';
  return [...(legacy ? [] : ['grep']), '--hidden', ...workers, '--glob', glob, ...scenario.terms.flatMap((term) => [legacy ? '--content' : '-e', term])];
}

function run(scenario, config) {
  const started = performance.now();
  const result = spawnSync(config.binary, args(scenario, config), { cwd: fixture, encoding: 'utf8', windowsHide: true });
  const milliseconds = performance.now() - started;
  const legacy = config.tool === 'branchcut-before' && baselineSyntax === 'legacy';
  const expectedStatus = scenario.matches > 0 || legacy ? 0 : 1;
  if (result.status !== expectedStatus || result.error || result.stderr) {
    throw Error(`${scenario.id}/${config.tool}: ${result.error ?? (result.stderr || result.status)}`);
  }
  const lines = result.stdout.trimEnd().split(/\r?\n/).filter(Boolean)
    .map((line) => line.replaceAll('\\', '/').replace(/^\.\//, '')).sort();
  if (lines.length !== scenario.matches) throw Error(`Wrong result count for ${scenario.id}/${config.tool}`);
  return { milliseconds, outputHash: sha256(lines.join('\n')) };
}

const warmups = 2;
const samples = 7;
const reports = scenarios.map((scenario) => {
  const expected = run(scenario, configurations[0]).outputHash;
  const timings = configurations.map(({ tool, threads }) => ({ tool, threads, milliseconds: [] }));
  for (let iteration = 0; iteration < warmups + samples; iteration++) {
    const order = configurations.map((_, index) => (index + iteration) % configurations.length);
    if (iteration % 2 === 1) order.reverse();
    for (const index of order) {
      const { milliseconds, outputHash } = run(scenario, configurations[index]);
      if (outputHash !== expected) throw Error(`Output mismatch for ${scenario.id}/${configurations[index].tool}`);
      if (iteration >= warmups) timings[index].milliseconds.push(milliseconds);
    }
  }
  for (const timing of timings) {
    const ordered = [...timing.milliseconds].sort((left, right) => left - right);
    timing.median = ordered[Math.floor(samples / 2)];
    timing.p90 = ordered[Math.ceil(samples * 0.9) - 1];
  }
  console.log(scenario.id, timings.map(({ median }) => median.toFixed(2)).join(' / '));
  return { ...scenario, bytes: scenario.files * Buffer.byteLength(file), outputSha256: expected, timings };
});

const command = (binary, args) => spawnSync(binary, args, { encoding: 'utf8', windowsHide: true }).stdout.trim();
writeFileSync(reportPath, JSON.stringify({
  sourceHead: command('git', ['rev-parse', 'HEAD']),
  sourceDirty: Boolean(command('git', ['status', '--porcelain', '--untracked-files=no'])),
  binaries: { beforeSha256: sha256(readFileSync(before)), branchcutSha256: sha256(readFileSync(branchcut)) },
  rustc: command('rustc', ['--version']), ripgrep: command('rg', ['--version']),
  os: os.version(), kernel: os.release(), cpu: os.cpus()[0].model, logicalCpus: os.cpus().length,
  baselineSyntax,
  totalMemoryBytes: os.totalmem(), filesystem, node: process.version, warmups, samples, reports,
}, null, 2) + '\n');
