import Image from 'next/image';
import Link from 'next/link';
import { LiveTerminal } from '@/components/live-terminal';
import { CopyInstall } from '@/components/copy-install';

export default function HomePage() {
  return (
    <main className="branchcut-home branchcut-home-v2">
      <section className="hero-grid hero-live">
        <div className="hero-copy">
          <div className="eyebrow"><span /> ZERO CRATES · ONE RUST FILE</div>
          <div className="hero-brand">
            <Image src="/branchcut-icon.svg" alt="" width={88} height={48} priority />
            <span>branchcut</span>
          </div>
          <h1>Compile the query.<br /><em>Cut the tree.</em></h1>
          <p>
            One filesystem query engine that plans the walk, shares glob states,
            and cuts irrelevant directories before opening them.
          </p>
          <div className="hero-actions">
            <Link className="primary-action" href="/docs/getting-started/quick-start">Get started <span>→</span></Link>
            <Link className="secondary-action" href="/docs">Read the docs</Link>
          </div>
          <CopyInstall />
        </div>

        <LiveTerminal />
      </section>

      <section className="proof-strip proof-live" aria-label="Project facts">
        <div><strong>0</strong><span>third-party crates</span></div>
        <div><strong>1</strong><span>Rust source file</span></div>
        <div><strong>**</strong><span>component-aware globstar</span></div>
        <div><strong>0</strong><span>per-entry metadata calls</span></div>
      </section>

      <section className="feature-intro">
        <p className="section-kicker">THE QUERY BECOMES THE WALK</p>
        <h2>Watch the planner do less.</h2>
        <p>Every feature changes traversal—not merely the filtering performed afterward.</p>
      </section>

      <section className="feature-lab" aria-label="Branchcut features">
        <article className="feature-widget feature-wide">
          <div className="widget-copy">
            <span className="widget-number">01 / QUERY COMPILER</span>
            <h3>Four patterns.<br />One shared program.</h3>
            <p>Common path segments compile once and stay shared while patterns diverge at the leaf.</p>
            <Link href="/docs/concepts/query-compiler">Inside the compiler →</Link>
          </div>
          <div className="compiler-widget" aria-label="Patterns combining into a shared query program">
            <div className="pattern-inputs">
              <code>src/**/*.rs</code>
              <code>src/**/*.toml</code>
              <code>src/**/test*.rs</code>
              <code>src/components/**/*.css</code>
            </div>
            <div className="compile-spine"><span>COMPILE</span></div>
            <div className="compiled-tree">
              <code><b>src/</b></code>
              <code className="tree-indent"><b>└── **/</b></code>
              <code className="tree-indent-2">├── *.rs</code>
              <code className="tree-indent-2">├── *.toml</code>
              <code className="tree-indent-2">├── test*.rs</code>
              <code className="tree-indent-2">└── components/**/*.css</code>
            </div>
          </div>
        </article>

        <article className="feature-widget">
          <div className="widget-copy compact">
            <span className="widget-number">02 / PRUNING</span>
            <h3>Cut before opening.</h3>
            <p>Excluded and impossible subtrees never become a full walk.</p>
          </div>
          <div className="prune-widget" aria-label="Filesystem tree with pruned directories">
            <div><span>◆</span> repository/</div>
            <div className="keep"><span>├─</span> src/ <b>OPEN</b></div>
            <div className="keep child"><span>│ └─</span> main.rs <b>MATCH</b></div>
            <div className="cut cut-one"><span>├─</span> target/ <b>╳ CUT</b></div>
            <div className="cut cut-two"><span>├─</span> node_modules/ <b>╳ CUT</b></div>
            <div className="cut cut-three"><span>└─</span> dist/ <b>╳ CUT</b></div>
          </div>
        </article>

        <article className="feature-widget">
          <div className="widget-copy compact">
            <span className="widget-number">03 / STREAMING</span>
            <h3>Stop when the query is done.</h3>
            <p>Matches arrive immediately. <code>--limit</code> becomes a traversal condition.</p>
          </div>
          <div className="stream-widget" aria-label="A query stopping after three streamed matches">
            <div className="stream-head"><span>--limit 3</span><b>3 / 3</b></div>
            <div className="stream-row row-one"><i /> src/main.rs <b>01</b></div>
            <div className="stream-row row-two"><i /> src/query.rs <b>02</b></div>
            <div className="stream-row row-three"><i /> src/walk.rs <b>03</b></div>
            <div className="stream-stop"><span>■</span> traversal stopped</div>
          </div>
        </article>

        <article className="feature-widget feature-wide metadata-feature">
          <div className="widget-copy">
            <span className="widget-number">04 / COST AWARENESS</span>
            <h3>Do not ask the filesystem<br />questions you do not need.</h3>
            <p>Type information comes from directory entries. Metadata work stays out of the hot path unless the query requires it.</p>
            <Link href="/docs/concepts/traversal-and-pruning">Traversal architecture →</Link>
          </div>
          <div className="metadata-widget">
            <div className="metadata-query"><span>QUERY</span><code>**/*.rs</code></div>
            <div className="metadata-wire"><i /><i /><i /><i /><i /></div>
            <div className="metadata-readout">
              <span>per-entry metadata</span>
              <strong>0</strong>
              <small>NOT REQUIRED</small>
            </div>
          </div>
        </article>
      </section>

      <section className="comparison-section">
        <div className="comparison-heading">
          <p className="section-kicker">THE DIFFERENCE</p>
          <h2>Filtering late versus planning early.</h2>
        </div>
        <div className="comparison-grid">
          <div className="comparison-card conventional">
            <span>CONVENTIONAL PIPELINE</span>
            <div className="work-meter"><i /><i /><i /><i /><i /><i /><i /><i /><i /><i /></div>
            <code>walk → match → ignore → filter</code>
            <strong>Everything enters the pipeline.</strong>
          </div>
          <div className="comparison-arrow">→</div>
          <div className="comparison-card planned">
            <span>BRANCHCUT PLAN</span>
            <div className="work-meter"><i /><i /><i className="off" /><i className="off" /><i className="off" /><i className="off" /><i className="off" /><i className="off" /><i className="off" /><i className="off" /></div>
            <code>compile → prune → stream</code>
            <strong>Impossible work never begins.</strong>
          </div>
        </div>
      </section>

      <section className="docs-cta">
        <Image src="/branchcut-icon.svg" alt="" width={88} height={48} />
        <p className="section-kicker">READY TO CUT THE TREE?</p>
        <h2>Start with one query.</h2>
        <div className="hero-actions">
          <Link className="primary-action" href="/docs/getting-started/quick-start">Quick start <span>→</span></Link>
          <Link className="secondary-action" href="/docs/reference/cli">CLI reference</Link>
        </div>
      </section>
    </main>
  );
}
