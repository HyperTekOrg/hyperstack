import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";

export default defineConfig({
  integrations: [
    starlight({
      title: "Hyperstack",
      social: [
        {
          icon: "github",
          label: "GitHub",
          href: "https://github.com/HyperTekOrg/hyperstack",
        },
      ],
      customCss: ["./src/styles/custom.css"],
      // Component overrides for custom design
      components: {
        Sidebar: "./src/components/overrides/Sidebar.astro",
        EditLink: "./src/components/overrides/EditLink.astro",
      },
      // Autogenerate sidebar from directory structure
      // Contributors only need to add frontmatter to control ordering
      sidebar: [
        {
          label: "Using Stacks",
          autogenerate: { directory: "using-stacks" },
        },
        {
          label: "Building Stacks",
          autogenerate: { directory: "building-stacks" },
        },
        {
          label: "Concepts",
          autogenerate: { directory: "concepts" },
        },
        {
          label: "SDKs",
          items: [
            {
              label: "TypeScript",
              autogenerate: { directory: "sdks/typescript" },
            },
            {
              label: "Rust",
              autogenerate: { directory: "sdks/rust" },
            },
          ],
        },
        {
          label: "CLI",
          autogenerate: { directory: "cli" },
        },
        {
          label: "Self-Hosting",
          autogenerate: { directory: "self-hosting" },
        },
      ],
      // Enable search when content is ready
      pagefind: true,
    }),
  ],
});
