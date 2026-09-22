// Tauri doesn't have a Node.js server to do proper SSR
// so we use adapter-static with a fallback to index.html to put the site in SPA mode
// See: https://svelte.dev/docs/kit/single-page-apps
// See: https://v2.tauri.app/start/frontend/sveltekit/ for more info
import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    paths: {
      base:
        process.env.SAVE_BUILD_TARGET === "desktop"
          ? ""
          : process.env.BASE_PATH || "",
    },
    files: {
      // The generated wasm package is a browser asset. Keeping a separate,
      // minimal desktop asset directory prevents it from being embedded in Tauri.
      assets:
        process.env.SAVE_BUILD_TARGET === "desktop"
          ? "desktop-static"
          : "static",
    },
    adapter: adapter({
      pages:
        process.env.SAVE_BUILD_TARGET === "desktop"
          ? "build/desktop"
          : "build/web",
      assets:
        process.env.SAVE_BUILD_TARGET === "desktop"
          ? "build/desktop"
          : "build/web",
      fallback: "index.html",
    }),
  },
};

export default config;
