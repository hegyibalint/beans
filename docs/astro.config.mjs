import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
  site: 'https://hegyibalint.github.io',
  base: '/beans',
  integrations: [
    starlight({
      title: 'Beans',
      favicon: '/favicon.png',
      logo: { src: './src/assets/logo.png' },
      sidebar: [{ label: 'Getting Started', slug: 'getting-started' }],
      social: [
        { icon: 'github', label: 'GitHub', href: 'https://github.com/hegyibalint/beans' },
      ],
    }),
  ],
});
