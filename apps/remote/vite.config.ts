import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vite";

/**
 * The phone remote is built into `dist/` and served by SimpleWorship itself
 * (see `crates/sw-server`). While developing, `pnpm dev` here serves it on
 * port 1430 and passes the server's routes on to a running SimpleWorship
 * with the remote turned on.
 */
const simpleworship = "http://127.0.0.1:7270";

export default defineConfig({
  plugins: [svelte()],
  // Served from the root of SimpleWorship's server.
  base: "/",
  server: {
    port: 1430,
    host: true,
    proxy: {
      "/api": simpleworship,
      "/media": simpleworship,
      "/thumbnails": simpleworship,
      "/ws": { target: simpleworship.replace("http", "ws"), ws: true },
    },
  },
  build: {
    // Phones from the last few years: iOS 15+ Safari, Android Chrome.
    target: ["es2022", "safari15", "chrome100"],
  },
});
