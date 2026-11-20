import Image from 'next/image';
import Link from 'next/link';
import { ArrowRight, GitBranch } from 'lucide-react';
import { BenchmarkLab } from '@/components/benchmark-lab';
import { InstallTabs } from '@/components/install-tabs';
import { ScrollReveal } from '@/components/scroll-reveal';
import { WorkflowTabs } from '@/components/workflow-tabs';
import { withBasePath } from '@/lib/base-path';

const repositoryUrl = 'https://github.com/codex-mohan/branchcut';

export default function HomePage() {
  return (
    <main className="bc-landing">
      <section className="bc-hero" aria-labelledby="hero-title">
        <div className="bc-hero-copy">
          <h1 id="hero-title">
            Find the files.
            <span>Skip the work.</span>
          </h1>
          <p>
            Branchcut compiles globs, exclusions, ignore rules, filters, and
            limits into one traversal plan. It opens the directories that can
            contribute, streams native paths, and shows you the work it avoided.
          </p>
          <div className="bc-hero-actions">
            <Link className="bc-primary-action" href="/docs/getting-started/installation">
              Install Branchcut <ArrowRight aria-hidden="true" />
            </Link>
            <Link className="bc-secondary-action" href="/docs/getting-started/quick-start">
              Read the quick start
            </Link>
          </div>
        </div>

        <div className="bc-install-stage">
          <div className="bc-install-window">
            <div className="bc-install-window-bar">
              <span className="bc-window-controls" aria-hidden="true"><i /><i /><i /></span>
              <span>install · branchcut</span>
              <b>ready</b>
            </div>
            <InstallTabs />
            <div className="bc-install-facts">
              <span>Rust 2024</span>
              <span>Windows · Linux · macOS</span>
              <span>Zero core crates</span>
            </div>
          </div>
        </div>
      </section>

      <section className="bc-performance-section" id="performance" aria-label="Performance comparison">
        <BenchmarkLab />
      </section>

      <section className="bc-feature-section" id="features" aria-label="Feature showcase">
        <WorkflowTabs />
      </section>

      <section className="bc-boundary-section" aria-labelledby="boundary-title">
        <div className="bc-boundary-intro" data-reveal="left">
          <h2 id="boundary-title">Useful, with the boundary drawn.</h2>
          <p>
            Branchcut is a filesystem query engine. It compiles globs,
            exclusions, ignores, filters, and limits into one traversal plan.
            Opt-in literal content search with <code>--content</code> lets you
            find text in the files that plan selects, while keeping content
            reads out of ordinary path queries. Branchcut is not an index or a
            universal replacement for every glob API. The supported surface is
            documented and tested.
          </p>
          <Link href="/docs/reference/feature-status">
            Complete feature status <ArrowRight aria-hidden="true" />
          </Link>
        </div>

        <div className="bc-boundary-table" role="table" aria-label="Branchcut boundaries" data-reveal="right">
          <div role="row" className="bc-boundary-row bc-boundary-header">
            <span role="columnheader">Built for</span>
            <span role="columnheader">Branchcut</span>
            <span role="columnheader">Use another tool when</span>
          </div>
          <div role="row" className="bc-boundary-row">
            <span role="cell">Filename selection</span>
            <strong role="cell">Globs, literals, type, extension, depth, size, time</strong>
            <span role="cell">You already have a mature ecosystem-specific glob API</span>
          </div>
          <div role="row" className="bc-boundary-row">
            <span role="cell">Ignored trees</span>
            <strong role="cell">Nested, repository and global ignores; explicit override files</strong>
            <span role="cell">You need every Git config condition or environment override</span>
          </div>
          <div role="row" className="bc-boundary-row">
            <span role="cell">Automation</span>
            <strong role="cell">Native paths, JSON Lines, NUL output, shell-free commands</strong>
            <span role="cell">You need a shell interpreter rather than explicit argv</span>
          </div>
          <div role="row" className="bc-boundary-row">
            <span role="cell">Content or structure</span>
            <strong role="cell">Filesystem metadata and path semantics</strong>
            <span role="cell">You need text search, parsing, indexing, or a service protocol</span>
          </div>
        </div>
      </section>

      <section className="bc-close" aria-labelledby="close-title">
        <div className="bc-close-mark" aria-hidden="true">
          <Image
            src={withBasePath('/branchcut-icon.svg')}
            alt=""
            width={88}
            height={48}
          />
        </div>
        <div>
          <h2 id="close-title">Point Branchcut at the tree.</h2>
          <p>Start with one filename or compile the whole query.</p>
        </div>
        <div className="bc-close-actions">
          <Link className="bc-primary-action" href="/docs/getting-started/installation">
            Install <ArrowRight aria-hidden="true" />
          </Link>
          <a className="bc-secondary-action" href={repositoryUrl}>
            <GitBranch aria-hidden="true" /> Source
          </a>
        </div>
      </section>

      <footer className="bc-footer">
        <div className="bc-footer-brand">
          <Image src={withBasePath('/branchcut-icon.svg')} alt="" width={88} height={48} />
          <strong>Branchcut</strong>
          <p>Compile the query. Cut the tree.</p>
        </div>
        <nav aria-label="Documentation">
          <strong>Learn</strong>
          <Link href="/docs/getting-started/installation">Installation</Link>
          <Link href="/docs/getting-started/quick-start">Quick start</Link>
          <Link href="/docs/reference/cli">CLI reference</Link>
          <Link href="/docs/reference/glob-syntax">Glob syntax</Link>
        </nav>
        <nav aria-label="Project">
          <strong>Project</strong>
          <a href={repositoryUrl}>Source on GitHub</a>
          <a href={`${repositoryUrl}/issues`}>Report an issue</a>
          <Link href="/docs/evidence/cli-comparison">Benchmarks</Link>
          <a href={`${repositoryUrl}/blob/master/LICENSE`}>MIT license</a>
        </nav>
        <p className="bc-footer-note">
          Core engine and ordinary CLI: Rust standard library only. Optional
          integrations may carry dependencies without entering the core graph.
        </p>
      </footer>

      <ScrollReveal />
    </main>
  );
}
