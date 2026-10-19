import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

/** Svelte compiler options, shared by Vite and svelte-check. */
export default {
  preprocess: vitePreprocess(),
};
