import { Analytics } from '@vercel/analytics/next';
import { RootProvider } from 'fumadocs-ui/provider/next';
import type { Metadata } from 'next';
import './global.css';

export const metadata: Metadata = {
  title: {
    default: 'Branchcut — compile the query, cut the tree',
    template: '%s · Branchcut',
  },
  description:
    'A single-file, zero-crate Rust filesystem query engine that compiles queries into pruned traversal plans.',
  metadataBase: new URL(process.env.NEXT_PUBLIC_SITE_URL ?? 'http://localhost:3000'),
  openGraph: {
    type: 'website',
    title: 'Branchcut — compile the query, cut the tree',
    description:
      'A single-file, zero-crate Rust filesystem query engine that plans before it walks.',
    images: ['/og.png'],
  },
  twitter: {
    card: 'summary_large_image',
    title: 'Branchcut — compile the query, cut the tree',
    description:
      'A single-file, zero-crate Rust filesystem query engine that plans before it walks.',
    images: ['/og.png'],
  },
};

export default function Layout({ children }: LayoutProps<'/'>) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body className="flex flex-col min-h-screen">
        <RootProvider>{children}</RootProvider>
        <Analytics />
      </body>
    </html>
  );
}
