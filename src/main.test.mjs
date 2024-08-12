import test from "node:test"
import assert from "node:assert"

import { camelCase } from "@luca/cases"
test("@luca/cases", () => {
  assert.equal(camelCase("hello world"), "helloWorld")
})

import { Hono } from '@hono/hono'
test("@hono/hono", async () => {
  const app = new Hono()
  app.get('/posts', (c) => c.text('Many posts'))
  const res = await app.request('/posts')
  assert.equal(res.status, 200)
  assert.equal(await res.text(), 'Many posts')
})

// Start of npm packages

import * as babel_core from "@babel/core";
test("@babel/core", () => {
  assert.ok(babel_core);
});

import * as babel_generator from "@babel/generator";
test("@babel/generator", () => {
  assert.ok(babel_generator);
});

import * as babel_parser from "@babel/parser";
test("@babel/parser", () => {
  assert.ok(babel_parser);
});

import * as babel_preset_env from "@babel/preset-env";
test("@babel/preset-env", () => {
  assert.ok(babel_preset_env);
});

import * as testing_library_user_event from "@testing-library/user-event";
test("@testing-library/user-event", () => {
  assert.ok(testing_library_user_event);
});

import * as acorn from "acorn";
test("acorn", () => {
  assert.ok(acorn);
});

import * as acorn_walk from "acorn-walk";
test("acorn-walk", () => {
  assert.ok(acorn_walk);
});

import * as ajv from "ajv";
test("ajv", () => {
  assert.ok(ajv);
});

import * as ansi_escapes from "ansi-escapes";
test("ansi-escapes", () => {
  assert.ok(ansi_escapes);
});

import * as ansi_regex from "ansi-regex";
test("ansi-regex", () => {
  assert.ok(ansi_regex);
});

import * as ansi_styles from "ansi-styles";
test("ansi-styles", () => {
  assert.ok(ansi_styles);
});

import * as arg from "arg";
test("arg", () => {
  assert.ok(arg);
});

import * as argparse from "argparse";
test("argparse", () => {
  assert.ok(argparse);
});

import * as async from "async";
test("async", () => {
  assert.ok(async);
});

import * as autoprefixer from "autoprefixer";
test("autoprefixer", () => {
  assert.ok(autoprefixer);
});

import * as axios from "axios";
test("axios", () => {
  assert.ok(axios);
});

import * as babel_jest from "babel-jest";
test("babel-jest", () => {
  assert.ok(babel_jest);
});

import * as balanced_match from "balanced-match";
test("balanced-match", () => {
  assert.ok(balanced_match);
});

import * as base64_js from "base64-js";
test("base64-js", () => {
  assert.ok(base64_js);
});

import * as brace_expansion from "brace-expansion";
test("brace-expansion", () => {
  assert.ok(brace_expansion);
});

import * as braces from "braces";
test("braces", () => {
  assert.ok(braces);
});

import * as browserslist from "browserslist";
test("browserslist", () => {
  assert.ok(browserslist);
});

import * as buffer from "buffer";
test("buffer", () => {
  assert.ok(buffer);
});

import * as call_bind from "call-bind";
test("call-bind", () => {
  assert.ok(call_bind);
});

import * as camelcase from "camelcase";
test("camelcase", () => {
  assert.ok(camelcase);
});

import * as caniuse_lite from "caniuse-lite";
test("caniuse-lite", () => {
  assert.ok(caniuse_lite);
});

import * as chalk from "chalk";
test("chalk", () => {
  assert.ok(chalk);
});

import * as chokidar from "chokidar";
test("chokidar", () => {
  assert.ok(chokidar);
});

import * as classnames from "classnames";
test("classnames", () => {
  assert.ok(classnames);
});

import * as cliui from "cliui";
test("cliui", () => {
  assert.ok(cliui);
});

import * as clone from "clone";
test("clone", () => {
  assert.ok(clone);
});

import * as clsx from "clsx";
test("clsx", () => {
  assert.ok(clsx);
});

import * as color_convert from "color-convert";
test("color-convert", () => {
  assert.ok(color_convert);
});

import * as color_name from "color-name";
test("color-name", () => {
  assert.ok(color_name);
});

import * as commander from "commander";
test("commander", () => {
  assert.ok(commander);
});

import * as concat_map from "concat-map";
test("concat-map", () => {
  assert.ok(concat_map);
});

import * as convert_source_map from "convert-source-map";
test("convert-source-map", () => {
  assert.ok(convert_source_map);
});

import * as cookie from "cookie";
test("cookie", () => {
  assert.ok(cookie);
});

import * as cors from "cors";
test("cors", () => {
  assert.ok(cors);
});

import * as cosmiconfig from "cosmiconfig";
test("cosmiconfig", () => {
  assert.ok(cosmiconfig);
});

import * as cross_spawn from "cross-spawn";
test("cross-spawn", () => {
  assert.ok(cross_spawn);
});

import * as date_fns from "date-fns";
test("date-fns", () => {
  assert.ok(date_fns);
});

import * as dayjs from "dayjs";
test("dayjs", () => {
  assert.ok(dayjs);
});

import * as debug from "debug";
test("debug", () => {
  assert.ok(debug);
});

import * as deepmerge from "deepmerge";
test("deepmerge", () => {
  assert.ok(deepmerge);
});

import * as diff from "diff";
test("diff", () => {
  assert.ok(diff);
});

import * as doctrine from "doctrine";
test("doctrine", () => {
  assert.ok(doctrine);
});

import * as dotenv from "dotenv";
test("dotenv", () => {
  assert.ok(dotenv);
});

import * as ejs from "ejs";
test("ejs", () => {
  assert.ok(ejs);
});

import * as emoji_regex from "emoji-regex";
test("emoji-regex", () => {
  assert.ok(emoji_regex);
});

import * as esbuild from "esbuild";
test("esbuild", () => {
  assert.ok(esbuild);
});

import * as escalade from "escalade";
test("escalade", () => {
  assert.ok(escalade);
});

import * as escape_string_regexp from "escape-string-regexp";
test("escape-string-regexp", () => {
  assert.ok(escape_string_regexp);
});

import * as eslint from "eslint";
test("eslint", () => {
  assert.ok(eslint);
});

import * as eslint_config_prettier from "eslint-config-prettier";
test("eslint-config-prettier", () => {
  assert.ok(eslint_config_prettier);
});

import * as eslint_plugin_import from "eslint-plugin-import";
test("eslint-plugin-import", () => {
  assert.ok(eslint_plugin_import);
});

import * as eslint_plugin_react from "eslint-plugin-react";
test("eslint-plugin-react", () => {
  assert.ok(eslint_plugin_react);
});

import * as eslint_plugin_react_hooks from "eslint-plugin-react-hooks";
test("eslint-plugin-react-hooks", () => {
  assert.ok(eslint_plugin_react_hooks);
});

import * as eslint_scope from "eslint-scope";
test("eslint-scope", () => {
  assert.ok(eslint_scope);
});

import * as eslint_visitor_keys from "eslint-visitor-keys";
test("eslint-visitor-keys", () => {
  assert.ok(eslint_visitor_keys);
});

import * as estraverse from "estraverse";
test("estraverse", () => {
  assert.ok(estraverse);
});

import * as eventemitter3 from "eventemitter3";
test("eventemitter3", () => {
  assert.ok(eventemitter3);
});

import * as events from "events";
test("events", () => {
  assert.ok(events);
});

import * as execa from "execa";
test("execa", () => {
  assert.ok(execa);
});

import * as express from "express";
test("express", () => {
  assert.ok(express);
});

import * as fast_deep_equal from "fast-deep-equal";
test("fast-deep-equal", () => {
  assert.ok(fast_deep_equal);
});

import * as fast_glob from "fast-glob";
test("fast-glob", () => {
  assert.ok(fast_glob);
});

import * as fill_range from "fill-range";
test("fill-range", () => {
  assert.ok(fill_range);
});

import * as find_up from "find-up";
test("find-up", () => {
  assert.ok(find_up);
});

import * as form_data from "form-data";
test("form-data", () => {
  assert.ok(form_data);
});

import * as fs_extra from "fs-extra";
test("fs-extra", () => {
  assert.ok(fs_extra);
});

import * as function_bind from "function-bind";
test("function-bind", () => {
  assert.ok(function_bind);
});

import * as get_stream from "get-stream";
test("get-stream", () => {
  assert.ok(get_stream);
});

import * as glob from "glob";
test("glob", () => {
  assert.ok(glob);
});

import * as glob_parent from "glob-parent";
test("glob-parent", () => {
  assert.ok(glob_parent);
});

import * as globals from "globals";
test("globals", () => {
  assert.ok(globals);
});

import * as globby from "globby";
test("globby", () => {
  assert.ok(globby);
});

import * as graceful_fs from "graceful-fs";
test("graceful-fs", () => {
  assert.ok(graceful_fs);
});

import * as handlebars from "handlebars";
test("handlebars", () => {
  assert.ok(handlebars);
});

import * as has_flag from "has-flag";
test("has-flag", () => {
  assert.ok(has_flag);
});

import * as hasown from "hasown";
test("hasown", () => {
  assert.ok(hasown);
});

import * as https_proxy_agent from "https-proxy-agent";
test("https-proxy-agent", () => {
  assert.ok(https_proxy_agent);
});

import * as human_signals from "human-signals";
test("human-signals", () => {
  assert.ok(human_signals);
});

import * as iconv_lite from "iconv-lite";
test("iconv-lite", () => {
  assert.ok(iconv_lite);
});

import * as ignore from "ignore";
test("ignore", () => {
  assert.ok(ignore);
});

import * as inherits from "inherits";
test("inherits", () => {
  assert.ok(inherits);
});

import * as ini from "ini";
test("ini", () => {
  assert.ok(ini);
});

import * as inquirer from "inquirer";
test("inquirer", () => {
  assert.ok(inquirer);
});

import * as is_extglob from "is-extglob";
test("is-extglob", () => {
  assert.ok(is_extglob);
});

import * as is_fullwidth_code_point from "is-fullwidth-code-point";
test("is-fullwidth-code-point", () => {
  assert.ok(is_fullwidth_code_point);
});

import * as is_glob from "is-glob";
test("is-glob", () => {
  assert.ok(is_glob);
});

import * as is_stream from "is-stream";
test("is-stream", () => {
  assert.ok(is_stream);
});

import * as isarray from "isarray";
test("isarray", () => {
  assert.ok(isarray);
});

import * as isexe from "isexe";
test("isexe", () => {
  assert.ok(isexe);
});

import * as jest from "jest";
test("jest", () => {
  assert.ok(jest);
});

import * as jquery from "jquery";
test("jquery", () => {
  assert.ok(jquery);
});

import * as js_tokens from "js-tokens";
test("js-tokens", () => {
  assert.ok(js_tokens);
});

import * as js_yaml from "js-yaml";
test("js-yaml", () => {
  assert.ok(js_yaml);
});

import * as jsesc from "jsesc";
test("jsesc", () => {
  assert.ok(jsesc);
});

import * as json_schema_traverse from "json-schema-traverse";
test("json-schema-traverse", () => {
  assert.ok(json_schema_traverse);
});

import * as json5 from "json5";
test("json5", () => {
  assert.ok(json5);
});

import * as jsonfile from "jsonfile";
test("jsonfile", () => {
  assert.ok(jsonfile);
});

import * as jsonwebtoken from "jsonwebtoken";
test("jsonwebtoken", () => {
  assert.ok(jsonwebtoken);
});

import * as locate_path from "locate-path";
test("locate-path", () => {
  assert.ok(locate_path);
});

import * as lodash from "lodash";
test("lodash", () => {
  assert.ok(lodash);
});

import * as lodash_merge from "lodash.merge";
test("lodash.merge", () => {
  assert.ok(lodash_merge);
});

import * as log_symbols from "log-symbols";
test("log-symbols", () => {
  assert.ok(log_symbols);
});

import * as lru_cache from "lru-cache";
test("lru-cache", () => {
  assert.ok(lru_cache);
});

import * as make_dir from "make-dir";
test("make-dir", () => {
  assert.ok(make_dir);
});

import * as micromatch from "micromatch";
test("micromatch", () => {
  assert.ok(micromatch);
});

import * as mime from "mime";
test("mime", () => {
  assert.ok(mime);
});

import * as mime_db from "mime-db";
test("mime-db", () => {
  assert.ok(mime_db);
});

import * as mime_types from "mime-types";
test("mime-types", () => {
  assert.ok(mime_types);
});

import * as minimatch from "minimatch";
test("minimatch", () => {
  assert.ok(minimatch);
});

import * as minimist from "minimist";
test("minimist", () => {
  assert.ok(minimist);
});

import * as minipass from "minipass";
test("minipass", () => {
  assert.ok(minipass);
});

import * as mkdirp from "mkdirp";
test("mkdirp", () => {
  assert.ok(mkdirp);
});

import * as moment from "moment";
test("moment", () => {
  assert.ok(moment);
});

import * as ms from "ms";
test("ms", () => {
  assert.ok(ms);
});

import * as nanoid from "nanoid";
test("nanoid", () => {
  assert.ok(nanoid);
});

import * as next from "next";
test("next", () => {
  assert.ok(next);
});

import * as node_fetch from "node-fetch";
test("node-fetch", () => {
  assert.ok(node_fetch);
});

import * as normalize_path from "normalize-path";
test("normalize-path", () => {
  assert.ok(normalize_path);
});

import * as object_assign from "object-assign";
test("object-assign", () => {
  assert.ok(object_assign);
});

import * as once from "once";
test("once", () => {
  assert.ok(once);
});

import * as onetime from "onetime";
test("onetime", () => {
  assert.ok(onetime);
});

import * as ora from "ora";
test("ora", () => {
  assert.ok(ora);
});

import * as p_limit from "p-limit";
test("p-limit", () => {
  assert.ok(p_limit);
});

import * as p_locate from "p-locate";
test("p-locate", () => {
  assert.ok(p_locate);
});

import * as path_exists from "path-exists";
test("path-exists", () => {
  assert.ok(path_exists);
});

import * as path_key from "path-key";
test("path-key", () => {
  assert.ok(path_key);
});

import * as path_to_regexp from "path-to-regexp";
test("path-to-regexp", () => {
  assert.ok(path_to_regexp);
});

import * as path_type from "path-type";
test("path-type", () => {
  assert.ok(path_type);
});

import * as picocolors from "picocolors";
test("picocolors", () => {
  assert.ok(picocolors);
});

import * as picomatch from "picomatch";
test("picomatch", () => {
  assert.ok(picomatch);
});

import * as pkg_dir from "pkg-dir";
test("pkg-dir", () => {
  assert.ok(pkg_dir);
});

import * as postcss from "postcss";
test("postcss", () => {
  assert.ok(postcss);
});

import * as prettier from "prettier";
test("prettier", () => {
  assert.ok(prettier);
});

import * as pretty_format from "pretty-format";
test("pretty-format", () => {
  assert.ok(pretty_format);
});

import * as prop_types from "prop-types";
test("prop-types", () => {
  assert.ok(prop_types);
});

import * as punycode from "punycode";
test("punycode", () => {
  assert.ok(punycode);
});

import * as qs from "qs";
test("qs", () => {
  assert.ok(qs);
});

import * as react from "react";
test("react", () => {
  assert.ok(react);
});

import * as react_dom from "react-dom";
test("react-dom", () => {
  assert.ok(react_dom);
});

import * as react_is from "react-is";
test("react-is", () => {
  assert.ok(react_is);
});

import * as readable_stream from "readable-stream";
test("readable-stream", () => {
  assert.ok(readable_stream);
});

import * as reflect_metadata from "reflect-metadata";
test("reflect-metadata", () => {
  assert.ok(reflect_metadata);
});

import * as regenerator_runtime from "regenerator-runtime";
test("regenerator-runtime", () => {
  assert.ok(regenerator_runtime);
});

import * as resolve from "resolve";
test("resolve", () => {
  assert.ok(resolve);
});

import * as resolve_from from "resolve-from";
test("resolve-from", () => {
  assert.ok(resolve_from);
});

import * as rimraf from "rimraf";
test("rimraf", () => {
  assert.ok(rimraf);
});

import * as rollup from "rollup";
test("rollup", () => {
  assert.ok(rollup);
});

import * as safe_buffer from "safe-buffer";
test("safe-buffer", () => {
  assert.ok(safe_buffer);
});

import * as scheduler from "scheduler";
test("scheduler", () => {
  assert.ok(scheduler);
});

import * as semver from "semver";
test("semver", () => {
  assert.ok(semver);
});

import * as shebang_command from "shebang-command";
test("shebang-command", () => {
  assert.ok(shebang_command);
});

import * as shebang_regex from "shebang-regex";
test("shebang-regex", () => {
  assert.ok(shebang_regex);
});

import * as signal_exit from "signal-exit";
test("signal-exit", () => {
  assert.ok(signal_exit);
});

import * as slash from "slash";
test("slash", () => {
  assert.ok(slash);
});

import * as source_map_support from "source-map-support";
test("source-map-support", () => {
  assert.ok(source_map_support);
});

import * as sprintf_js from "sprintf-js";
test("sprintf-js", () => {
  assert.ok(sprintf_js);
});

import * as string_width from "string-width";
test("string-width", () => {
  assert.ok(string_width);
});

import * as string_decoder from "string_decoder";
test("string_decoder", () => {
  assert.ok(string_decoder);
});

import * as strip_ansi from "strip-ansi";
test("strip-ansi", () => {
  assert.ok(strip_ansi);
});

import * as strip_bom from "strip-bom";
test("strip-bom", () => {
  assert.ok(strip_bom);
});

import * as strip_json_comments from "strip-json-comments";
test("strip-json-comments", () => {
  assert.ok(strip_json_comments);
});

import * as supports_color from "supports-color";
test("supports-color", () => {
  assert.ok(supports_color);
});

import * as through2 from "through2";
test("through2", () => {
  assert.ok(through2);
});

import * as tmp from "tmp";
test("tmp", () => {
  assert.ok(tmp);
});

import * as ts_node from "ts-node";
test("ts-node", () => {
  assert.ok(ts_node);
});

import * as tslib from "tslib";
test("tslib", () => {
  assert.ok(tslib);
});

import * as typescript from "typescript";
test("typescript", () => {
  assert.ok(typescript);
});

import * as universalify from "universalify";
test("universalify", () => {
  assert.ok(universalify);
});

import * as uuid from "uuid";
test("uuid", () => {
  assert.ok(uuid);
});

import * as vue from "vue";
test("vue", () => {
  assert.ok(vue);
});

import * as webpack from "webpack";
test("webpack", () => {
  assert.ok(webpack);
});

import * as whatwg_url from "whatwg-url";
test("whatwg-url", () => {
  assert.ok(whatwg_url);
});

import * as which from "which";
test("which", () => {
  assert.ok(which);
});

import * as wrap_ansi from "wrap-ansi";
test("wrap-ansi", () => {
  assert.ok(wrap_ansi);
});

import * as wrappy from "wrappy";
test("wrappy", () => {
  assert.ok(wrappy);
});

import * as ws from "ws";
test("ws", () => {
  assert.ok(ws);
});

import * as xml2js from "xml2js";
test("xml2js", () => {
  assert.ok(xml2js);
});

import * as yallist from "yallist";
test("yallist", () => {
  assert.ok(yallist);
});

import * as yaml from "yaml";
test("yaml", () => {
  assert.ok(yaml);
});

import * as yargs from "yargs";
test("yargs", () => {
  assert.ok(yargs);
});

import * as yargs_parser from "yargs-parser";
test("yargs-parser", () => {
  assert.ok(yargs_parser);
});

import * as yocto_queue from "yocto-queue";
test("yocto-queue", () => {
  assert.ok(yocto_queue);
});

import * as zod from "zod";
test("zod", () => {
  assert.ok(zod);
});
