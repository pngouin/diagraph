import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "e2e",
  testMatch: "*.fuzz.ts",
  timeout: 0,
  reporter: "list",
  outputDir: "fuzz-failures/playwright",
  use: { browserName: "chromium", headless: true },
});
