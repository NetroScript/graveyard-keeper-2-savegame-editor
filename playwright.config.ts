import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "tests/browser",
  workers: 1,
  timeout: 60_000,
  use: {
    baseURL: "http://127.0.0.1:4189",
    // Default to Playwright's portable Chromium; an installed channel is an optional override.
    channel: process.env.PLAYWRIGHT_CHANNEL,
    headless: true,
  },
  webServer: {
    command: "pnpm preview --host 127.0.0.1 --port 4189 --strictPort",
    url: "http://127.0.0.1:4189",
    reuseExistingServer: false,
  },
});
