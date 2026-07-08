// Differential oracle over bundled CLI tools (see
// docs/superpowers/specs/2026-07-07-bundle-diff-oracle-design.md).
//
// Usage: node scripts/bundle-diff-test.mjs [--config <path>] [name ...]
//
// Per tool: run twice unminified (determinism gate), minify the bundle in
// place via `runtime-minify`, run again, byte-diff stdout/stderr/exit code
// and OUTDIR files, then restore the original bundle. Any diff = minifier
// bug (exit 1). Nondeterministic tool = ::warning:: + skip.
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import zlib from "node:zlib";

const args = process.argv.slice(2);
let configPath = "bundle-tools.json";
const names = [];
for (let i = 0; i < args.length; i++) {
  if (args[i] === "--config") {
    configPath = args[++i];
    if (!configPath) usage();
  } else {
    names.push(args[i]);
  }
}

function usage() {
  console.error("usage: node scripts/bundle-diff-test.mjs [--config <path>] [name ...]");
  process.exit(2);
}

const bin = process.env.MONITOR_OXC_BIN ?? "./monitor-oxc";
let tools;
try {
  tools = JSON.parse(fs.readFileSync(configPath, "utf8"));
} catch (err) {
  console.error(`failed to read ${configPath}: ${err.message}`);
  process.exit(2);
}
if (!Array.isArray(tools)) {
  console.error(`${configPath} must contain a JSON array of tool entries`);
  process.exit(2);
}
const selected = names.length === 0 ? tools : names.map((name) => {
  const tool = tools.find((t) => t.name === name);
  if (!tool) {
    console.error(`no tool named \`${name}\` in ${configPath}`);
    process.exit(2);
  }
  return tool;
});

// Minimal glob for the `sources` patterns we use (`a/b.js`, `dir/*.js`,
// `dir/**/*.js`) — avoids fs.globSync's experimental warning.
function globMatch(pattern, rel) {
  const re = new RegExp(
    "^" +
      pattern
        .split(/(\*\*\/|\*\*|\*)/)
        .map((part) => {
          if (part === "**/") return "(?:.*/)?";
          if (part === "**") return ".*";
          if (part === "*") return "[^/]*";
          return part.replace(/[.+?^${}()|[\]\\]/g, "\\$&");
        })
        .join("") +
      "$",
  );
  return re.test(rel);
}

function listFiles(dir) {
  const out = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true, recursive: true })) {
    if (entry.isFile()) {
      out.push(path.relative(dir, path.join(entry.parentPath, entry.name)));
    }
  }
  return out;
}

function bundleFiles(tool) {
  if (!fs.existsSync(tool.dir)) {
    console.error(`${tool.name}: dir not found: ${tool.dir}`);
    process.exit(2);
  }
  const rels = listFiles(tool.dir).filter((rel) =>
    tool.sources.some((pattern) => globMatch(pattern, rel.replaceAll("\\", "/"))),
  );
  if (rels.length === 0) {
    console.error(`${tool.name}: sources matched no files under ${tool.dir}`);
    process.exit(2);
  }
  return rels.map((rel) => path.join(tool.dir, rel));
}

function runTool(tool) {
  const outdir = fs.mkdtempSync(path.join(os.tmpdir(), "bundle-diff-"));
  const command = tool.run.replaceAll("OUTDIR", outdir);
  const { status, stdout, stderr } = spawnSync(command, {
    shell: true,
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  });
  const files = {};
  for (const rel of listFiles(outdir).sort()) {
    files[rel] = fs.readFileSync(path.join(outdir, rel), "utf8");
  }
  fs.rmSync(outdir, { recursive: true, force: true });
  return { status, stdout, stderr, files };
}

function firstDivergence(label, expected, actual) {
  if (expected === actual) return null;
  const e = expected.split("\n");
  const a = actual.split("\n");
  for (let i = 0; i < Math.max(e.length, a.length); i++) {
    if (e[i] !== a[i]) {
      return `${label} differs at line ${i + 1}:\n  expected: ${e[i] ?? "<eof>"}\n  actual:   ${a[i] ?? "<eof>"}`;
    }
  }
  return `${label} differs`;
}

function compare(expected, actual) {
  const diffs = [];
  if (expected.status !== actual.status) {
    diffs.push(`exit code: expected ${expected.status}, actual ${actual.status}`);
  }
  for (const stream of ["stdout", "stderr"]) {
    const d = firstDivergence(stream, expected[stream], actual[stream]);
    if (d) diffs.push(d);
  }
  const keys = new Set([...Object.keys(expected.files), ...Object.keys(actual.files)]);
  for (const key of [...keys].sort()) {
    if (!(key in actual.files)) {
      diffs.push(`output file missing after minify: ${key}`);
    } else if (!(key in expected.files)) {
      diffs.push(`unexpected output file after minify: ${key}`);
    } else {
      const d = firstDivergence(`file ${key}`, expected.files[key], actual.files[key]);
      if (d) diffs.push(d);
    }
  }
  return diffs;
}

function bundleSizes(files) {
  let raw = 0;
  let gzip = 0;
  for (const file of files) {
    const buf = fs.readFileSync(file);
    raw += buf.length;
    gzip += zlib.gzipSync(buf, { level: 9 }).length;
  }
  return { raw, gzip };
}

function sizeLine(before, after) {
  const n = (v) => v.toLocaleString("en-US");
  const pct = (a, b) => `${(((b - a) / a) * 100).toFixed(1)}%`;
  return (
    `raw ${n(before.raw)} -> ${n(after.raw)} bytes (${pct(before.raw, after.raw)}), ` +
    `gzip ${n(before.gzip)} -> ${n(after.gzip)} bytes (${pct(before.gzip, after.gzip)})`
  );
}

function saveArtifacts(tool, expected, actual, minifiedPaths) {
  const dir = path.join("bundle-diff-artifacts", tool.name);
  fs.rmSync(dir, { recursive: true, force: true });
  fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(path.join(dir, "expected-stdout.txt"), expected.stdout);
  fs.writeFileSync(path.join(dir, "actual-stdout.txt"), actual.stdout);
  fs.writeFileSync(path.join(dir, "expected-stderr.txt"), expected.stderr);
  fs.writeFileSync(path.join(dir, "actual-stderr.txt"), actual.stderr);
  for (const [rel, content] of Object.entries(expected.files)) {
    const p = path.join(dir, "expected-files", rel);
    fs.mkdirSync(path.dirname(p), { recursive: true });
    fs.writeFileSync(p, content);
  }
  for (const [rel, content] of Object.entries(actual.files)) {
    const p = path.join(dir, "actual-files", rel);
    fs.mkdirSync(path.dirname(p), { recursive: true });
    fs.writeFileSync(p, content);
  }
  for (const file of minifiedPaths) {
    const p = path.join(dir, "minified", path.relative(tool.dir, file));
    fs.mkdirSync(path.dirname(p), { recursive: true });
    fs.copyFileSync(file, p);
  }
  console.log(`${tool.name}: artifacts saved to ${dir}`);
}

let failed = false;

for (const tool of selected) {
  console.log(`=== ${tool.name}`);
  const expected = runTool(tool);
  if (expected.status !== 0) {
    console.error(`${tool.name}: expected (unminified) run failed with exit ${expected.status}`);
    console.error(expected.stderr);
    process.exit(2);
  }
  const second = runTool(tool);
  const nondet = compare(expected, second);
  if (nondet.length > 0) {
    console.log(`::warning::${tool.name} is nondeterministic unminified — skipped, not an oxc bug`);
    for (const d of nondet) console.log(`  ${d}`);
    continue;
  }

  const files = bundleFiles(tool);
  const backups = files.map((file) => {
    const backup = `${file}.bundle-diff-backup`;
    fs.copyFileSync(file, backup);
    return [file, backup];
  });

  try {
    const minify = spawnSync(bin, ["runtime-minify", "--config", configPath, "--name", tool.name, "--dir", tool.dir], {
      stdio: "inherit",
    });
    if (minify.status !== 0) {
      console.error(
        `${tool.name}: runtime-minify failed (exit ${minify.status}${minify.error ? `, ${minify.error.message}` : ""})`,
      );
      failed = true;
      continue;
    }
    // Originals are already overwritten; measure "before" from the backups.
    console.log(
      `${tool.name}: ${sizeLine(bundleSizes(backups.map(([, backup]) => backup)), bundleSizes(files))}`,
    );
    const actual = runTool(tool);
    const diffs = compare(expected, actual);
    if (diffs.length > 0) {
      console.error(`${tool.name}: minified output DIFFERS — minifier/codegen bug`);
      for (const d of diffs) console.error(`  ${d}`);
      saveArtifacts(tool, expected, actual, files);
      failed = true;
    } else {
      console.log(
        `${tool.name}: OK (${Object.keys(expected.files).length} output files, stdout ${expected.stdout.length} bytes)`,
      );
    }
  } finally {
    for (const [file, backup] of backups) {
      fs.copyFileSync(backup, file);
      fs.rmSync(backup);
    }
  }
}

process.exit(failed ? 1 : 0);
