'use client';

import { useEffect, useMemo, useState } from 'react';

type TerminalLine = {
  text: string;
  tone?: 'default' | 'muted' | 'green' | 'amber' | 'heading';
  delay?: number;
};

type Demo = {
  label: string;
  command: string;
  lines: TerminalLine[];
};

const demos: Demo[] = [
  {
    label: 'Query',
    command:
      "branchcut --glob '**/*.{rs,md}' --exclude '**/target/**' --exclude '**/node_modules/**' --count --stats",
    lines: [
      { text: '32', tone: 'green', delay: 250 },
      { text: 'matched                 32' },
      { text: 'directories considered  35' },
      { text: 'directories opened      33' },
      { text: 'directories pruned      2', tone: 'amber' },
      { text: '  positive              0', tone: 'muted' },
      { text: '  excluded              2', tone: 'amber' },
      { text: 'entries inspected       119' },
      { text: 'candidate files         77' },
      { text: 'metadata calls          1', tone: 'green' },
      { text: 'filesystem errors       0', tone: 'green' },
      { text: 'elapsed                 2.776ms', tone: 'green' },
    ],
  },
  {
    label: 'Explain',
    command:
      "branchcut --glob 'src/**/*.{rs,toml}' --exclude '**/target/**' --limit 100 --explain",
    lines: [
      { text: 'QUERY PLAN', tone: 'heading', delay: 260 },
      { text: '' },
      { text: 'ROOT', tone: 'heading' },
      { text: '  .\\src', tone: 'green' },
      { text: '' },
      { text: 'SHARED LITERAL PREFIX', tone: 'heading' },
      { text: '  src', tone: 'green' },
      { text: '' },
      { text: 'POSITIVE PATTERNS', tone: 'heading' },
      { text: '  src/**/*.rs [FixedPrefixRecursive]' },
      { text: '  src/**/*.toml [FixedPrefixRecursive]' },
      { text: '' },
      { text: 'EXCLUSIONS', tone: 'heading' },
      { text: '  **/target/** [UnboundedRecursive]', tone: 'amber' },
      { text: '' },
      { text: 'METADATA', tone: 'heading' },
      { text: '  not required', tone: 'green' },
      { text: '' },
      { text: 'TERMINATION', tone: 'heading' },
      { text: '  first 100 matches', tone: 'green' },
    ],
  },
  {
    label: 'Stream',
    command: "branchcut --glob 'docs/**/*.md' --limit 6 --stats",
    lines: [
      { text: 'docs/concepts/parallelism.md', tone: 'green', delay: 220 },
      { text: 'docs/concepts/path-semantics.md', tone: 'green' },
      { text: 'docs/concepts/query-compiler.md', tone: 'green' },
      { text: 'docs/concepts/traversal-and-pruning.md', tone: 'green' },
      { text: 'docs/contributing/benchmarking.md', tone: 'green' },
      { text: 'docs/contributing/documentation.md', tone: 'green' },
      { text: 'matched                 6', delay: 260 },
      { text: 'directories considered  4' },
      { text: 'directories opened      4' },
      { text: 'entries inspected       12' },
      { text: 'metadata calls          1', tone: 'green' },
      { text: 'elapsed                 0.543ms', tone: 'green' },
    ],
  },
];

export function LiveTerminal() {
  const [demoIndex, setDemoIndex] = useState(0);
  const [run, setRun] = useState(0);
  const [typed, setTyped] = useState('');
  const [visibleLines, setVisibleLines] = useState(0);
  const [phase, setPhase] = useState<'typing' | 'running' | 'done'>('typing');
  const demo = demos[demoIndex];
  const shownLines = useMemo(
    () => demo.lines.slice(0, visibleLines),
    [demo, visibleLines],
  );

  useEffect(() => {
    const timers: ReturnType<typeof setTimeout>[] = [];
    const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

    if (reducedMotion) {
      timers.push(setTimeout(() => {
        setTyped(demo.command);
        setVisibleLines(demo.lines.length);
        setPhase('done');
      }, 0));
      return () => timers.forEach(clearTimeout);
    }

    timers.push(setTimeout(() => {
      setTyped('');
      setVisibleLines(0);
      setPhase('typing');
    }, 0));

    for (let index = 1; index <= demo.command.length; index += 1) {
      timers.push(
        setTimeout(() => setTyped(demo.command.slice(0, index)), 18 * index),
      );
    }

    let elapsed = demo.command.length * 18 + 420;
    timers.push(setTimeout(() => setPhase('running'), elapsed - 120));

    demo.lines.forEach((line, index) => {
      elapsed += line.delay ?? (line.text === '' ? 70 : 105);
      timers.push(setTimeout(() => setVisibleLines(index + 1), elapsed));
    });

    timers.push(setTimeout(() => setPhase('done'), elapsed + 120));
    timers.push(
      setTimeout(
        () => setDemoIndex((current) => (current + 1) % demos.length),
        elapsed + 3600,
      ),
    );

    return () => timers.forEach(clearTimeout);
  }, [demo, run]);

  function selectDemo(index: number) {
    if (index === demoIndex) setRun((current) => current + 1);
    else setDemoIndex(index);
  }

  return (
    <div className="live-terminal" aria-label="Branchcut command demonstration">
      <div className="live-terminal-bar">
        <div className="window-controls" aria-hidden="true">
          <span className="traffic red" />
          <span className="traffic amber" />
          <span className="traffic green" />
        </div>
        <span className="terminal-directory">branchcut — pwsh</span>
        <span className={`terminal-status ${phase}`}>
          <i /> {phase === 'typing' ? 'typing' : phase === 'running' ? 'running' : 'complete'}
        </span>
      </div>

      <div className="live-terminal-screen" aria-live="polite">
        <div className="terminal-command">
          <span className="terminal-prompt">PS branchcut&gt;</span>{' '}
          <span>{typed}</span>
          {phase === 'typing' && <span className="terminal-cursor" aria-hidden="true" />}
        </div>
        {phase !== 'typing' && <div className="terminal-enter">↵</div>}
        <div className="terminal-output">
          {shownLines.map((line, index) => (
            <div className={`terminal-line ${line.tone ?? 'default'}`} key={`${index}-${line.text}`}>
              {line.text || '\u00a0'}
            </div>
          ))}
          {phase === 'running' && <span className="output-cursor" aria-hidden="true" />}
        </div>
      </div>

      <div className="terminal-demos" aria-label="Terminal demonstrations">
        {demos.map((item, index) => (
          <button
            className={index === demoIndex ? 'active' : ''}
            key={item.label}
            onClick={() => selectDemo(index)}
            type="button"
          >
            <span>0{index + 1}</span> {item.label}
          </button>
        ))}
        <button className="terminal-replay" onClick={() => setRun((current) => current + 1)} type="button">
          ↻ Replay
        </button>
      </div>
    </div>
  );
}
