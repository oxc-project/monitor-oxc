// Differential runtime oracle over UglifyJS's `test/compress` corpus.
//
// Usage: node scripts/uglify-corpus-test.mjs <uglify-clone-dir>
//   env MONITOR_OXC_BIN overrides the binary path (default `./monitor-oxc`).
//   Exit 0 = every non-skipped case ran identically before and after minify.
//   Exit 1 = a runtime diff or a minify failure (writes evidence to
//            uglify-corpus-artifacts/). Exit 2 = usage/config/corpus error.
//
// How it works (purely differential — the corpus's own `expect_stdout`
// strings are NEVER trusted; expected output is always produced by executing
// the ORIGINAL case in this run):
//
//   1. Parse each `test/compress/*.js` DSL file. Every top-level labelled
//      block is one case: `name: { options=..., input: {...}, expect: ...,
//      expect_stdout: ..., node_version: ... }`. A tiny brace/token scanner
//      (string / template / regex / comment aware) slices out the `input`
//      block body verbatim and detects the `expect_stdout`, `expression` and
//      `node_version` members. We implement our own minimal extractor rather
//      than reusing uglify's AST walker because that needs uglify's parser +
//      the `semver` npm dep, and this harness is builtins-only.
//   2. A case is RUNNABLE iff it carries `expect_stdout` (that flag is only
//      used as a runnable marker) and its `node_version` range admits the
//      current node. Runnable, non-skip-listed cases are written to a scratch
//      `cases/<file>/<name>.js` tree — expression cases wrapped in
//      `console.log(( ... ))` so their value is observed (and not DCE'd).
//   3. ONE `runtime-minify` invocation minifies the whole tree in place with
//      the standard full pipeline (per-case uglify `options` are ignored, as
//      esbuild does). Any minify diagnostic on a non-skipped case fails the
//      run — that is parser/minifier signal, not noise; such corpus files are
//      instead seeded into the skip list with a reason.
//   4. Each case is executed original-vs-minified in fresh `vm` contexts using
//      a port of uglify's `test/sandbox.js` semantics (captured stdout,
//      Function.prototype.toString normalisation so renamed functions print
//      identically, `F######N` id stripping, 5s timeout). The original is run
//      twice as a determinism gate; a case that differs across its two
//      original runs is nondeterministic and auto-skipped with `::warning::`,
//      never a red. A deterministic original/minified stdout diff is a bug.
//
// `.js` files are parsed unambiguously by the pipeline, so a case using
// top-level import/export becomes a module and cannot run in `vm`'s script
// mode; such files live in the skip list.

import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import util from "node:util";
import vm from "node:vm";

// A case's async body can reject *after* its synchronous portion (all we
// capture and compare) has run. Swallow those late rejections so they neither
// pollute stderr nor trip Node's default reject-throws exit — uglify's own
// sandbox does the same with `process.on("unhandledRejection", () => {})`.
process.on("unhandledRejection", () => {});

// ---------------------------------------------------------------------------
// CLI / config
// ---------------------------------------------------------------------------

function usage(msg) {
  if (msg) console.error(msg);
  console.error("usage: node scripts/uglify-corpus-test.mjs <uglify-clone-dir>");
  process.exit(2);
}

const argv = process.argv.slice(2).filter((a) => a !== "");
if (argv.length !== 1 || argv[0] === "-h" || argv[0] === "--help") usage();
const cloneDir = argv[0];

const bin = process.env.MONITOR_OXC_BIN ?? "./monitor-oxc";
const configPath = "uglify-corpus.json";

const compressDir = path.join(cloneDir, "test", "compress");
if (!fs.existsSync(compressDir)) {
  usage(`corpus dir not found: ${compressDir} (is \`${cloneDir}\` a UglifyJS checkout?)`);
}

let corpus;
try {
  corpus = JSON.parse(fs.readFileSync(configPath, "utf8"));
} catch (err) {
  console.error(`failed to read ${configPath}: ${err.message}`);
  process.exit(2);
}
if (!corpus || typeof corpus !== "object" || !Array.isArray(corpus.skips)) {
  console.error(`${configPath} must be an object with a "skips" array`);
  process.exit(2);
}

// skip index: file -> { whole: reason|null, names: Map<name, reason> }
const skipIndex = new Map();
for (const skip of corpus.skips) {
  if (!skip || typeof skip.file !== "string" || typeof skip.reason !== "string") {
    console.error(`${configPath}: every skip needs string "file" and "reason"`);
    process.exit(2);
  }
  let entry = skipIndex.get(skip.file);
  if (!entry) {
    entry = { whole: null, names: new Map() };
    skipIndex.set(skip.file, entry);
  }
  if (skip.name === undefined) entry.whole = skip.reason;
  else entry.names.set(skip.name, skip.reason);
}
function skipReason(file, name) {
  const entry = skipIndex.get(file);
  if (!entry) return null;
  if (entry.whole !== null) return entry.whole;
  return entry.names.get(name) ?? null;
}

// ---------------------------------------------------------------------------
// Minimal token-aware scanner for the uglify test DSL
// ---------------------------------------------------------------------------

function skipString(src, i) {
  const quote = src[i++];
  while (i < src.length) {
    const c = src[i];
    if (c === "\\") i += 2;
    else if (c === quote) return i + 1;
    else i++;
  }
  return i;
}

function skipTemplate(src, i) {
  i++; // opening backtick
  while (i < src.length) {
    const c = src[i];
    if (c === "\\") {
      i += 2;
    } else if (c === "`") {
      return i + 1;
    } else if (c === "$" && src[i + 1] === "{") {
      i = matchBrace(src, i + 1); // skip ${ ... }
    } else {
      i++;
    }
  }
  return i;
}

// Regex-vs-division: a `/` begins a regex unless the previous significant
// token could end an expression (identifier, number, `)`, `]`, `}`, `.`).
function regexAllowed(src, i) {
  let j = i - 1;
  while (j >= 0) {
    const c = src[j];
    if (c === " " || c === "\t" || c === "\n" || c === "\r") { j--; continue; }
    // skip a preceding line/block comment
    if (c === "/" && src[j - 1] === "/") { j -= 2; continue; }
    break;
  }
  if (j < 0) return true;
  const c = src[j];
  if (/[)\]}]/.test(c)) return false;
  if (/[\w$]/.test(c)) {
    // keyword (return, typeof, ...) => regex; identifier/number => division
    let k = j;
    while (k >= 0 && /[\w$]/.test(src[k])) k--;
    const word = src.slice(k + 1, j + 1);
    return /^(return|typeof|instanceof|in|of|new|delete|void|do|else|yield|await|case)$/.test(word);
  }
  return true;
}

function skipRegex(src, i) {
  i++; // opening slash
  let inClass = false;
  while (i < src.length) {
    const c = src[i];
    if (c === "\\") { i += 2; continue; }
    if (c === "[") inClass = true;
    else if (c === "]") inClass = false;
    else if (c === "/" && !inClass) { i++; break; }
    else if (c === "\n") break; // unterminated; bail
    i++;
  }
  while (i < src.length && /[a-z]/i.test(src[i])) i++; // flags
  return i;
}

// Given index of an opening `{`, return the index just past its match.
function matchBrace(src, open) {
  let depth = 0;
  let i = open;
  while (i < src.length) {
    const c = src[i];
    if (c === "/") {
      const n = src[i + 1];
      if (n === "/") { const e = src.indexOf("\n", i + 2); i = e < 0 ? src.length : e; continue; }
      if (n === "*") { const e = src.indexOf("*/", i + 2); i = e < 0 ? src.length : e + 2; continue; }
      if (regexAllowed(src, i)) { i = skipRegex(src, i); continue; }
      i++; continue;
    }
    if (c === '"' || c === "'") { i = skipString(src, i); continue; }
    if (c === "`") { i = skipTemplate(src, i); continue; }
    if (c === "{") { depth++; i++; continue; }
    if (c === "}") { depth--; i++; if (depth === 0) return i; continue; }
    i++;
  }
  return -1;
}

const IDENT_START = /[A-Za-z_$]/;

// Skip whitespace and comments; return next index.
function skipTrivia(src, i) {
  while (i < src.length) {
    const c = src[i];
    if (c === " " || c === "\t" || c === "\n" || c === "\r") { i++; continue; }
    if (c === "/" && src[i + 1] === "/") { const e = src.indexOf("\n", i + 2); i = e < 0 ? src.length : e; continue; }
    if (c === "/" && src[i + 1] === "*") { const e = src.indexOf("*/", i + 2); i = e < 0 ? src.length : e + 2; continue; }
    break;
  }
  return i;
}

// Parse the top-level cases of a corpus file: `name: { ... }` labelled blocks.
function parseCases(src) {
  const cases = [];
  let i = 0;
  while (i < src.length) {
    i = skipTrivia(src, i);
    if (i >= src.length) break;
    if (!IDENT_START.test(src[i])) { i++; continue; }
    let j = i;
    while (j < src.length && /[\w$]/.test(src[j])) j++;
    const name = src.slice(i, j);
    const afterName = skipTrivia(src, j);
    if (src[afterName] !== ":") { i = j; continue; }
    const afterColon = skipTrivia(src, afterName + 1);
    if (src[afterColon] !== "{") { i = afterColon; continue; }
    const end = matchBrace(src, afterColon);
    if (end < 0) break;
    const body = src.slice(afterColon + 1, end - 1);
    cases.push({ name, body });
    i = end;
  }
  return cases;
}

// Within a case body, locate the `input: { ... }` block and read the flat
// members we care about. Returns { input, hasStdout, expression, nodeVersion }.
function parseCaseBody(body) {
  let input = null;
  let inputStart = -1;
  let inputEnd = -1;
  let hasStdout = false;
  let expression = false;
  let nodeVersion = null;

  let i = 0;
  while (i < body.length) {
    i = skipTrivia(body, i);
    if (i >= body.length) break;
    if (!IDENT_START.test(body[i])) { i++; continue; }
    let j = i;
    while (j < body.length && /[\w$]/.test(body[j])) j++;
    const member = body.slice(i, j);
    const afterName = skipTrivia(body, j);
    const sep = body[afterName];

    if (sep === ":") {
      const valStart = skipTrivia(body, afterName + 1);
      if (body[valStart] === "{") {
        const end = matchBrace(body, valStart);
        if (member === "input") {
          input = body.slice(valStart + 1, end - 1);
          inputStart = i;
          inputEnd = end;
        }
        i = end < 0 ? body.length : end;
        continue;
      }
      if (member === "expect_stdout") hasStdout = true;
      if (member === "node_version") {
        const q = body[valStart];
        if (q === '"' || q === "'") {
          const strEnd = skipString(body, valStart);
          nodeVersion = body.slice(valStart + 1, strEnd - 1);
        }
      }
      // advance past this value (string / array / call / etc.)
      i = skipValue(body, valStart);
      continue;
    }
    if (sep === "=") {
      const valStart = skipTrivia(body, afterName + 1);
      if (member === "expression" && /^true\b/.test(body.slice(valStart))) expression = true;
      i = skipValue(body, valStart);
      continue;
    }
    i = j;
  }
  return { input, inputStart, inputEnd, hasStdout, expression, nodeVersion };
}

// Advance past a member value (object, array, call, string, literal) until the
// next member starts or the body ends.
function skipValue(src, i) {
  while (i < src.length) {
    const c = src[i];
    if (c === "/") {
      const n = src[i + 1];
      if (n === "/") { const e = src.indexOf("\n", i + 2); i = e < 0 ? src.length : e; continue; }
      if (n === "*") { const e = src.indexOf("*/", i + 2); i = e < 0 ? src.length : e + 2; continue; }
      if (regexAllowed(src, i)) { i = skipRegex(src, i); continue; }
      i++; continue;
    }
    if (c === '"' || c === "'") { i = skipString(src, i); continue; }
    if (c === "`") { i = skipTemplate(src, i); continue; }
    if (c === "{" || c === "[" || c === "(") { i = matchBracket(src, i); continue; }
    if (c === "\n") {
      // A newline at depth 0 ends the value if a new member follows.
      const k = skipTrivia(src, i + 1);
      if (k >= src.length) return k;
      if (IDENT_START.test(src[k])) {
        let m = k;
        while (m < src.length && /[\w$]/.test(src[m])) m++;
        const after = skipTrivia(src, m);
        if (src[after] === ":" || src[after] === "=") return i + 1;
      }
      i++;
      continue;
    }
    i++;
  }
  return i;
}

// Match any bracket kind `{}` `[]` `()` from its opening index.
function matchBracket(src, open) {
  const pairs = { "{": "}", "[": "]", "(": ")" };
  const stack = [pairs[src[open]]];
  let i = open + 1;
  while (i < src.length && stack.length) {
    const c = src[i];
    if (c === "/") {
      const n = src[i + 1];
      if (n === "/") { const e = src.indexOf("\n", i + 2); i = e < 0 ? src.length : e; continue; }
      if (n === "*") { const e = src.indexOf("*/", i + 2); i = e < 0 ? src.length : e + 2; continue; }
      if (regexAllowed(src, i)) { i = skipRegex(src, i); continue; }
      i++; continue;
    }
    if (c === '"' || c === "'") { i = skipString(src, i); continue; }
    if (c === "`") { i = skipTemplate(src, i); continue; }
    if (c === "{" || c === "[" || c === "(") { stack.push(pairs[c]); i++; continue; }
    if (c === "}" || c === "]" || c === ")") {
      if (stack[stack.length - 1] === c) stack.pop();
      i++;
      continue;
    }
    i++;
  }
  return i;
}

// ---------------------------------------------------------------------------
// node_version range check (minimal; conservative: unknown => not satisfied)
// ---------------------------------------------------------------------------

function parseVersion(v) {
  const m = /^v?(\d+)(?:\.(\d+))?(?:\.(\d+))?/.exec(v.trim());
  if (!m) return null;
  return [Number(m[1]), Number(m[2] ?? 0), Number(m[3] ?? 0)];
}
function cmp(a, b) {
  for (let k = 0; k < 3; k++) if (a[k] !== b[k]) return a[k] < b[k] ? -1 : 1;
  return 0;
}
function satisfiesComparator(version, token) {
  const m = /^(>=|<=|>|<|=)?\s*(.+)$/.exec(token.trim());
  if (!m) return false;
  const op = m[1] ?? "=";
  const target = parseVersion(m[2]);
  if (!target) return false;
  // Fill missing components: for upper bounds treat as +Infinity so e.g.
  // `<=10` admits 10.22.0; a bare/exact token matches on given precision.
  const c = cmp(version, target);
  switch (op) {
    case ">=": return c >= 0;
    case ">": return c > 0;
    case "<=": {
      const parts = m[2].trim().split(".").length;
      const upper = [target[0], parts > 1 ? target[1] : Infinity, parts > 2 ? target[2] : Infinity];
      return cmp(version, upper) <= 0;
    }
    case "<": return cmp(version, target) < 0;
    default: {
      // bare/exact: match on the precision given (e.g. "4" => major 4)
      const parts = m[2].trim().split(".").length;
      for (let k = 0; k < parts; k++) if (version[k] !== target[k]) return false;
      return true;
    }
  }
}
function nodeSatisfies(range, versionStr) {
  const version = parseVersion(versionStr);
  if (!version) return false;
  return range.split("||").some((group) =>
    group.trim().split(/\s+/).filter(Boolean).every((tok) => satisfiesComparator(version, tok)),
  );
}

// ---------------------------------------------------------------------------
// vm sandbox (port of uglify test/sandbox.js run_code_vm semantics)
// ---------------------------------------------------------------------------

// Match uglify's util.inspect setup so object/array logging is deterministic.
function setupLog() {
  const inspect = util.inspect;
  if (inspect.defaultOptions) {
    const logOptions = {
      breakLength: Infinity,
      colors: false,
      compact: true,
      customInspect: false,
      depth: Infinity,
      maxArrayLength: Infinity,
      maxStringLength: Infinity,
      showHidden: false,
    };
    for (const name in logOptions) {
      if (name in inspect.defaultOptions) inspect.defaultOptions[name] = logOptions[name];
    }
  }
  return inspect;
}

// uglify's in-context setup: normalise Function#toString (so minifier renames
// don't change logged function bodies), install a safe console.log, expose
// global/self/window, delete host builtins. Copied semantically from
// test/sandbox.js `setup`.
function setup(global, builtins, setupLog) {
  [Array, Boolean, Error, Function, Number, Object, RegExp, String].forEach((f) => {
    f.toString = Function.prototype.toString;
  });
  Function.prototype.toString = (function () {
    const configurable = Object.getOwnPropertyDescriptor(Function.prototype, "name").configurable;
    let id = 100000;
    return function () {
      let n = this.name;
      if (!/^F[0-9]{6}N$/.test(n)) {
        n = "F" + ++id + "N";
        if (configurable) Object.defineProperty(this, "name", { get: () => n });
      }
      return "function(){}";
    };
  })();
  const log = console.log;
  const safeConsole = {
    log: function (msg) {
      if (arguments.length === 1 && typeof msg === "string") return log("%s", msg);
      return log.apply(null, [].map.call(arguments, (arg) =>
        safeLog(arg, { level: 5, original: [], replaced: [] }),
      ));
    },
  };
  const props = {
    console: { get: () => safeConsole },
    global: { get: self },
    self: { get: self },
    window: { get: self },
  };
  builtins.forEach((name) => {
    try { delete global[name]; } catch (e) {}
  });
  Object.defineProperties(global, props);
  global.__proto__ = Object.defineProperty(Object.create(global.__proto__), "toString", {
    value: () => "[object global]",
  });

  function self() { return this; }

  function safeLog(arg, cache) {
    if (arg) switch (typeof arg) {
      case "function":
        return arg.toString();
      case "object":
        if (arg === global) return "[object global]";
        if (/Error$/.test(arg.name)) return arg.toString();
        if (typeof arg.then === "function") return "[object Promise]";
        if (arg.constructor) arg.constructor.toString();
        var index = cache.original.indexOf(arg);
        if (index >= 0) return cache.replaced[index];
        if (--cache.level < 0) return "[object Object]";
        var value = {};
        cache.original.push(arg);
        cache.replaced.push(value);
        for (var key in arg) {
          var desc = Object.getOwnPropertyDescriptor(arg, key);
          if (desc && (desc.get || desc.set)) Object.defineProperty(value, key, desc);
          else value[key] = safeLog(arg[key], cache);
        }
        return value;
    }
    return arg;
  }
}

setupLog();

function stripColorCodes(value) {
  return value.replace(/\[\d+m/g, "");
}
function stripFuncIds(text) {
  return ("" + text).replace(/F[0-9]{6}N/g, "<F<>N>");
}
function isError(result) {
  return result && typeof result.name === "string" && typeof result.message === "string";
}

// Build the in-context setup code exactly like sandbox.js: `(setup)(this,
// <builtins>, setupLog);`. Builtins are the context globals to delete, found
// by logging Object.keys(this) from a bare run.
function runVm(prelude, code, timeout = 5000) {
  let stdout = "";
  const originalWrite = process.stdout.write;
  process.stdout.write = (chunk) => { stdout += chunk; return true; };
  let result;
  try {
    const ctx = vm.createContext({ console });
    if (prelude) vm.runInContext(prelude, ctx);
    vm.runInContext(code, ctx, { timeout });
    result = {};
  } catch (ex) {
    if (ex && (ex.code === "ERR_SCRIPT_EXECUTION_TIMEOUT" || /timed out/i.test(ex.message ?? ""))) {
      result = { timeout: true };
    } else {
      result = { error: { name: ex && ex.name, message: ex && ex.message } };
    }
  } finally {
    process.stdout.write = originalWrite;
  }
  // Keep stdout on error results too: output produced before a throw is
  // oracle-relevant (a dropped/reordered log ahead of the same throw is a bug).
  result.stdout = stripColorCodes(stdout.replace(/\b(Array \[|Object {)/g, (m) => m.slice(-1)));
  return result;
}

const builtinsLiteral = (runVm("", "console.log(Object.keys(this));").stdout ?? "[]").trim();
const setupCode =
  "(" + setup + ")(" + ["this", builtinsLiteral, setupLog].join(",\n") + ");\n";

function runCase(code, timeout) {
  return runVm(setupCode, code, timeout);
}

// uglify same_stdout compares errors by name + last message line only; we are
// stricter and additionally require identical pre-throw stdout, so a minifier
// bug that drops/reorders logs ahead of an identical final throw still fails.
function sameResult(a, b) {
  if (a.timeout || b.timeout) return a.timeout && b.timeout;
  const aErr = a.error && isError(a.error);
  const bErr = b.error && isError(b.error);
  if (aErr !== bErr) return false;
  if (aErr) {
    if (a.error.name !== b.error.name) return false;
    const lastLine = (m) => m.slice(m.lastIndexOf("\n") + 1);
    if (stripFuncIds(lastLine(a.error.message)) !== stripFuncIds(lastLine(b.error.message))) {
      return false;
    }
  } else if (a.error || b.error) {
    return false; // a non-Error throw
  }
  return stripFuncIds(a.stdout) === stripFuncIds(b.stdout);
}

function describe(r) {
  if (r.timeout) return "<timeout>";
  if (r.error) {
    const error = `${r.error.name}: ${r.error.message}`;
    return r.stdout ? `${JSON.stringify(r.stdout)} then ${error}` : error;
  }
  return JSON.stringify(r.stdout);
}

// ---------------------------------------------------------------------------
// Extract runnable cases
// ---------------------------------------------------------------------------

const corpusFiles = fs
  .readdirSync(compressDir)
  .filter((n) => /\.js$/i.test(n))
  .sort();

const stats = {
  total: 0,
  skipped: {}, // reason -> count
  excluded: {}, // reason -> count
};
function bump(bucket, reason) {
  bucket[reason] = (bucket[reason] ?? 0) + 1;
}

const keepScratch = process.env.UGLIFY_CORPUS_KEEP === "1";
const scratchRoot = fs.mkdtempSync(path.join(os.tmpdir(), "uglify-corpus-"));
const casesRoot = path.join(scratchRoot, "cases");
const runnable = []; // { file, name, rel, absPath, original, expression }

for (const file of corpusFiles) {
  const relFile = `compress/${file}`;
  const src = fs.readFileSync(path.join(compressDir, file), "utf8");
  let cases;
  try {
    cases = parseCases(src);
  } catch (err) {
    console.error(`failed to parse corpus file ${relFile}: ${err.message}`);
    if (!keepScratch) fs.rmSync(scratchRoot, { recursive: true, force: true }); else console.error(`kept scratch: ${scratchRoot}`);
    process.exit(2);
  }
  for (const c of cases) {
    stats.total++;
    const reason = skipReason(relFile, c.name);
    if (reason !== null) {
      bump(stats.skipped, reason);
      continue;
    }
    const info = parseCaseBody(c.body);
    if (!info.hasStdout || info.input === null) {
      bump(stats.excluded, "not runnable (no expect_stdout)");
      continue;
    }
    if (info.nodeVersion && !nodeSatisfies(info.nodeVersion, process.version)) {
      bump(stats.excluded, "node_version gate");
      continue;
    }
    let body = info.input;
    if (info.expression) body = `console.log((\n${body.replace(/;\s*$/, "")}\n));`;
    const rel = path.join(file.replace(/\.js$/i, ""), `${c.name}.js`);
    const absPath = path.join(casesRoot, rel);
    fs.mkdirSync(path.dirname(absPath), { recursive: true });
    fs.writeFileSync(absPath, body + "\n");
    runnable.push({ file: relFile, name: c.name, rel, absPath, original: body + "\n", expression: info.expression });
  }
}

if (runnable.length === 0) {
  console.error("no runnable cases extracted — extractor or corpus problem");
  if (!keepScratch) fs.rmSync(scratchRoot, { recursive: true, force: true }); else console.error(`kept scratch: ${scratchRoot}`);
  process.exit(2);
}

// ---------------------------------------------------------------------------
// Minify the whole tree in one runtime-minify invocation
// ---------------------------------------------------------------------------

const minifyConfig = path.join(scratchRoot, "config.json");
fs.writeFileSync(
  minifyConfig,
  JSON.stringify([{ name: "uglify-corpus", sources: ["cases/**/*.js"] }]),
);

const minify = spawnSync(
  bin,
  ["runtime-minify", "--config", minifyConfig, "--name", "uglify-corpus", "--dir", scratchRoot],
  { stdio: "inherit" },
);
if (minify.status !== 0) {
  console.error(
    `runtime-minify failed (exit ${minify.status}${minify.error ? `, ${minify.error.message}` : ""}) — ` +
      `add the offending corpus file(s) to ${configPath} skips with a reason, or fix the minifier`,
  );
  if (!keepScratch) fs.rmSync(scratchRoot, { recursive: true, force: true }); else console.error(`kept scratch: ${scratchRoot}`);
  process.exit(1);
}

// ---------------------------------------------------------------------------
// Execute + diff
// ---------------------------------------------------------------------------

const artifactsDir = "uglify-corpus-artifacts";
let compared = 0;
let failed = false;

function saveArtifacts(item, original, minified, expected, actual) {
  const dir = path.join(artifactsDir, item.file.replace(/[\/]/g, "_"), item.name);
  fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(path.join(dir, "original.js"), original);
  fs.writeFileSync(path.join(dir, "minified.js"), minified);
  fs.writeFileSync(path.join(dir, "expected.txt"), describe(expected) + "\n");
  fs.writeFileSync(path.join(dir, "actual.txt"), describe(actual) + "\n");
}

// The transform stage lowers modern syntax (class fields/private members,
// object spread/rest, ...) into calls to `@oxc-project/runtime` helpers, and
// module cases keep import/export. None of that resolves in a builtins-only
// `vm` script, so such cases are non-oracle here (runtime-helper correctness
// is covered by the Runtime Bundles oracle, which runs in real node_modules).
function needsModuleLoader(code) {
  return (
    /\brequire\s*\(\s*["'`]/.test(code) ||
    /(^|[\n;{}])\s*(?:import|export)[\s{*]/.test(code)
  );
}

for (const item of runnable) {
  const minified = fs.readFileSync(item.absPath, "utf8");
  if (needsModuleLoader(minified) || needsModuleLoader(item.original)) {
    bump(stats.excluded, "needs module loader (transform runtime helpers / ESM)");
    continue;
  }
  const first = runCase(item.original);

  // Original that cannot run in this harness is a non-oracle case.
  if (first.timeout) {
    bump(stats.excluded, "original times out");
    continue;
  }
  if (first.error && first.error.name === "SyntaxError") {
    bump(stats.excluded, "original SyntaxError in vm (module/host syntax)");
    continue;
  }

  const second = runCase(item.original);
  if (!sameResult(first, second)) {
    console.log(`::warning::${item.file} [${item.name}] is nondeterministic — skipped, not an oxc bug`);
    bump(stats.skipped, "nondeterministic original (auto)");
    continue;
  }

  const actual = runCase(minified);
  if (!sameResult(first, actual)) {
    console.error(`FAIL ${item.file} [${item.name}]: minified output DIFFERS`);
    console.error(`  expected: ${describe(first)}`);
    console.error(`  actual:   ${describe(actual)}`);
    saveArtifacts(item, item.original, minified, first, actual);
    failed = true;
    continue;
  }
  compared++;
}

// ---------------------------------------------------------------------------
// Summary
// ---------------------------------------------------------------------------

if (!keepScratch) fs.rmSync(scratchRoot, { recursive: true, force: true });
else console.error(`kept scratch: ${scratchRoot}`);

const skippedTotal = Object.values(stats.skipped).reduce((a, b) => a + b, 0);
const excludedTotal = Object.values(stats.excluded).reduce((a, b) => a + b, 0);
const fmt = (bucket) =>
  Object.entries(bucket)
    .sort((a, b) => b[1] - a[1])
    .map(([r, n]) => `${r}: ${n}`)
    .join("; ");

console.log(
  `${stats.total} cases: ${compared} compared, ${skippedTotal} skipped ` +
    `(${fmt(stats.skipped) || "none"}), ${excludedTotal} excluded (${fmt(stats.excluded) || "none"})`,
);

process.exit(failed ? 1 : 0);
