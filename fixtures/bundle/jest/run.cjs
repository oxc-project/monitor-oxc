const { runCLI } = require("jest");

runCLI(
  {
    rootDir: "fixtures/bundle/jest",
    runInBand: true,
    silent: true,
    cache: false,
    reporters: ["<rootDir>/reporter.cjs"],
  },
  ["fixtures/bundle/jest"],
).then(({ results }) => {
  process.exit(results.success ? 0 : 1);
});
