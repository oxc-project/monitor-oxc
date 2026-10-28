// Turn two `monitor-oxc size` TSVs (merge-base vs PR head) into the
// `## Minify Size` markdown comment posted on oxc PRs, plus the full report
// for the run's summary page.
//
// Usage:
//   node scripts/size-compare.mjs --base base.tsv --head head.tsv \
//     --base-sha <sha> --head-sha <sha> --base-ref <branch> --head-ref <branch> --run-url <url> \
//     [--build-result <result> --base-ok <bool> --base-exit <code> --head-exit <code>] \
//     [--emitted <dir> --diffs-url <url>] [--full size-report.md] > size-comment.md
//   node scripts/size-compare.mjs --base base.tsv --head head.tsv --list <n> > paths.txt
//   node scripts/size-compare.mjs --base base.tsv --head head.tsv --emitted <dir> --write-diffs <dir>
//
// `--list` prints the full paths of the <n> files that shrank most and the <n>
// that grew most, for `monitor-oxc size-emit`. `--emitted` points at a
// directory holding `base/` and `head/` from `size-emit`; the report then
// inlines diffs of the top few files and `--write-diffs` writes one `.diff`
// per listed file (for an artifact). Diffs use the first view that differs:
// `compress` (pretty-printed, unmangled), then `minify` (reprinted, mangled),
// then `raw` (the measured output, split after `;`, `{` and `}`).
//
// When the build or a run failed (`--build-result`, `--base-ok`, the exit
// codes) or a TSV is missing, both outputs say why instead of showing sizes.
//
// The TSVs come from a job that ran the PR's own build, so they are treated
// as untrusted data: unknown statuses become ERR; paths and error text lose
// control characters (a lone CR ends a markdown line), are length-capped and
// only ever render inside code spans; npmx links are only built from plain
// package names, versions and paths; oversized TSVs are rejected.
//
// The comment (stdout) holds only the summary: new failures, the totals and a
// link to the run, whose summary page shows the full report (`--full`): top
// files and packages, output diffs and fixed files.
//
// TSV rows are `path \t input_bytes \t status \t output_bytes \t gzip_bytes \t
// brotli_bytes \t detail` with
// status OK | INVALID | ERR | PANIC (see src/size.rs). Only files that are OK
// on both sides count toward sizes; OK -> non-OK transitions are listed as new
// failures. Exit 0 whenever a report is printed: this job reports, it never
// gates. Exit 2 on usage errors.

import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { parseArgs } from "node:util";

const MARKER = "<!-- monitor-oxc-size -->";
// GitHub caps comments at 65,536 characters and step summaries at 1 MiB.
const MAX_COMMENT = 60_000;
const MAX_FULL = 900_000;
const COMMENT_FAILURES = 5;
const MINIFIED = /\.min\.[cm]?js$/;
const STATUSES = new Set(["OK", "INVALID", "ERR", "PANIC"]);
// Real TSVs are ~20 MB; anything far larger is not a measurement.
const MAX_TSV_BYTES = 256 * 1024 * 1024;
const LIST_COUNT = 20;
const DIFF_VIEWS = ["compress", "minify", "raw"];
const INLINE_DIFF_LINES = 30;
const INLINE_DIFF_WIDTH = 160;

const { values: args } = parseArgs({
  options: {
    base: { type: "string" },
    head: { type: "string" },
    "base-sha": { type: "string", default: "" },
    "head-sha": { type: "string", default: "" },
    "run-url": { type: "string", default: "" },
    "head-ref": { type: "string", default: "" },
    "base-ref": { type: "string", default: "" },
    "build-result": { type: "string" },
    "base-ok": { type: "string" },
    "base-exit": { type: "string" },
    "head-exit": { type: "string" },
    list: { type: "string" },
    emitted: { type: "string" },
    "write-diffs": { type: "string" },
    "diffs-url": { type: "string", default: "" },
    full: { type: "string" },
  },
});

if (!args.base || !args.head) {
  console.error("Pass --base and --head.");
  process.exit(2);
}

const fmt = (n) => n.toLocaleString("en-US");
const signed = (n) => (n > 0 ? `+${fmt(n)}` : fmt(n));
// Changes stay exact (small ones are the point); absolute sizes read as
// decimal units, like oxc's minsize snapshot.
const delta = (n) => `${signed(n)} B`;
function size(n) {
  if (n < 1000) return `${n} B`;
  const [value, unit] = n < 1e6 ? [n / 1e3, "kB"] : n < 1e9 ? [n / 1e6, "MB"] : [n / 1e9, "GB"];
  return `${value.toFixed(2)} ${unit}`;
}

const isSha = (sha) => /^[0-9a-f]{40}$/.test(sha ?? "");
const commitLink = (sha) =>
  isSha(sha) ? `[\`${sha.slice(0, 12)}\`](https://github.com/oxc-project/oxc/commit/${sha})` : "unknown";

// The branch name, linked to the exact commit measured (a branch moves after
// the run), or the short sha when the branch is unknown.
function side(ref, sha, label) {
  if (!ref) return `${label} ${commitLink(sha)}`;
  const name = `\`${ref.replaceAll("`", "'")}\``;
  return isSha(sha) ? `**[${name}](https://github.com/oxc-project/oxc/commit/${sha})**` : `**${name}**`;
}

// A full report's comment links the run through its "Full report" line
// instead; pass `summary` only then.
function header(summary = false) {
  const run = args["run-url"] && !summary ? ` · [run](${args["run-url"]})` : "";
  const head = side(args["head-ref"], args["head-sha"], "Head");
  const base = side(args["base-ref"], args["base-sha"], "merge-base");
  return `## Minify Size\n\n${head} vs ${base}${run}\n`;
}

// Why there are no sizes to report, or null when there are.
function unavailableReason() {
  // Exit codes come from the measuring job's outputs; accept only integers.
  const exitCode = (value) => (/^\d+$/.test(value ?? "") ? value : "");
  const baseExit = exitCode(args["base-exit"]);
  const headExit = exitCode(args["head-exit"]);
  if (args["build-result"] !== undefined && args["build-result"] !== "success") {
    return "Head build failed; no size report for this commit.";
  }
  if (args["base-ok"] !== undefined && args["base-ok"] !== "true") {
    const sha = isSha(args["base-sha"]) ? ` \`${args["base-sha"].slice(0, 12)}\`` : "";
    return `Baseline unavailable: merge-base${sha} does not build with current monitor-oxc. Rebasing the PR usually fixes this.`;
  }
  if (headExit && !baseExit) {
    return `**Head run crashed (exit ${headExit}) but base ran cleanly**, likely a stack overflow introduced by this PR. See the run log.`;
  }
  if (baseExit) return `Base run crashed (exit ${baseExit}); no size report.`;
  const usable = (file) => {
    try {
      const stat = fs.statSync(file);
      return stat.isFile() && stat.size <= MAX_TSV_BYTES;
    } catch {
      return false;
    }
  };
  if (!usable(args.base) || !usable(args.head)) return "The size measurement did not finish; see the run.";
  return null;
}

const reason = unavailableReason();
if (reason !== null) {
  if (args.list !== undefined || args["write-diffs"] !== undefined) process.exit(0);
  if (args.full) fs.writeFileSync(args.full, `${header()}\n${reason}\n`);
  process.stdout.write(`${header()}\n${reason}\n\n${MARKER}\n`);
  process.exit(0);
}

// `node_modules/.pnpm/<dir>/node_modules/<name>/<rest>` -> `<name>@<ver>`,
// where <dir> is `<name with / as +>@<ver>[_<peers>]`. Dropping the peer
// suffix folds identical peer-variant copies of a package together.
const PNPM_PATH = /node_modules\/\.pnpm\/([^/]+)\/node_modules\/((?:@[^/]+\/)?[^/]+)\/(.*)$/;

// npmx.dev pages for a package version and for one file in it. Encode each
// segment but keep the `@` of scoped names readable.
const NPMX = "https://npmx.dev";
const urlPath = (s) => s.split("/").map((part) => encodeURIComponent(part).replaceAll("%40", "@")).join("/");
// Link only names that cannot carry markdown (`)` would end the link early).
const PLAIN = /^[A-Za-z0-9._@+/-]+$/;

function splitPath(file) {
  const m = PNPM_PATH.exec(file);
  if (!m) return { pkg: file, key: file, pkgUrl: "", fileUrl: "" };
  const [, dir, name, rest] = m;
  const prefix = `${name.replace("/", "+")}@`;
  if (!dir.startsWith(prefix)) return { pkg: dir, key: `${dir}/${rest}`, pkgUrl: "", fileUrl: "" };
  const version = dir.slice(prefix.length).split("_")[0];
  const plainPkg = PLAIN.test(name) && PLAIN.test(version);
  return {
    pkg: `${name}@${version}`,
    key: `${name}@${version}/${rest}`,
    pkgUrl: plainPkg ? `${NPMX}/package/${urlPath(name)}/v/${urlPath(version)}` : "",
    fileUrl:
      plainPkg && PLAIN.test(rest)
        ? `${NPMX}/package-code/${urlPath(name)}/v/${urlPath(version)}/${urlPath(rest)}`
        : "",
  };
}

// Untrusted text: drop control characters (CR, NUL, ESC, ...) and line/paragraph
// separators, and cap the length.
const clean = (s, max) => s.replaceAll(/[\x00-\x1f\x7f\u2028\u2029]/g, " ").slice(0, max);

function readTsv(file) {
  const rows = new Map();
  let duplicates = 0;
  for (const line of fs.readFileSync(file, "utf8").split("\n")) {
    if (!line) continue;
    const fields = line.split("\t");
    const [, input, rawStatus, output, gzip, brotli] = fields;
    const file = clean(fields[0], 1000);
    const detail = clean(fields[6] ?? "", 300);
    const status = STATUSES.has(rawStatus) ? rawStatus : "ERR";
    const { pkg, key, pkgUrl, fileUrl } = splitPath(file);
    if (rows.has(key)) {
      duplicates++;
      continue;
    }
    rows.set(key, {
      key,
      pkg,
      file,
      pkgUrl,
      fileUrl,
      input: Number(input),
      status,
      output: Number(output),
      gzip: Number(gzip),
      brotli: Number(brotli),
      detail,
    });
  }
  return { rows, duplicates };
}

const base = readTsv(args.base);
const head = readTsv(args.head);

const compared = [];
const newFailures = [];
const fixed = [];
let onlyOneSide = 0;
for (const [key, b] of base.rows) {
  const h = head.rows.get(key);
  if (!h) {
    onlyOneSide++;
    continue;
  }
  if (b.status === "OK" && h.status === "OK") {
    compared.push({
      key,
      pkg: b.pkg,
      file: b.file,
      pkgUrl: b.pkgUrl,
      fileUrl: b.fileUrl,
      base: b.output,
      head: h.output,
      delta: h.output - b.output,
      // Compressed sizes only feed the totals: per file they swing with
      // unrelated byte shifts and would read as noise.
      gzip: [b.gzip, h.gzip],
      brotli: [b.brotli, h.brotli],
    });
  } else if (b.status === "OK") {
    newFailures.push({ key, fileUrl: h.fileUrl, status: h.status, detail: h.detail });
  } else if (h.status === "OK") {
    fixed.push({ key, fileUrl: b.fileUrl, status: b.status, detail: b.detail });
  }
}
for (const key of head.rows.keys()) if (!base.rows.has(key)) onlyOneSide++;

const changed = compared.filter((f) => f.delta !== 0);
const larger = changed.filter((f) => f.delta > 0).sort((a, b) => b.delta - a.delta);
const smaller = changed.filter((f) => f.delta < 0).sort((a, b) => a.delta - b.delta);

// The files `--list` hands to `size-emit`: the <n> that shrank most, then the
// <n> that grew most.
function listed(n) {
  return [...smaller.slice(0, n), ...larger.slice(0, n)];
}

const readOrNull = (file) => (fs.existsSync(file) ? fs.readFileSync(file, "utf8") : null);
const splitRaw = (code) => code.replaceAll(/([;{}])/g, "$1\n");

// Unified diff of one file's base vs head output, from the first view that
// differs. Returns `{ view, diff }` or null when no view has both sides.
function diffFile(f) {
  for (const view of DIFF_VIEWS) {
    let a = readOrNull(path.join(args.emitted, "base", view, f.file));
    let b = readOrNull(path.join(args.emitted, "head", view, f.file));
    if (a === null || b === null || a === b) continue;
    if (view === "raw") [a, b] = [splitRaw(a), splitRaw(b)];
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "size-diff-"));
    fs.writeFileSync(path.join(dir, "a"), a);
    fs.writeFileSync(path.join(dir, "b"), b);
    const { stdout } = spawnSync(
      "diff",
      ["-U2", "--label", `base/${f.key}`, "--label", `head/${f.key}`, path.join(dir, "a"), path.join(dir, "b")],
      { encoding: "utf8", maxBuffer: 256 * 1024 * 1024 },
    );
    fs.rmSync(dir, { recursive: true });
    if (stdout) return { view, diff: stdout };
  }
  return null;
}

if (args.list !== undefined) {
  for (const f of listed(Number(args.list))) console.log(f.file);
  process.exit(0);
}

if (args["write-diffs"] !== undefined) {
  if (!args.emitted) {
    console.error("--write-diffs needs --emitted.");
    process.exit(2);
  }
  const outDir = args["write-diffs"];
  fs.mkdirSync(outDir, { recursive: true });
  listed(LIST_COUNT).forEach((f, i) => {
    const result = diffFile(f);
    if (!result) return;
    // Stay well under the 255-byte file name limit on deep paths.
    const name = `${String(i + 1).padStart(2, "0")}-${f.key.replaceAll("/", "__").slice(-180)}.diff`;
    fs.writeFileSync(path.join(outDir, name), `# ${f.key}: ${delta(f.delta)}, ${result.view} view\n${result.diff}`);
  });
  process.exit(0);
}

const pct = (delta, total) => (total ? `${delta > 0 ? "+" : ""}${((delta / total) * 100).toFixed(3)}%` : "-");
// Table cells: keep the pipe from splitting the row and the backtick from
// closing the code span.
const code = (s) => `\`${s.replaceAll("`", "'").replaceAll("|", "\\|")}\``;
// A code-formatted name, linked to npmx when the path parsed as a package.
const link = (label, url) => (url ? `[${code(label)}](${url})` : code(label));
const htmlLink = (label, url) => {
  const html = `<code>${label.replaceAll("&", "&amp;").replaceAll("<", "&lt;")}</code>`;
  return url ? `<a href="${url}">${html}</a>` : html;
};
// ANSI escapes lost their ESC in `clean`; drop the leftover `[31m` codes.
const stripAnsi = (s) => s.replaceAll(/\[[0-9;]*m/g, "");

// One totals cell: the change, then base -> head underneath.
function sizeCell(baseBytes, headBytes) {
  const change = headBytes - baseBytes;
  return `${delta(change)} (${pct(change, baseBytes)})<br><sub>${size(baseBytes)} → ${size(headBytes)}</sub>`;
}

const sum = (files, pick) => files.reduce((total, f) => total + pick(f), 0);

function totalsRow(label, files) {
  const changed = files.filter((f) => f.delta !== 0);
  const smaller = changed.filter((f) => f.delta < 0).length;
  return [
    label,
    fmt(files.length),
    `${fmt(changed.length)}<br><sub>${fmt(smaller)} smaller, ${fmt(changed.length - smaller)} larger</sub>`,
    sizeCell(sum(files, (f) => f.base), sum(files, (f) => f.head)),
    sizeCell(sum(files, (f) => f.gzip[0]), sum(files, (f) => f.gzip[1])),
    sizeCell(sum(files, (f) => f.brotli[0]), sum(files, (f) => f.brotli[1])),
  ].join(" | ").replace(/^/, "| ").concat(" |");
}

function fileTable(files) {
  const rows = files.map((f) => `| ${link(f.key, f.fileUrl)} | ${size(f.base)} | ${size(f.head)} | ${delta(f.delta)} |`);
  return ["| file | base | head | Δ |", "|---|---:|---:|---:|", ...rows].join("\n");
}

function packageTable(pkgs) {
  const rows = pkgs.map((p) => `| ${link(p.pkg, p.pkgUrl)} | ${fmt(p.files)} | ${size(p.base)} | ${size(p.head)} | ${delta(p.delta)} |`);
  return ["| package | changed files | base | head | Δ |", "|---|---:|---:|---:|---:|", ...rows].join("\n");
}

function failureTable(files, side) {
  const rows = files.map((f) => `| ${link(f.key, f.fileUrl)} | ${f.status} | ${f.detail ? code(stripAnsi(f.detail)) : ""} |`);
  return [`| file | ${side} status | detail |`, "|---|---|---|", ...rows].join("\n");
}

const details = (summary, body) => `<details><summary>${summary}</summary>\n\n${body}\n\n</details>\n`;

// The first lines of a diff's hunks, shortened to fit a comment, in a fence
// longer than any backtick run inside it.
function inlineDiff(f, { view, diff }) {
  const lines = diff.split("\n").slice(2).filter((line) => line !== "");
  const shown = lines
    .slice(0, INLINE_DIFF_LINES)
    .map((line) => (line.length > INLINE_DIFF_WIDTH ? `${line.slice(0, INLINE_DIFF_WIDTH)} …` : line));
  const more = lines.length - shown.length;
  const body = shown.join("\n");
  const fence = "`".repeat(Math.max(3, ...(body.match(/`+/g) ?? []).map((run) => run.length + 1)));
  const tail = more > 0 ? `\n\n…${fmt(more)} more diff lines in the full diff.` : "";
  return details(`${htmlLink(f.key, f.fileUrl)} ${delta(f.delta)} (${view} view)`, `${fence}diff\n${body}\n${fence}${tail}`);
}

// `summary` renders the comment; otherwise the full report.
function render({ summary, topFiles, topPackages, maxFailures, inlineDiffs }) {
  const out = [header(summary)];
  const normal = compared.filter((f) => !MINIFIED.test(f.key));
  const minified = compared.filter((f) => MINIFIED.test(f.key));

  if (newFailures.length > 0) {
    const shown = newFailures.slice(0, maxFailures);
    const more = newFailures.length - shown.length;
    out.push(
      `> [!CAUTION]\n> **${fmt(newFailures.length)} file${newFailures.length === 1 ? "" : "s"} minified on base but not on head.** \`PANIC\` and \`INVALID\` (output no longer parses) are regressions.\n`,
      failureTable(shown, "head") +
        (more > 0 ? `\n\n…and ${fmt(more)} more in ${summary ? "the full report" : "the \`minify-size\` artifact"}.` : "") +
        "\n",
    );
  }

  if (changed.length === 0) {
    out.push(`No size change across ${fmt(compared.length)} files.\n`);
  } else {
    out.push(
      `**${fmt(changed.length)} of ${fmt(compared.length)} files changed.**\n`,
      "| | files | changed | raw | gzip -9 | brotli q5 |",
      "|---|---:|---:|---:|---:|---:|",
      totalsRow("normal", normal),
      totalsRow("already minified (`*.min.js`)", minified),
      "",
    );
  }

  if (summary) {
    if (fixed.length > 0) {
      const one = fixed.length === 1;
      out.push(`${fmt(fixed.length)} file${one ? "" : "s"} that failed on base now minif${one ? "ies" : "y"} on head.\n`);
    }
    if (args["run-url"]) {
      out.push(`**[Full report](${args["run-url"]})** (Test Minify Size summary): top files and packages each way, and before/after diffs of the files that changed most.\n`);
    }
  } else if (changed.length > 0) {
    if (smaller.length > 0) {
      out.push(details(`Top ${Math.min(topFiles, smaller.length)} smaller files (of ${fmt(smaller.length)})`, fileTable(smaller.slice(0, topFiles))));
    }
    if (larger.length > 0) {
      out.push(details(`Top ${Math.min(topFiles, larger.length)} larger files (of ${fmt(larger.length)})`, fileTable(larger.slice(0, topFiles))));
    }

    const byPkg = new Map();
    for (const f of changed) {
      const p = byPkg.get(f.pkg) ?? { pkg: f.pkg, pkgUrl: f.pkgUrl, files: 0, base: 0, head: 0, delta: 0 };
      p.files++;
      p.base += f.base;
      p.head += f.head;
      p.delta += f.delta;
      byPkg.set(f.pkg, p);
    }
    const pkgs = [...byPkg.values()];
    const pkgSmaller = pkgs.filter((p) => p.delta < 0).sort((a, b) => a.delta - b.delta).slice(0, topPackages);
    const pkgLarger = pkgs.filter((p) => p.delta > 0).sort((a, b) => b.delta - a.delta).slice(0, topPackages);
    if (pkgSmaller.length > 0) out.push(details(`Top ${pkgSmaller.length} smaller packages`, packageTable(pkgSmaller)));
    if (pkgLarger.length > 0) out.push(details(`Top ${pkgLarger.length} larger packages`, packageTable(pkgLarger)));

    if (args.emitted && inlineDiffs > 0) {
      const diffs = [...smaller.slice(0, inlineDiffs), ...larger.slice(0, inlineDiffs)]
        .map((f) => [f, diffCache(f)])
        .filter(([, result]) => result)
        .map(([f, result]) => inlineDiff(f, result));
      if (diffs.length > 0) {
        const full = args["diffs-url"]
          ? ` Full diffs for the top ${LIST_COUNT} files each way: [minify-size-diffs](${args["diffs-url"]}).`
          : "";
        out.push(
          `**Output diffs.** Pretty-printed before/after of the files that changed most.${full}\n`,
          ...diffs,
        );
      }
    }
  }

  if (!summary && fixed.length > 0) {
    const shown = fixed.slice(0, maxFailures);
    out.push(details(`${fmt(fixed.length)} file${fixed.length === 1 ? "" : "s"} failed on base but minify on head`, failureTable(shown, "base")));
  }

  const notes = [
    "Output bytes of oxc's full minify pipeline (compress + mangle + whitespace) over monitor-oxc's `node_modules` corpus; gzip and brotli compress each file on its own and are summed. Sizes only: correctness is in the Monitor Oxc comment.",
  ];
  const collapsed = Math.max(base.duplicates, head.duplicates);
  if (collapsed > 0) {
    notes.push(`Identical pnpm peer-variant copies were counted once (${fmt(collapsed)} collapsed).`);
  }
  if (onlyOneSide > 0) notes.push(`Files present on only one side were skipped (${fmt(onlyOneSide)}).`);
  out.push(`<sub>${notes.join(" ")}</sub>\n`);
  if (summary) out.push(MARKER);
  return out.join("\n") + "\n";
}

const diffResults = new Map();
function diffCache(f) {
  if (!diffResults.has(f.key)) diffResults.set(f.key, diffFile(f));
  return diffResults.get(f.key);
}

// Render with the first limits that fit.
function fit(summary, max) {
  const limits = [[20, 10, 50, 5], [10, 10, 20, 3], [5, 5, 10, 1], [5, 5, 10, 0]];
  let body;
  for (const [topFiles, topPackages, maxFailures, inlineDiffs] of limits) {
    body = render({
      summary,
      topFiles,
      topPackages,
      maxFailures: summary ? Math.min(maxFailures, COMMENT_FAILURES) : maxFailures,
      inlineDiffs,
    });
    if (body.length <= max) return body;
  }
  // Still too long (only possible with crafted data): say so instead of
  // failing to post.
  return `${header()}\nThe report is too large to show here; see the run.\n${summary ? `\n${MARKER}\n` : ""}`;
}

if (args.full) fs.writeFileSync(args.full, fit(false, MAX_FULL));
process.stdout.write(fit(true, MAX_COMMENT));
