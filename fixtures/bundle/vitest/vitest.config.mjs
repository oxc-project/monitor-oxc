import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    watch: false,
    fileParallelism: false,
    coverage: { enabled: false },
  },
});
