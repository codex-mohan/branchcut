'use client';

import { useState } from 'react';

const command = 'cargo install --path .';

export function CopyInstall() {
  const [copied, setCopied] = useState(false);

  async function copy() {
    await navigator.clipboard.writeText(command);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1400);
  }

  return (
    <div className="install-line">
      <span>$</span> {command}
      <button onClick={copy} type="button" aria-label="Copy install command">
        {copied ? '✓' : '⧉'}
      </button>
    </div>
  );
}
