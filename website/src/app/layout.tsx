import { Analytics } from '@vercel/analytics/next';
import { RootProvider } from 'fumadocs-ui/provider/next';
import type { Metadata } from 'next';
import { Archivo, Fragment_Mono } from 'next/font/google';
import './global.css';

const branchcutSans = Archivo({
  variable: '--font-branchcut',
  subsets: ['latin'],
  display: 'swap',
});

const branchcutMono = Fragment_Mono({
  variable: '--font-branchcut-mono',
  subsets: ['latin'],
  weight: '400',
  display: 'swap',
});

const basePath = (process.env.NEXT_PUBLIC_BASE_PATH ?? '').replace(/\/$/, '');
const searchApi = `${basePath}/api/search`;

export const metadata: Metadata = {
  title: {
    default: 'Branchcut — compile the query, cut the tree',
    template: '%s · Branchcut',
  },
  description:
    'A modular, zero-crate-core Rust filesystem query engine that compiles queries into pruned traversal plans.',
  metadataBase: new URL(process.env.NEXT_PUBLIC_SITE_URL ?? 'http://localhost:3000'),
  openGraph: {
    type: 'website',
    title: 'Branchcut — compile the query, cut the tree',
    description:
      'A modular, zero-crate-core Rust filesystem query engine that plans before it walks.',
    images: ['/og.png'],
  },
  twitter: {
    card: 'summary_large_image',
    title: 'Branchcut — compile the query, cut the tree',
    description:
      'A modular, zero-crate-core Rust filesystem query engine that plans before it walks.',
    images: ['/og.png'],
  },
};

export default function Layout({ children }: LayoutProps<'/'>) {
  return (
    <html lang="en" className="dark" suppressHydrationWarning>
      <head>
        <script
          dangerouslySetInnerHTML={{
            __html: "document.documentElement.classList.add('bc-js')",
          }}
        />
      </head>
      <body className={`${branchcutSans.variable} ${branchcutMono.variable} flex min-h-screen flex-col`}>
        <RootProvider
          search={{
            options: {
              type: 'static',
              api: searchApi,
            },
          }}
          theme={{
            defaultTheme: 'dark',
            enableSystem: false,
          }}
        >
          {children}
        </RootProvider>
        <Analytics />
      </body>
    </html>
  );
}
