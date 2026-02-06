import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import starlightLlmsTxt from "starlight-llms-txt";
import { ecVersionPlugin } from "./src/plugins/ec-version-plugin.mjs";
import { remarkVersion } from "./src/plugins/remark-version.mjs";

export default defineConfig({
  markdown: {
    remarkPlugins: [remarkVersion],
  },
  site: "https://docs.usehyperstack.com",
  integrations: [
    starlight({
      expressiveCode: {
        plugins: [ecVersionPlugin()],
      },
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
        PageTitle: "./src/components/overrides/PageTitle.astro",
      },
      // Autogenerate sidebar from directory structure
      // Contributors only need to add frontmatter to control ordering
      sidebar: [
        {
          label: "Get Started",
          autogenerate: { directory: "using-stacks" },
        },
        {
          label: "Building Stacks",
          items: [
            { slug: "building-stacks/workflow" },
            { slug: "building-stacks/stack-definitions" },
            { slug: "building-stacks/installation" },
            { slug: "building-stacks/configuration" },
            { slug: "building-stacks/your-first-stack" },
            {
              label: "Rust DSL",
              items: [
                { slug: "building-stacks/rust-dsl/overview" },
                { slug: "building-stacks/rust-dsl/macros" },
                { slug: "building-stacks/rust-dsl/strategies" },
                { slug: "building-stacks/rust-dsl/resolvers" },
              ],
            },
          ],
        },

        {
          label: "SDK Reference",
          items: [
            {
              label: "TypeScript",
              link: "/sdks/typescript/",
            },
            {
              label: "React",
              link: "/sdks/react/",
            },
            {
              label: "Rust",
              link: "/sdks/rust/",
            },
            {
              label: "Schema Validation",
              link: "/sdks/validation/",
            },
          ],
        },
        {
          label: "CLI",
          autogenerate: { directory: "cli" },
        },
        {
          label: "Server",
          autogenerate: { directory: "hyperstack-server" },
        },
      ],
      // Enable search when content is ready
      pagefind: true,
    }),
  ],
});
