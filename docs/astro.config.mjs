import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
  integrations: [
    starlight({
      title: 'Beans',
      logo: { src: './src/assets/logo.png' },
      sidebar: [{ label: 'Getting Started', slug: 'getting-started' }],
      social: [
        { icon: 'github', label: 'GitHub', href: 'https://github.com/hegyibalint/beans' },
      ],
    }),
  ],
});
