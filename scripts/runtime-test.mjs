// Differential runtime harness for `runtime-repos.json` entries.
//
// Usage: node scripts/runtime-test.mjs <install|baseline|test> <name> <dir> [config]
//
//   install  — run the repo's install command.
//   baseline — run the repo's test suite once (untouched sources).
//   test     — classify a minified run:
//                minified green                -> exit 0
//                red once, green on rerun      -> exit 0 (flake)
//                red twice, baseline green     -> exit 1 (oxc bug)
//                red twice, baseline red       -> exit 0 + ::warning:: (rot)
import { spawnSync } from "node:child_process";
import fs from "node:fs";

const [mode, name, dir, configPath = "runtime-repos.json"] = process.argv.slice(2);

if (!["install", "baseline", "test"].includes(mode) || !name || !dir) {
  console.error(
    "usage: node scripts/runtime-test.mjs <install|baseline|test> <name> <dir> [config]",
  );
  process.exit(2);
}

const repos = JSON.parse(fs.readFileSync(configPath, "utf8"));
const repo = repos.find((r) => r.name === name);
if (!repo) {
  console.error(`no repo named \`${name}\` in ${configPath}`);
  process.exit(2);
}

// Pin `@oxc-project/runtime` to the version resolved in monitor-oxc's own
// committed lockfile (kept fresh by renovate), so install commands don't
// float on the registry's `latest` tag: a publish between the weekly bump
// run and a later CI run would otherwise change this oracle's inputs.
const runtimeVersionMatch = fs
  .readFileSync("pnpm-lock.yaml", "utf8")
  .match(/'@oxc-project\/runtime':\s*\n\s*specifier:[^\n]*\n\s*version:\s*(\S+)/);
if (!runtimeVersionMatch) {
  console.error("could not resolve @oxc-project/runtime from pnpm-lock.yaml");
  process.exit(2);
}

function run(command) {
  console.log(`$ ${command}`);
  const { status } = spawnSync(command, {
    cwd: dir,
    shell: true,
    stdio: "inherit",
    env: {
      ...process.env,
      // Lets install/test commands reference committed fixtures (e.g.
      // "$MONITOR_ROOT/fixtures/runtime/...") regardless of where the clone is.
      MONITOR_ROOT: process.cwd(),
      OXC_RUNTIME_VERSION: runtimeVersionMatch[1],
    },
  });
  return status === 0;
}

switch (mode) {
  case "install":
    process.exit(run(repo.install) ? 0 : 1);
  case "baseline":
    process.exit(run(repo.test) ? 0 : 1);
  case "test": {
    if (run(repo.test)) process.exit(0);
    console.log("Minified suite failed once; rerunning to rule out a flake.");
    if (run(repo.test)) process.exit(0);
    console.log("Restoring original sources for the baseline run.");
    if (!run("git checkout -- .")) process.exit(2);
    if (run(repo.test)) {
      console.log(
        `Baseline passes but the minified suite fails: minifier/transformer bug (${name}).`,
      );
      process.exit(1);
    }
    console.log(
      `::warning::Baseline suite failed for ${name} — upstream flake or environment rot, not an oxc bug.`,
    );
    process.exit(0);
  }
}
