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

// types
// import "@types/express";
// import "@types/node";
// import "type-fest";

// has many esm exports
// import "@babel/runtime";

import * as typescript_eslint_eslint_plugin from "@typescript-eslint/eslint-plugin";
test("@typescript-eslint/eslint-plugin", () => {
  assert.ok(typescript_eslint_eslint_plugin);
});

import * as typescript_eslint_parser from "@typescript-eslint/parser";
test("@typescript-eslint/parser", () => {
  assert.ok(typescript_eslint_parser);
});

import * as acorn from "acorn";
test("acorn", () => {
  assert.ok(acorn);
});

import * as ajv from "ajv";
test("ajv", () => {
  assert.ok(ajv);
});

import * as ansi_regex from "ansi-regex";
test("ansi-regex", () => {
  assert.ok(ansi_regex);
});

import * as ansi_styles from "ansi-styles";
test("ansi-styles", () => {
  assert.ok(ansi_styles);
});

import * as argparse from "argparse";
test("argparse", () => {
  assert.ok(argparse);
});

import * as async from "async";
test("async", () => {
  assert.ok(async);
});

import * as brace_expansion from "brace-expansion";
test("brace-expansion", () => {
  assert.ok(brace_expansion);
});

import * as buffer from "buffer";
test("buffer", () => {
  assert.ok(buffer);
});

import * as camelcase from "camelcase";
test("camelcase", () => {
  assert.ok(camelcase);
});

import * as chalk from "chalk";
test("chalk", () => {
  assert.ok(chalk);
});

import * as chokidar from "chokidar";
test("chokidar", () => {
  assert.ok(chokidar);
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

import * as cross_spawn from "cross-spawn";
test("cross-spawn", () => {
  assert.ok(cross_spawn);
});

import * as debug from "debug";
test("debug", () => {
  assert.ok(debug);
});

import * as deepmerge from "deepmerge";
test("deepmerge", () => {
  assert.ok(deepmerge);
});

import * as dotenv from "dotenv";
test("dotenv", () => {
  assert.ok(dotenv);
});

import * as emoji_regex from "emoji-regex";
test("emoji-regex", () => {
  assert.ok(emoji_regex);
});

import * as escape_string_regexp from "escape-string-regexp";
test("escape-string-regexp", () => {
  assert.ok(escape_string_regexp);
});

import * as eslint from "eslint";
test("eslint", () => {
  assert.ok(eslint);
});

import * as eslint_plugin_import from "eslint-plugin-import";
test("eslint-plugin-import", () => {
  assert.ok(eslint_plugin_import);
});

import * as eventemitter3 from "eventemitter3";
test("eventemitter3", () => {
  assert.ok(eventemitter3);
});

import * as execa from "execa";
test("execa", () => {
  assert.ok(execa);
});

import * as express from "express";
test("express", () => {
  assert.ok(express);
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

import * as has_flag from "has-flag";
test("has-flag", () => {
  assert.ok(has_flag);
});

import * as https_proxy_agent from "https-proxy-agent";
test("https-proxy-agent", () => {
  assert.ok(https_proxy_agent);
});

import * as iconv_lite from "iconv-lite";
test("iconv-lite", () => {
  assert.ok(iconv_lite);
});

import * as inquirer from "inquirer";
test("inquirer", () => {
  assert.ok(inquirer);
});

import * as is_fullwidth_code_point from "is-fullwidth-code-point";
test("is-fullwidth-code-point", () => {
  assert.ok(is_fullwidth_code_point);
});

import * as isarray from "isarray";
test("isarray", () => {
  assert.ok(isarray);
});

import * as js_tokens from "js-tokens";
test("js-tokens", () => {
  assert.ok(js_tokens);
});

import * as js_yaml from "js-yaml";
test("js-yaml", () => {
  assert.ok(js_yaml);
});

import * as json5 from "json5";
test("json5", () => {
  assert.ok(json5);
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

import * as lru_cache from "lru-cache";
test("lru-cache", () => {
  assert.ok(lru_cache);
});

import * as mime from "mime";
test("mime", () => {
  assert.ok(mime);
});

import * as minimatch from "minimatch";
test("minimatch", () => {
  assert.ok(minimatch);
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

import * as object_assign from "object-assign";
test("object-assign", () => {
  assert.ok(object_assign);
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

import * as path_to_regexp from "path-to-regexp";
test("path-to-regexp", () => {
  assert.ok(path_to_regexp);
});

import * as picocolors from "picocolors";
test("picocolors", () => {
  assert.ok(picocolors);
});

import * as postcss from "postcss";
test("postcss", () => {
  assert.ok(postcss);
});

import * as prettier from "prettier";
test("prettier", () => {
  assert.ok(prettier);
});

import * as prop_types from "prop-types";
test("prop-types", () => {
  assert.ok(prop_types);
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

import * as regenerator_runtime from "regenerator-runtime";
test("regenerator-runtime", () => {
  assert.ok(regenerator_runtime);
});

import * as resolve from "resolve";
test("resolve", () => {
  assert.ok(resolve);
});

import * as rimraf from "rimraf";
test("rimraf", () => {
  assert.ok(rimraf);
});

import * as rxjs from "rxjs";
test("rxjs", () => {
  assert.ok(rxjs);
});

import * as semver from "semver";
test("semver", () => {
  assert.ok(semver);
});

import * as signal_exit from "signal-exit";
test("signal-exit", () => {
  assert.ok(signal_exit);
});

import * as slash from "slash";
test("slash", () => {
  assert.ok(slash);
});

import * as source_map from "source-map";
test("source-map", () => {
  assert.ok(source_map);
});

import * as source_map_support from "source-map-support";
test("source-map-support", () => {
  assert.ok(source_map_support);
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

import * as supports_color from "supports-color";
test("supports-color", () => {
  assert.ok(supports_color);
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

import * as uuid from "uuid";
test("uuid", () => {
  assert.ok(uuid);
});

import * as webpack from "webpack";
test("webpack", () => {
  assert.ok(webpack);
});

import * as which from "which";
test("which", () => {
  assert.ok(which);
});

import * as wrap_ansi from "wrap-ansi";
test("wrap-ansi", () => {
  assert.ok(wrap_ansi);
});

import * as ws from "ws";
test("ws", () => {
  assert.ok(ws);
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
