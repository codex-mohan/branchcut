'use client';

import { Check, Copy } from 'lucide-react';
import { useState } from 'react';
import { copyText } from '@/lib/clipboard';

type Preset = {
  id: string;
  label: string;
  command: string;
  root: string;
  strategy: string;
  metadata: string;
  rows: Array<{
    depth: number;
    name: string;
    state: 'open' | 'match' | 'cut' | 'idle';
    note: string;
  }>;
};

const presets: Preset[] = [
  {
    id: 'fixed',
    label: 'Fixed prefix',
    command: "branchcut --glob 'packages/**/src/**/*.{rs,ts}'",
    root: 'packages/',
    strategy: 'shared literal prefix',
    metadata: 'not required',
    rows: [
      { depth: 0, name: 'packages/', state: 'open', note: 'root' },
      { depth: 1, name: 'core/src/', state: 'match', note: 'open' },
      { depth: 2, name: 'query.rs', state: 'match', note: 'match' },
      { depth: 1, name: 'web/src/', state: 'open', note: 'open' },
      { depth: 2, name: 'client.ts', state: 'match', note: 'match' },
      { depth: 1, name: 'target/', state: 'cut', note: 'cut' },
      { depth: 1, name: 'node_modules/', state: 'cut', note: 'cut' },
    ],
  },
  {
    id: 'exclude',
    label: 'Prune generated',
    command:
      "branchcut --glob '**/*.{rs,ts}' --exclude '**/{target,node_modules,dist}/**'",
    root: './',
    strategy: 'trailing exclusion pruning',
    metadata: 'not required',
    rows: [
      { depth: 0, name: 'repository/', state: 'open', note: 'root' },
      { depth: 1, name: 'src/', state: 'match', note: 'open' },
      { depth: 2, name: 'main.rs', state: 'match', note: 'match' },
      { depth: 1, name: 'crates/', state: 'open', note: 'open' },
      { depth: 2, name: 'engine/lib.rs', state: 'match', note: 'match' },
      { depth: 1, name: 'target/', state: 'cut', note: 'cut' },
      { depth: 1, name: 'node_modules/', state: 'cut', note: 'cut' },
      { depth: 1, name: 'dist/', state: 'cut', note: 'cut' },
    ],
  },
  {
    id: 'limit',
    label: 'Stop early',
    command: "branchcut --glob '**/*.rs' --limit 10 --stats",
    root: './',
    strategy: 'sequential early termination',
    metadata: 'not required',
    rows: [
      { depth: 0, name: 'repository/', state: 'open', note: 'root' },
      { depth: 1, name: 'src/', state: 'match', note: 'open' },
      { depth: 2, name: 'main.rs', state: 'match', note: '01 match' },
      { depth: 2, name: 'query.rs', state: 'match', note: '02 match' },
      { depth: 2, name: 'pattern/mod.rs', state: 'match', note: '03 match' },
      { depth: 1, name: 'tests/', state: 'open', note: 'open' },
      { depth: 2, name: 'cli.rs', state: 'match', note: 'next' },
      { depth: 1, name: 'remaining work', state: 'cut', note: 'stop after 10' },
    ],
  },
];

export function QueryMap() {
  const [selected, setSelected] = useState(presets[0]);
  const [copied, setCopied] = useState(false);

  async function copyCommand() {
    await navigator.clipboard.writeText(selected.command);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1400);
  }

  return (
    <div className="bc-query-map">
      <div className="bc-window-bar">
        <div className="bc-window-dots" aria-hidden="true">
          <i />
          <i />
          <i />
        </div>
        <span>branchcut / planner</span>
        <span className="bc-live-dot">ready</span>
      </div>

      <div className="bc-command-row">
        <span aria-hidden="true">$</span>
        <code>{selected.command}</code>
        <button type="button" onClick={copyCommand} aria-label="Copy query">
          {copied ? <Check aria-hidden="true" /> : <Copy aria-hidden="true" />}
        </button>
      </div>

      <div className="bc-query-tabs" role="tablist" aria-label="Query presets">
        {presets.map((preset) => (
          <button
            type="button"
            role="tab"
            aria-selected={selected.id === preset.id}
            className={selected.id === preset.id ? 'active' : ''}
            key={preset.id}
            onClick={() => setSelected(preset)}
          >
            {preset.label}
          </button>
        ))}
      </div>

      <div className="bc-plan-head">
        <span>ROOT</span>
        <strong>{selected.root}</strong>
        <span>STRATEGY</span>
        <strong>{selected.strategy}</strong>
        <span>METADATA</span>
        <strong>{selected.metadata}</strong>
      </div>

      <div className="bc-tree-stage" key={selected.id}>
        <div className="bc-scan-line" aria-hidden="true" />
        <div className="bc-tree" aria-label={`${selected.label} traversal preview`}>
          {selected.rows.map((row) => (
            <div
              className={`bc-tree-row state-${row.state}`}
              style={{ '--depth': row.depth } as React.CSSProperties}
              key={`${row.depth}-${row.name}`}
            >
              <span className="bc-tree-joint" aria-hidden="true" />
              <code>{row.name}</code>
              <span>{row.note}</span>
              {row.state === 'cut' && <i aria-hidden="true" />}
            </div>
          ))}
        </div>
        <p className="bc-preview-note">
          Illustrative traversal geometry. Use <code>--explain</code> and{' '}
          <code>--stats</code> for the actual work on your tree.
        </p>
      </div>
    </div>
  );
}
