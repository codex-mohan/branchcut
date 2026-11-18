'use client';

import { Check, Copy } from 'lucide-react';
import Link from 'next/link';
import { useState } from 'react';
import { copyText } from '@/lib/clipboard';

type Workflow = {
  id: string;
  label: string;
  title: string;
  description: string;
  points: string[];
  command: string;
  path: string;
  branch: string;
  guide: string;
  output: {
    meta: [string, string];
    files?: Array<[string, string]>;
    raw?: string;
    stats: Array<[string, string]>;
  };
};

const workflows: Workflow[] = [
  {
    id: 'everyday',
    label: 'Everyday search',
    title: 'Find a filename without learning glob syntax.',
    description:
      'Plain text searches filename containment. Use explicit glob syntax only when structure and wildcards matter.',
    points: ['Literal filename search', 'ASCII smart case', 'Hidden paths excluded by default', 'Depth bounds'],
    command:
      "branchcut --cwd . config --exclude '**/target/**' --exclude '**/node_modules/**'",
    path: '~/workspace/branchcut',
    branch: 'git:main ✓',
    guide: '/docs/guides/simple-search',
    output: {
      meta: ['6 matches', '12.569 ms'],
      files: [
        ['MATCH', 'website/eslint.config.mjs'],
        ['MATCH', 'src/cli/config.rs'],
        ['MATCH', 'website/next.config.mjs'],
        ['MATCH', 'website/postcss.config.mjs'],
        ['MATCH', 'website/tsconfig.json'],
        ['MATCH', 'website/tsconfig.tsbuildinfo'],
      ],
      stats: [['directories opened', '239'], ['metadata calls', '1'], ['errors', '0']],
    },
  },
  {
    id: 'compose',
    label: 'Composed query',
    title: 'Compose the entire selection into one query.',
    description:
      'Globs, exclusions, metadata predicates, and limits are compiled together and evaluated from cheapest to most expensive.',
    points: ['Repeatable globs', 'Extension and type filters', 'Size and modification time', 'Limit without unnecessary metadata'],
    command:
      "branchcut\n  --glob '**/*.{rs,toml}'\n  --exclude '**/target/**'\n  --limit 4 --stats",
    path: '~/workspace/branchcut',
    branch: 'git:main ✓',
    guide: '/docs/guides/query-planning',
    output: {
      meta: ['4 matches', '1.764 ms'],
      files: [
        ['FILE', 'Cargo.toml'],
        ['FILE', 'examples/measure_engine.rs'],
        ['FILE', 'src/cli/args.rs'],
        ['FILE', 'src/cli/config.rs'],
      ],
      stats: [['directories opened', '17'], ['entries inspected', '133'], ['metadata calls', '1']],
    },
  },
  {
    id: 'automation',
    label: 'Automation',
    title: 'Keep exact native paths between programs.',
    description:
      'Choose line-oriented, JSON Lines, or NUL-delimited output. Command execution receives native paths as separate arguments.',
    points: ['NUL-delimited paths', 'JSON Lines with native companions', 'Shell-free command templates', 'Per-match or bounded batches'],
    command: "branchcut --glob '**/*.rs' --json --limit 3",
    path: '~/workspace/branchcut',
    branch: 'git:main ✓',
    guide: '/docs/guides/output-and-exec',
    output: {
      meta: ['3 streamed rows', 'JSON Lines'],
      raw: '{"path":"examples/measure_engine.rs"}\n{"path":"src/cli/args.rs"}\n{"path":"src/cli/config.rs"}',
      stats: [['shell interpolation', 'none'], ['native paths', 'preserved'], ['errors', '0']],
    },
  },
  {
    id: 'library',
    label: 'Rust library',
    title: 'Compile once, then stream native paths from Rust.',
    description:
      'The CLI is a thin adapter over the reusable library. Build a typed query, compile it, and visit paths with cancellation and error handling.',
    points: ['Typed query API', 'Reusable compiled plans', 'Sequential and parallel visitors', 'Candidate explanations'],
    command:
      "cargo run --release --example measure_engine -- --glob '**/*.rs' --cwd . --iterations 2",
    path: '~/workspace/branchcut',
    branch: 'git:main ✓',
    guide: '/docs/concepts/library-and-modules',
    output: {
      meta: ['compiled plan', '2 traversals'],
      raw: '{"compile_ns":58400}\n{"elapsed_ns":275267200,"first_match_ns":1351400,"matches":227,"dirs_opened":3614,"metadata_calls":1}\n{"elapsed_ns":387633200,"first_match_ns":913500,"matches":227,"dirs_opened":3614,"metadata_calls":1}',
      stats: [['completion', 'Complete'], ['first match', '0.91 ms'], ['native paths', 'yes']],
    },
  },
];

export function WorkflowTabs() {
  const [selectedId, setSelectedId] = useState<Workflow['id']>('everyday');
  const [copied, setCopied] = useState(false);
  const selected = workflows.find((item) => item.id === selectedId) ?? workflows[0];

  function selectWorkflow(id: Workflow['id']) {
    setSelectedId(id);
    setCopied(false);
  }

  async function copyCommand() {
    await navigator.clipboard.writeText(selected.command);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1400);
  }

  return (
    <div className="bc-feature-showcase" data-reveal="up">
      <header className="bc-feature-heading" data-reveal="left">
        <h2>One query. Every workflow.</h2>
        <p>
          Start with a filename, compose a precise query, stream paths into
          another program, or embed the same compiled engine in Rust.
        </p>
      </header>

      <div className="bc-feature-tabs" role="tablist" aria-label="Branchcut workflow">
        {workflows.map((workflow) => (
          <button
            type="button"
            role="tab"
            aria-selected={workflow.id === selected.id}
            className={workflow.id === selected.id ? 'active' : ''}
            key={workflow.id}
            onClick={() => selectWorkflow(workflow.id)}
          >
            {workflow.label}
          </button>
        ))}
      </div>

      <div className="bc-feature-panel" key={selected.id}>
        <div className="bc-feature-copy">
          <h3>{selected.title}</h3>
          <p>{selected.description}</p>
          <div className="bc-feature-points">
            {selected.points.map((point) => <span key={point}>{point}</span>)}
          </div>
          <div className="bc-feature-links">
            <Link href={selected.guide}>Read the guide</Link>
            <Link href="/docs/reference/feature-status">Feature status</Link>
          </div>
        </div>

        <div className="bc-console-stage">
          <div className="bc-console-window">
            <div className="bc-console-bar">
              <span className="bc-window-controls" aria-hidden="true"><i /><i /><i /></span>
              <span>{selected.id === 'library' ? 'measure_engine · main.rs' : 'branchcut · powershell'}</span>
              <button type="button" onClick={copyCommand} aria-label={copied ? 'Copied' : 'Copy command'}>
                {copied ? <Check aria-hidden="true" /> : <Copy aria-hidden="true" />}
              </button>
            </div>

            <div className="bc-console-screen" aria-live="polite">
              <div className="bc-shell-prompt" aria-label="Shell prompt">
                <span className="user">mohan</span>
                <span className="host">branchcut</span>
                <span className="path">{selected.path}</span>
                <span className="branch">{selected.branch}</span>
              </div>
              <div className="bc-command-line">
                <span aria-hidden="true">❯</span>
                <code>{selected.command}</code>
                <i aria-hidden="true" />
              </div>
              <div className="bc-console-output">
                <div className="bc-output-meta">
                  <b>{selected.output.meta[0]}</b>
                  <span>{selected.output.meta[1]}</span>
                </div>
                {selected.output.files?.map(([kind, value]) => (
                  <div className="bc-output-line" key={`${kind}-${value}`}>
                    <span>{kind}</span>
                    <code>{value}</code>
                    <b>✓</b>
                  </div>
                ))}
                {selected.output.raw && (
                  <pre className="bc-output-raw"><code>{selected.output.raw}</code></pre>
                )}
                <div className="bc-output-stats">
                  {selected.output.stats.map(([label, value]) => (
                    <span key={label}>{label} <b>{value}</b></span>
                  ))}
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div className="bc-feature-strip" aria-label="Core capabilities">
        <article><span>Selection</span><strong>Globs, literals, types, extensions</strong><p>Positive patterns form a union; filters compose with AND.</p></article>
        <article><span>Traversal</span><strong>Prefixes, pruning, depth, early stop</strong><p>The query changes what the walker opens and when it stops.</p></article>
        <article><span>Policy</span><strong>Hidden paths and nested ignores</strong><p>Explicit exclusions and opt-in Git-compatible ignore rules.</p></article>
        <article><span>Evidence</span><strong>Explain, why, and real work counters</strong><p>See the compiled plan and the filesystem work performed.</p></article>
      </div>
    </div>
  );
}
