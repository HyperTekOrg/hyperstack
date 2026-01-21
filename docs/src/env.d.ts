/// <reference path="../.astro/types.d.ts" />
/// <reference types="astro/client" />
/// <reference types="@astrojs/starlight/virtual.d.ts" />
/// <reference types="@astrojs/starlight/virtual-internal.d.ts" />
/// <reference types="@astrojs/starlight/locals.d.ts" />

declare module "@pagefind/default-ui" {
  export class PagefindUI {
    constructor(options: {
      element?: string;
      baseUrl?: string;
      bundlePath?: string;
      showImages?: boolean;
      translations?: Record<string, string>;
      showSubResults?: boolean;
      processResult?: (result: {
        url: string;
        sub_results: Array<{ url: string }>;
      }) => void;
      [key: string]: unknown;
    });
  }
}
