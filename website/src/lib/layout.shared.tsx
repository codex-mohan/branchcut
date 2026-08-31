import type { BaseLayoutProps } from 'fumadocs-ui/layouts/shared';
import Image from 'next/image';
import { appName, gitConfig } from './shared';

export function baseOptions(): BaseLayoutProps {
  return {
    nav: {
      title: (
        <span className="branchcut-lockup">
          <Image src="/branchcut-icon.svg" alt="" width={88} height={48} />
          <span>{appName}</span>
        </span>
      ),
      transparentMode: 'top',
    },
    links: [
      { text: 'Docs', url: '/docs', active: 'nested-url' },
      { text: 'CLI reference', url: '/docs/reference/cli', active: 'url' },
      { text: 'Benchmarks', url: '/docs/evidence/benchmarks', active: 'url' },
    ],
    githubUrl: `https://github.com/${gitConfig.user}/${gitConfig.repo}`,
  };
}
