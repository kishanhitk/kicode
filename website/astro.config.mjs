import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import tailwind from '@astrojs/tailwind';
import vercel from '@astrojs/vercel';

export default defineConfig({
  site: 'https://kicode.dev',
  output: 'static',
  adapter: vercel(),
  integrations: [
    starlight({
      title: 'kicode',
      components: {
        SiteTitle: './src/components/overrides/SiteTitle.astro',
      },
      description: 'AI-powered coding assistant for your terminal',
      social: {
        github: 'https://github.com/kishanhitk/kicode',
      },
      customCss: ['./src/styles/global.css'],
      sidebar: [
        {
          label: 'Getting Started',
          items: [
            { label: 'Installation', slug: 'getting-started/installation' },
            { label: 'Configuration', slug: 'getting-started/configuration' },
            { label: 'First Session', slug: 'getting-started/first-session' },
          ],
        },
        {
          label: 'Tools',
          items: [
            { label: 'Overview', slug: 'tools' },
            { label: 'Read File', slug: 'tools/read-file' },
            { label: 'Write File', slug: 'tools/write-file' },
            { label: 'Edit File', slug: 'tools/edit-file' },
            { label: 'Shell', slug: 'tools/shell' },
            { label: 'Grep', slug: 'tools/grep' },
            { label: 'Glob Search', slug: 'tools/glob-search' },
          ],
        },
        {
          label: 'Reference',
          items: [
            { label: 'Configuration', slug: 'reference/configuration' },
            { label: 'Models', slug: 'reference/models' },
            { label: 'Safety', slug: 'reference/safety' },
            { label: 'Environment', slug: 'reference/environment' },
          ],
        },
      ],
      head: [
        {
          tag: 'script',
          content: `
            // Force dark mode for terminal theme consistency
            document.documentElement.dataset.theme = 'dark';
            localStorage.setItem('starlight-theme', 'dark');
          `,
        },
        {
          tag: 'link',
          attrs: {
            rel: 'preconnect',
            href: 'https://fonts.googleapis.com',
          },
        },
        {
          tag: 'link',
          attrs: {
            rel: 'preconnect',
            href: 'https://fonts.gstatic.com',
            crossorigin: true,
          },
        },
        {
          tag: 'link',
          attrs: {
            href: 'https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500;600&display=swap',
            rel: 'stylesheet',
          },
        },
      ],
    }),
    tailwind({ applyBaseStyles: false }),
  ],
});
