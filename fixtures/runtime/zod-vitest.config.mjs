import { resolve } from "node:path";
import { defineConfig } from "vitest/config";
const c = ["@zod/source", "default"];
export default defineConfig({
  resolve: { conditions: c, externalConditions: c },
  ssr: { resolve: { conditions: c, externalConditions: c } },
  test: {
    name: "zod",
    root: resolve(process.cwd(), "packages/zod"),
    include: ["src/**/*.test.ts"],
    watch: false,
    isolate: true,
    setupFiles: [resolve(process.cwd(), "scripts/fail-on-console.ts")],
    typecheck: { enabled: false },
    silent: true,
  },
});
