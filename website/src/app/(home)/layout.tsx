import { HomeLayout } from 'fumadocs-ui/layouts/home';
import { baseOptions } from '@/lib/layout.shared';

export default function Layout({ children }: LayoutProps<'/'>) {
  return (
    <div className="bc-home-shell">
      <HomeLayout {...baseOptions()}>{children}</HomeLayout>
    </div>
  );
}
