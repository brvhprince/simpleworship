import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vite";

// Set by `tauri dev` when running on a phone or emulator.
const host = process.env.TAURI_DEV_HOST;

/** Vite configuration, following the Tauri 2 recommendations. */
export default defineConfig({
  plugins: [svelte()],

  // Keep Rust compiler errors visible in the terminal.
  clearScreen: false,

  server: {
    // Tauri expects a fixed port and fails if it is taken.
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: {
      // Rust changes are handled by `tauri dev`, not Vite.
      ignored: ["**/src-tauri/**"],
    },
  },

  build: {
    // Modern webviews only: WKWebView (macOS), WebView2 (Windows), WebKitGTK (Linux).
    target: ["es2022", "safari15", "chrome110"],
  },
});
