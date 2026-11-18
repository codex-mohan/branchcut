'use client';

import { useEffect, useRef, type CSSProperties } from 'react';

type Result = {
  tool: string;
  version?: string;
  time: number | null;
};

type Workload = {
  id: string;
  title: string;
  label: string;
  description: string;
  matches: string;
  mode: string;
  command: string;
  verification: string;
  multiplier: string;
  multiplierContext: string;
  results: Result[];
};

const rawBenchmarkUrl =
  'https://github.com/codex-mohan/branchcut/blob/master/benchmarks/results/2026-11-18-gap-closure-cli.json';
const rawMemoryUrl =
  'https://github.com/codex-mohan/branchcut/blob/master/benchmarks/results/2026-11-18-gap-closure-cli.json';

const workloads: Workload[] = [
  {
    "id": "broad",
    "title": "Broad files",
    "label": "Broad files",
    "description": "All 100,002 non-hidden files under the generated tree.",
    "matches": "100,002",
    "mode": "automatic workers",
    "command": "branchcut --cwd $FIXTURE --glob '**/*' --threads 0",
    "verification": "100,002 unique paths matched the generator manifest before and after timing.",
    "multiplier": "1.82×",
    "multiplierContext": "vs zlob",
    "results": [
      {
        "tool": "Branchcut",
        "time": 68.02
      },
      {
        "tool": "ripgrep",
        "version": "15.1.0",
        "time": 109.17
      },
      {
        "tool": "fd",
        "version": "10.4.2",
        "time": 117.14
      },
      {
        "tool": "zlob",
        "version": "1.7.0",
        "time": 123.94
      },
      {
        "tool": "tinyglobby",
        "version": "0.2.14",
        "time": 242.30
      },
      {
        "tool": "fast-glob",
        "version": "3.3.3",
        "time": 370.45
      }
    ]
  },
  {
    "id": "fixed",
    "title": "Fixed-prefix Rust",
    "label": "Fixed-prefix Rust",
    "description": "Only packages/p00/src/**/*.rs can contribute.",
    "matches": "1,250",
    "mode": "four workers",
    "command": "branchcut --cwd $FIXTURE --glob 'packages/p00/src/**/*.rs' --threads 4",
    "verification": "1,250 unique paths matched the generated result set.",
    "multiplier": "1.06×",
    "multiplierContext": "vs zlob · close result",
    "results": [
      {
        "tool": "Branchcut",
        "time": 33.39
      },
      {
        "tool": "zlob",
        "version": "1.7.0",
        "time": 35.46
      },
      {
        "tool": "fd",
        "version": "10.4.2",
        "time": 39.63
      },
      {
        "tool": "ripgrep",
        "version": "15.1.0",
        "time": 75.23
      },
      {
        "tool": "tinyglobby",
        "version": "0.2.14",
        "time": 153.56
      },
      {
        "tool": "fast-glob",
        "version": "3.3.3",
        "time": 210.08
      }
    ]
  },
  {
    "id": "exclude",
    "title": "Rust + exclusions",
    "label": "Rust + exclusions",
    "description": "Rust and TOML selection with generated trees excluded.",
    "matches": "37,501",
    "mode": "automatic workers",
    "command": "branchcut --glob '**/*.{rs,toml}' --exclude '**/target/**' --exclude '**/node_modules/**' --threads 0",
    "verification": "37,501 unique paths matched. Unmeasured combinations were omitted; this is a file-selection benchmark.",
    "multiplier": "1.35×",
    "multiplierContext": "vs ripgrep",
    "results": [
      {
        "tool": "Branchcut",
        "time": 58.88
      },
      {
        "tool": "ripgrep",
        "version": "15.1.0",
        "time": 79.27
      },
      {
        "tool": "fd",
        "version": "10.4.2",
        "time": 92.57
      },
      {
        "tool": "tinyglobby",
        "version": "0.2.14",
        "time": 208.11
      },
      {
        "tool": "fast-glob",
        "version": "3.3.3",
        "time": 333.23
      },
      {
        "tool": "zlob",
        "version": "1.7.0",
        "time": null
      }
    ]
  },
  {
    "id": "limit",
    "title": "First 10 Rust files",
    "label": "First 10 Rust",
    "description": "Early termination becomes part of traversal.",
    "matches": "10",
    "mode": "four workers",
    "command": "branchcut --glob '**/*.rs' --limit 10 --threads 4",
    "verification": "Exactly ten unique paths from the complete valid set.",
    "multiplier": "2.78×",
    "multiplierContext": "vs zlob",
    "results": [
      {
        "tool": "Branchcut",
        "time": 35.41
      },
      {
        "tool": "fd",
        "version": "10.4.2",
        "time": 46.00
      },
      {
        "tool": "zlob",
        "version": "1.7.0",
        "time": 98.46
      },
      {
        "tool": "ripgrep",
        "version": "15.1.0",
        "time": null
      },
      {
        "tool": "fast-glob",
        "version": "3.3.3",
        "time": null
      },
      {
        "tool": "tinyglobby",
        "version": "0.2.14",
        "time": null
      }
    ]
  }
];

const memoryResults = [
  {
    "tool": "Branchcut",
    "version": "0.1.0",
    "memory": 6.38
  },
  {
    "tool": "zlob",
    "version": "1.7.0",
    "memory": 20.17
  },
  {
    "tool": "ripgrep",
    "version": "15.1.0",
    "memory": 14.29
  },
  {
    "tool": "fd",
    "version": "10.4.2",
    "memory": 18.25
  },
  {
    "tool": "fast-glob",
    "version": "3.3.3",
    "memory": 111.43
  },
  {
    "tool": "tinyglobby",
    "version": "0.2.14",
    "memory": 84.71
  }
];

const summaries = [
  { workload: workloads[0],
  "time": 68.02,
  "comparison": "zlob",
  "comparisonTime": 123.94,
  "callout": "1.82× faster than zlob",
  "caption": "100,002 matches · automatic workers"
  },
  { workload: workloads[1],
  "time": 33.39,
  "comparison": "zlob",
  "comparisonTime": 35.46,
  "callout": "1.06× · close to zlob",
  "caption": "1,250 matches · four workers"
  },
  { workload: workloads[2],
  "time": 58.88,
  "comparison": "ripgrep",
  "comparisonTime": 79.27,
  "callout": "1.35× faster than ripgrep",
  "caption": "37,501 matches · automatic workers"
  },
  { workload: workloads[3],
  "time": 35.41,
  "comparison": "zlob",
  "comparisonTime": 98.46,
  "callout": "2.78× faster than zlob",
  "caption": "10 matches · four workers"
  }
];

function format(value: number | null) {
  return value === null ? null : `${value.toFixed(2)} ms`;
}

function barWidth(time: number, max: number) {
  return `${Math.max(1.5, (time / max) * 100)}%`;
}

function BenchmarkChart({ workload, index }: { workload: Workload; index: number }) {
  const measured = workload.results.flatMap((result) => (result.time === null ? [] : [result.time]));
  const max = Math.max(...measured);

  return (
    <article className="bc-bench-chart" data-reveal="up" data-bench-plot>
      <header className="bc-bench-chart-head">
        <div>
          <h3>{workload.title}</h3>
          <p>{workload.description}</p>
        </div>
        <div className="bc-bench-multiplier">
          {workload.multiplier}
          <small>{workload.multiplierContext}</small>
        </div>
      </header>

      <div className="bc-bench-chart-meta">
        <span>↓ Lower is better</span>
        <span>{workload.matches} matches</span>
      </div>

      <div className="bc-bench-legend" aria-hidden="true">
        <span><i /> Branchcut</span>
        <span><i className="other" /> Others</span>
      </div>

      <div className="bc-bench-scale" aria-hidden="true">
        <span>0</span>
        <span>{(max * 0.25).toFixed(1)}</span>
        <span>{(max * 0.5).toFixed(1)}</span>
        <span>{(max * 0.75).toFixed(1)}</span>
        <span>{max.toFixed(1)} ms</span>
      </div>

      <div
        className="bc-bench-rows"
        role="img"
        aria-label={`${workload.title} timing comparison. Lower is better.`}
      >
        {workload.results.map((result) => (
          <div
            className={`bc-bench-row ${result.tool === 'Branchcut' ? 'is-branchcut' : 'other'}`}
            key={result.tool}
          >
            <div className="bc-bench-tool">
              <strong>{result.tool}</strong>
              <small>{result.version ?? 'automatic policy'}</small>
            </div>
            <div className="bc-bench-track" aria-hidden="true">
              {result.time === null ? null : (
                <i
                  className="bc-bench-fill"
                  style={{
                    '--bar-width': barWidth(result.time, max),
                    '--bar-delay': `${index * 55 + workload.results.indexOf(result) * 55}ms`,
                  } as CSSProperties}
                />
              )}
            </div>
            <output>
              {format(result.time) ?? <small>not comparable</small>}
              {result.time !== null && result.tool !== 'Branchcut' && (
                <em>{(result.time / workload.results[0].time!).toFixed(2)}× more</em>
              )}
            </output>
          </div>
        ))}
      </div>

      <details className="bc-bench-details">
        <summary>
          <span>Open bench · Windows 11 / NTFS · 9 samples</span>
        </summary>
        <div className="bc-bench-details-grid">
          <div>
            <span>Command</span>
            <code>{workload.command}</code>
          </div>
          <div>
            <span>Correctness</span>
            <p>{workload.verification}</p>
          </div>
          <div>
            <span>Method</span>
            <p>Three warmups and nine measured fresh processes per configuration.</p>
            <a href={rawBenchmarkUrl}>Raw record</a>
          </div>
        </div>
      </details>
    </article>
  );
}

export function BenchmarkLab() {
  const sectionRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const node = sectionRef.current;
    if (!node) return;

    const plots = Array.from(node.querySelectorAll<HTMLElement>('[data-bench-plot]'));
    const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    if (reducedMotion || !('IntersectionObserver' in window)) {
      plots.forEach((plot) => plot.classList.add('is-drawn'));
      return;
    }

    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (!entry.isIntersecting) continue;
          entry.target.classList.add('is-drawn');

        }
      },
      {
        rootMargin: '0px 0px -8% 0px',
        threshold: 0.22,
      },
    );

    plots.forEach((plot) => observer.observe(plot));
    return () => observer.disconnect();
  }, []);

  function replay() {
    const node = sectionRef.current;
    if (!node) return;

    const plots = Array.from(node.querySelectorAll<HTMLElement>('[data-bench-plot]'));
    plots.forEach((plot) => plot.classList.remove('is-drawn'));

    const replayVisiblePlots = () => {
      for (const plot of plots) {
        const bounds = plot.getBoundingClientRect();
        if (bounds.bottom > 0 && bounds.top < window.innerHeight) {
          plot.classList.add('is-drawn');
        }
      }
    };

    requestAnimationFrame(() => requestAnimationFrame(replayVisiblePlots));
  }

  return (
    <div className="bc-performance" ref={sectionRef}>
      <header className="bc-performance-heading" data-reveal="left">
        <h2>Fast where the work is hard.</h2>
        <p>
          Branchcut avoids directories that cannot answer. The retained fixture
          compares <strong>100,000 files</strong> through each tool&apos;s
          official CLI or public API, with startup, traversal, filtering, and
          output included.
        </p>
      </header>

      <div className="bc-summary-grid">
        {summaries.map((summary) => (
          <article className="bc-summary-card" data-bench-plot key={summary.workload.id}>
            <div className="bc-summary-head">
              <span>{summary.workload.label}</span>
              <b>{summary.callout}</b>
            </div>
            <strong className="bc-summary-value">
              {summary.time.toFixed(2)} <small>ms</small>
            </strong>
            <div className="bc-summary-bars">
              <div>
                <span>Branchcut</span>
                <i style={{ '--mini-width': barWidth(summary.time, summary.comparisonTime) } as CSSProperties} />
                <output>{summary.time.toFixed(2)}</output>
              </div>
              <div className="other">
                <span>{summary.comparison}</span>
                <i style={{ '--mini-width': '100%' } as CSSProperties} />
                <output>{summary.comparisonTime.toFixed(2)}</output>
              </div>
            </div>
            <p>{summary.caption}</p>
          </article>
        ))}
      </div>

      <div className="bc-bench-grid">
        {workloads.map((workload, index) => (
          <BenchmarkChart workload={workload} index={index} key={workload.id} />
        ))}
      </div>

      <section className="bc-memory" aria-labelledby="memory-title" data-reveal="up">
        <div className="bc-memory-copy">
          <h3 id="memory-title">Memory across the field.</h3>
          <p>
            Separate runs return the same 100,002 matching paths on the 100,000-file fixture
            as newline-delimited output through every tool, using default invocations.
          </p>
        </div>
        <div className="bc-memory-data">
          <div className="bc-memory-total">
            <strong>6.38 <small>MiB</small></strong>
            <span>17.5× smaller working set than fast-glob</span>
          </div>
          <div className="bc-memory-rows" data-bench-plot>
            {memoryResults.map((result, index) => (
              <div
                className={`bc-memory-row ${result.tool === 'Branchcut' ? 'is-branchcut' : 'other'}`}
                key={result.tool}
              >
                <span>{result.tool} <small>{result.version}</small></span>
                <i
                  style={{
                    '--mini-width': barWidth(result.memory, Math.max(...memoryResults.map((item) => item.memory))),
                    '--bar-delay': `${index * 55}ms`,
                  } as CSSProperties}
                />
                <output>{result.memory.toFixed(2)} MiB</output>
              </div>
            ))}
          </div>
          <p className="bc-memory-note">
            Median of three observed OS peak working-set samples, polled every
            1 ms. Observed process working set; late peaks can be missed. Parent capture memory is excluded. {' '}
            <a href={rawMemoryUrl}>Raw record</a>.
          </p>
        </div>
      </section>

      <footer className="bc-performance-foot">
        <span>
          <strong>Method:</strong> fresh-process wall-clock time · 3 warmups ·
          9 measured samples · exact path sets verified
        </span>
        <span>18 Nov 2026 · Windows / NTFS · fastest tested workers · raw data retained</span>
        <button type="button" onClick={replay}>Replay animation</button>
      </footer>
    </div>
  );
}
