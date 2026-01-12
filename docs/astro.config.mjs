import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
  integrations: [
    starlight({
      title: 'Hyperstack',
      social: [
        { icon: 'github', label: 'GitHub', href: 'https://github.com/HyperTekOrg/hyperstack' },
      ],
      customCss: ['./src/styles/custom.css'],
      // Component overrides for custom design
      components: {
        Sidebar: './src/components/overrides/Sidebar.astro',
      },
      // Autogenerate sidebar from directory structure
      // Contributors only need to add frontmatter to control ordering
      sidebar: [
        {
          label: 'Getting Started',
          autogenerate: { directory: 'getting-started' },
        },
        {
          label: 'Concepts',
          autogenerate: { directory: 'concepts' },
        },
        {
          label: 'CLI',
          autogenerate: { directory: 'cli' },
        },
      ],
      // Enable search when content is ready
      pagefind: true,
    }),
  ],
});
