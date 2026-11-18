'use client';

import { Check, Copy } from 'lucide-react';
import { useState } from 'react';
import { copyText } from '@/lib/clipboard';

type InstallMethod = {
  id: 'windows' | 'unix' | 'cargo';
  label: string;
  command: string;
  note: string;
  scriptUrl?: string;
};

const methods: readonly InstallMethod[] = [
  {
    id: 'windows',
    label: 'Windows',
    command:
      'irm https://raw.githubusercontent.com/codex-mohan/branchcut/master/scripts/install.ps1 | iex',
    note: 'Runs the repository PowerShell installer, then verifies the installed executable.',
    scriptUrl:
      'https://github.com/codex-mohan/branchcut/blob/master/scripts/install.ps1',
  },
  {
    id: 'unix',
    label: 'Linux / macOS',
    command:
      "curl --proto '=https' --tlsv1.2 -fsSL https://raw.githubusercontent.com/codex-mohan/branchcut/master/scripts/install.sh | sh",
    note: 'Runs the repository POSIX installer, then verifies the installed executable.',
    scriptUrl:
      'https://github.com/codex-mohan/branchcut/blob/master/scripts/install.sh',
  },
  {
    id: 'cargo',
    label: 'Existing Rust',
    command:
      'cargo install --locked --force --git https://github.com/codex-mohan/branchcut.git',
    note: 'For systems that already have a current stable Rust toolchain.',
  },
] as const;

export function InstallTabs() {
  const [selectedId, setSelectedId] = useState<InstallMethod['id']>('windows');
  const [copied, setCopied] = useState(false);
  const selected = methods.find((method) => method.id === selectedId) ?? methods[0];

  async function copyCommand() {
    if (!(await copyText(selected.command))) return;
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1400);
  }

  return (
    <div className="bc-install-box">
      <div className="bc-install-tabs" role="tablist" aria-label="Install method">
        {methods.map((method) => (
          <button
            type="button"
            role="tab"
            aria-selected={method.id === selected.id}
            className={method.id === selected.id ? 'active' : ''}
            key={method.id}
            onClick={() => {
              setSelectedId(method.id);
              setCopied(false);
            }}
          >
            {method.label}
          </button>
        ))}
      </div>
      <div className="bc-install-command">
        <span aria-hidden="true">$</span>
        <code>{selected.command}</code>
        <button type="button" aria-label="Copy install command" onClick={copyCommand}>
          {copied ? <Check aria-hidden="true" /> : <Copy aria-hidden="true" />}
        </button>
      </div>
      <p>
        {selected.note}
        {selected.scriptUrl && (
          <>
            {' '}
            <a href={selected.scriptUrl} target="_blank" rel="noreferrer">
              Review script
            </a>
          </>
        )}
      </p>
    </div>
  );
}