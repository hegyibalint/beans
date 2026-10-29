import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
  site: 'https://hegyibalint.github.io',
  base: '/beans',
  integrations: [
    starlight({
      title: 'Beans',
      favicon: '/favicon.png',
      logo: { src: './src/assets/logo.svg' },
      customCss: ['./src/styles/theme.css', './src/styles/sidebar.css'],
      sidebar: [
        {
          label: 'Getting Started',
          collapsed: false,
          items: [
            { label: 'Overview', slug: '' },
            { label: 'Installation', slug: 'getting-started' },
          ],
        },
        {
          label: 'LSP',
          collapsed: false,
          items: [{ label: 'Editor Setup', slug: 'lsp/editor-setup' }],
        },
        {
          label: 'CLI',
          collapsed: false,
          items: [{ label: 'Commands', slug: 'cli/commands' }],
        },
        { label: 'Developing', collapsed: false, items: [] },
      ],
      social: [
        { icon: 'github', label: 'GitHub', href: 'https://github.com/hegyibalint/beans' },
      ],
    }),
  ],
});
