#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { createRequire } from 'node:module';
import os from 'node:os';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { mkdtemp, mkdir, rm, symlink, writeFile } from 'node:fs/promises';

const EXPECTED_FAST_GLOB = '3.3.3';

function usage() {
  return `Usage: node scripts/verify.mjs --binary PATH --oracle-dir PATH\n`;
}

function parseArgs(argv) {
  const options = {};
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === '--help' || arg === '-h') {
      process.stdout.write(usage());
      process.exit(0);
    }
    if (arg !== '--binary' && arg !== '--oracle-dir') {
      throw new Error(`Unknown option: ${arg}\n${usage()}`);
    }
    const value = argv[++i];
    if (!value || value.startsWith('--')) throw new Error(`Missing value for ${arg}`);
    options[arg.slice(2).replace('-', '_')] = path.resolve(value);
  }
  if (!options.binary || !options.oracle_dir) throw new Error(usage());
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
    throw new Error(`${program} ${args.join(' ')} exited ${result.status}:\n${result.stderr}`);
  }
  return result;
}

async function makeFixture() {
  const root = await mkdtemp(path.join(os.tmpdir(), 'branchcut-verify-'));
  const files = [
    'README.md',
    'root.txt',
    'alpha/lib.rs',
    'alpha/src/lib.rs',
    'alpha/src/main.rs',
    'alpha/src/unit.test.rs',
    'alpha/src/nested/mod.rs',
    'alpha/src/nested/view.ts',
    'alpha/src/config.toml',
    'alpha/target/debug/generated.rs',
    'alpha/node_modules/dep/index.ts',
    'alpha/.secret/key.rs',
    'alpha/src/.hidden/secret.rs',
    'alpha/logs/worker.log',
    'beta/lib.rs',
    'beta/src/main.rs',
    'beta/src/nested/mod.rs',
    'beta/docs/readme.md',
    'empty/placeholder.txt',
  ];
  for (const file of files) {
    const destination = path.join(root, file);
    await mkdir(path.dirname(destination), { recursive: true });
    await writeFile(destination, 'branchcut verification fixture\n');
  }
  try {
    await symlink(path.join(root, 'alpha/lib.rs'), path.join(root, 'alpha/lib-link.rs'));
  } catch (error) {
    if (!['EPERM', 'EACCES', 'ENOTSUP'].includes(error.code)) throw error;
  }
  return { root, fileCount: files.length };
}

function normalize(paths) {
  return paths.map((value) => value.replaceAll('\\', '/')).sort();
}

function assertNoDuplicates(label, values) {
  if (new Set(values).size !== values.length) {
    throw new Error(`${label} returned duplicate paths: ${values.join(', ')}`);
  }
}

function equalResults(label, expected, actual) {
  const lhs = normalize(expected);
  const rhs = normalize(actual);
  if (lhs.length === rhs.length && lhs.every((value, index) => value === rhs[index])) return;
  const left = new Set(lhs);
  const right = new Set(rhs);
  const missing = lhs.filter((value) => !right.has(value));
  const extra = rhs.filter((value) => !left.has(value));
  throw new Error(
    `${label}: result mismatch\n  missing from Branchcut: ${missing.join(', ') || '(none)'}` +
    `\n  extra from Branchcut: ${extra.join(', ') || '(none)'}`,
  );
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  const oracleRequire = createRequire(pathToFileURL(path.join(options.oracle_dir, 'branchcut-oracle-loader.cjs')));
  const oraclePackage = oracleRequire('fast-glob/package.json');
  if (oraclePackage.version !== EXPECTED_FAST_GLOB) {
    throw new Error(`Expected fast-glob@${EXPECTED_FAST_GLOB}, found ${oraclePackage.version}`);
  }
  const fastGlob = oracleRequire('fast-glob');
  const fixture = await makeFixture();
  const cases = [
    { name: 'literal', patterns: ['alpha/lib.rs'] },
    { name: 'shallow wildcard', patterns: ['alpha/*.rs'] },
    { name: 'question wildcard', patterns: ['alpha/src/ma?n.rs'] },
    { name: 'character class', patterns: ['alpha/src/[mn]ain.rs'] },
    { name: 'negated character class', patterns: ['alpha/src/[!x]ain.rs'] },
    { name: 'zero or many globstar components', patterns: ['**/main.rs'] },
    { name: 'flat braces', patterns: ['**/*.{rs,toml}'] },
    { name: 'multiple positive patterns', patterns: ['alpha/**/*.rs', 'beta/**/*.ts'] },
    { name: 'overlapping positive patterns deduplicate', patterns: ['**/*.rs', 'alpha/lib.rs'] },
    {
      name: 'globstar branch isolation',
      patterns: ['alpha/**/missing.rs', 'alpha/lib.rs'],
    },
    {
      name: 'subtree exclusions',
      patterns: ['**/*.rs'],
      excludes: ['**/target/**', '**/node_modules/**'],
    },
    { name: 'hidden paths excluded by default', patterns: ['**/*.rs'] },
    { name: 'hidden paths included explicitly', patterns: ['**/*.rs'], hidden: true },
    { name: 'no matches', patterns: ['**/does-not-exist.xyz'] },
  ];

  try {
    for (const testCase of cases) {
      const oracleOptions = {
        cwd: fixture.root,
        dot: Boolean(testCase.hidden),
        onlyFiles: true,
        unique: true,
        followSymbolicLinks: false,
        suppressErrors: false,
      };
      if (testCase.excludes?.length) oracleOptions.ignore = testCase.excludes;
      const expected = normalize(await fastGlob(testCase.patterns, oracleOptions));
      const baseArgs = ['--cwd', fixture.root];
      for (const pattern of testCase.patterns) baseArgs.push('--glob', pattern);
      for (const pattern of testCase.excludes ?? []) baseArgs.push('--exclude', pattern);
      if (testCase.hidden) baseArgs.push('--hidden');
      baseArgs.push('--type', 'file', '--sort', '--json');

      const sequential = run(options.binary, baseArgs);
      const sequentialPaths = sequential.stdout.trim()
        ? sequential.stdout.trimEnd().split(/\r?\n/).map((line) => JSON.parse(line).path)
        : [];
      assertNoDuplicates(`${testCase.name} (sequential)`, sequentialPaths);
      equalResults(`${testCase.name} (sequential)`, expected, sequentialPaths);

      if (!testCase.limit) {
        const parallel = run(options.binary, [...baseArgs, '--threads', '2']);
        const parallelPaths = parallel.stdout.trim()
          ? parallel.stdout.trimEnd().split(/\r?\n/).map((line) => JSON.parse(line).path)
          : [];
        assertNoDuplicates(`${testCase.name} (parallel)`, parallelPaths);
        equalResults(`${testCase.name} (parallel)`, expected, parallelPaths);
      }
      process.stdout.write(`PASS ${testCase.name} (${expected.length} matches)\n`);
    }
    process.stdout.write(`\n${cases.length} cases passed; fixture files: ${fixture.fileCount}\n`);
  } finally {
    const resolved = path.resolve(fixture.root);
    if (!resolved.startsWith(path.resolve(os.tmpdir()) + path.sep)) {
      throw new Error(`Refusing to remove fixture outside temporary directory: ${resolved}`);
    }
    await rm(resolved, { recursive: true, force: true });
  }
}

main().catch((error) => {
  process.stderr.write(`verify: ${error.stack ?? error}\n`);
  process.exitCode = 1;
});
