import type { BaseLayoutProps } from 'fumadocs-ui/layouts/shared';
import Image from 'next/image';
import { withBasePath } from './base-path';
import { appName, gitConfig } from './shared';

export function baseOptions(): BaseLayoutProps {
  return {
    nav: {
      title: (
        <span className="branchcut-lockup">
          <Image src={withBasePath('/branchcut-icon.svg')} alt="" width={88} height={48} />
          <span>{appName}</span>
        </span>
      ),
      transparentMode: 'top',
    },
    links: [
      { text: 'Docs', url: '/docs', active: 'nested-url' },
      { text: 'Tutorials', url: '/docs/tutorials/first-query' },
      { text: 'Cheat sheet', url: '/docs/reference/cheatsheet' },
      { text: 'Benchmarks', url: '/docs/evidence/cli-comparison', active: 'url' },
      { text: 'Roadmap', url: '/docs/project/roadmap', active: 'url' },
    ],
    githubUrl: `https://github.com/${gitConfig.user}/${gitConfig.repo}`,
  };
}
