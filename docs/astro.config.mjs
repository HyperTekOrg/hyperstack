import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
  integrations: [
    starlight({
      title: 'Hyperstack',
      pagefind: false,
      customCss: ['./src/styles/custom.css'],
    }),
  ],
});
