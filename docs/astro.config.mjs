import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import starlightLlmsTxt from "starlight-llms-txt";

export default defineConfig({
  site: "https://docs.usehyperstack.com",
  integrations: [
    starlight({
      plugins: [starlightLlmsTxt()],
      title: "Hyperstack",
      social: [
        {
          icon: "github",
          label: "GitHub",
          href: "https://github.com/HyperTekOrg/hyperstack",
        },
      ],
      customCss: ["./src/styles/custom.css"],
      // Component overrides for custom design and analytics
      components: {
        Sidebar: "./src/components/overrides/Sidebar.astro",
        EditLink: "./src/components/overrides/EditLink.astro",
        Head: "./src/components/overrides/Head.astro",
        Search: "./src/components/overrides/Search.astro",
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
