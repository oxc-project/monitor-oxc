use std::{path::Path, process::Command};

use oxc::{codegen::Codegen, span::SourceType, transformer::HelperLoaderMode};

use super::*;

fn source(path: &str, text: &str) -> Source {
    Source {
        path: path.into(),
        source_type: SourceType::from_path(path).unwrap(),
        source_text: text.into(),
    }
}

fn check(variant: &Variant, input: &Source) {
    if let Err(errors) = variant.check(input) {
        panic!("{}: {}: {errors:?}", variant.name, input.path.display());
    }
}

// Use fresh VM contexts / ESM module instances within one Node process. This
// keeps the runtime matrix cheap while isolating each original/generated run.
fn execute_many(inputs: &[(String, bool)]) -> Vec<String> {
    let script = format!(
        r"
        import vm from 'node:vm';
        import {{ format }} from 'node:util';
        const inputs = {};
        const outputs = [];
        for (const [index, [text, moduleMode]] of inputs.entries()) {{
            const lines = [];
            const log = (...args) => lines.push(format(...args));
            if (moduleMode) {{
                const originalLog = console.log;
                console.log = log;
                try {{
                    await import('data:text/javascript,' + encodeURIComponent(text) + '#' + index);
                }} finally {{ console.log = originalLog; }}
            }} else {{
                const module = {{ exports: {{}} }};
                vm.runInNewContext(text, {{ console: {{ log }}, module, exports: module.exports }}, {{ timeout: 5000 }});
            }}
            outputs.push(lines.join('\n'));
        }}
        process.stdout.write(JSON.stringify(outputs));
    ",
        serde_json::to_string(inputs).unwrap()
    );
    let result = Command::new("node")
        .args(["--input-type=module", "--eval", &script])
        .output()
        .expect("node is required for the runtime fixtures");
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    serde_json::from_slice(&result.stdout).unwrap()
}

fn execute(text: &str, module: bool) -> String {
    execute_many(&[(text.into(), module)]).pop().unwrap()
}

#[test]
fn codegen_syntax_matrix() {
    for variant in codegen() {
        for (path, text) in [
            (
                "input.js",
                "#!/usr/bin/env node\n/*! @license fixture */\n/** docs */\n// normal\nconst café = 'héllo 😀';\nconst value = /* @__PURE__ */ factory();",
            ),
            ("input.mjs", "import value from 'pkg' with { type: 'json' }; export { value };"),
            (
                "input.cjs",
                "module.exports = function (value) { return /é/u.test(value) ? `é${value}` : '\\u2028'; };",
            ),
            ("input.jsx", "const view = <><div title='é'>{value?.name}</div></>;"),
            ("input.ts", "interface Value { field?: string } export const value: Value = {};"),
            (
                "input.tsx",
                "export const View = (props: { value: string }) => <div>{props.value}</div>;",
            ),
            ("input.mts", "export enum Kind { First, Second }"),
            ("input.cts", "const value: number = 1; export = value;"),
            ("input.d.ts", "export declare class Value { readonly field: string; }"),
        ] {
            check(&variant, &source(path, text));
        }
    }
}

#[test]
fn codegen_maps_and_legal_comment_outputs() {
    let text = "/*! @license fixture */\nexport const café = 'héllo';";
    for variant in codegen() {
        let options = variant.options.codegen.unwrap();
        let allocator = Allocator::default();
        let parsed = Parser::new(&allocator, text, SourceType::mjs()).parse();
        let output = Codegen::new().with_options(options.clone()).build(&parsed.program);
        assert_eq!(output.map.is_some(), options.source_map_path.is_some(), "{}", variant.name);
        if let Some(map) = output.map {
            assert_eq!(map.get_source(0), Some("input.js"));
            assert_eq!(map.get_source_content(0), Some(text));
            assert!(map.get_tokens().next().is_some());
        }
        if matches!(options.comments.legal, LegalComment::Linked(_) | LegalComment::External) {
            assert_eq!(output.legal_comments.len(), 1, "{}", variant.name);
        }
    }
}

#[test]
fn transformer_syntax_matrix() {
    for variant in transformer() {
        for (path, text) in [
            (
                "input.js",
                "async function work(value) { return await value?.next() ?? 0; } async function* stream() { yield await work({next: () => 1}); }",
            ),
            (
                "input.mjs",
                "export class Value { #field = 1; static count = 0; static { this.count++; } read() { return this.#field; } }",
            ),
            ("input.cjs", "const value = {...{x: 1}}; value.x ||= 2; module.exports = value;"),
            ("input.jsx", "export function View(props) { return <><div>{props.value}</div></>; }"),
            (
                "input.ts",
                "import type { Type } from 'types'; import { Unused } from 'pkg'; namespace N { export const value: number = 1; } const enum Kind { First, Second } enum Other { A, B } class Value { field: number; initialized = Kind.Second; } export { N, Value, Other };",
            ),
            (
                "input.tsx",
                "export const View = (props: { value: string }) => <div>{props.value}</div>;",
            ),
            ("input.mts", "export const value: number = 1;"),
            ("input.cts", "const value: number = 1; module.exports = value;"),
        ] {
            check(&variant, &source(path, text));
        }
    }
}

#[test]
fn target_lowering_and_refresh_are_exercised() {
    let input = source(
        "input.mjs",
        "export async function work(value) { return await value?.next() ?? 0; }",
    );
    let variants = transformer();
    let old = variants[0].driver().run(&input.path, &input.source_text, input.source_type).unwrap();
    assert!(!old.contains("async function"));
    assert!(!old.contains("?."));
    assert!(old.contains("babelHelpers.asyncToGenerator"));
    let modern =
        variants[4].driver().run(&input.path, &input.source_text, input.source_type).unwrap();
    assert!(modern.contains("async function"));
    assert!(modern.contains("?."));

    let input = source("input.jsx", "export function View() { return <div />; }");
    let refresh = variants.last().unwrap();
    let output = refresh.driver().run(&input.path, &input.source_text, input.source_type).unwrap();
    assert!(output.contains("$RefreshReg$"));
    check(refresh, &input);
}

#[test]
fn pipeline_and_codegen_runtime_matrix() {
    let fixtures = [
        source(
            "input.cjs",
            "const café = 'héllo 😀'; function calc(value) { let total = 0; for (let i = 0; i < value; i++) total += i; return total; } module.exports = { calc }; console.log(café, calc(5), /é/u.test('é'), `hello${café}`);",
        ),
        source(
            "input.mjs",
            "export function calc({ value = 2, ...rest } = {}) { return [value ** 2, rest.extra ?? 'none']; } console.log(JSON.stringify(calc({ extra: 'yes' })), JSON.stringify(calc()));",
        ),
    ];
    for input in fixtures {
        let module = input.source_type.is_module();
        let mut inputs = vec![(input.source_text.clone(), module)];
        let mut names = vec![];
        for variant in codegen().into_iter().chain(minifier()) {
            check(&variant, &input);
            let output =
                variant.driver().run(&input.path, &input.source_text, input.source_type).unwrap();
            inputs.push((output, module));
            names.push(variant.name);
        }
        let outputs = execute_many(&inputs);
        for (name, actual) in names.iter().zip(&outputs[1..]) {
            assert_eq!(actual, &outputs[0], "{name}");
        }
    }
}

#[test]
fn transformer_runtime_matrix() {
    // These features lower without imported helpers, so this oracle uses only
    // Node and works in the build job before installing npm dependencies.
    let text = "let calls = 0; const value = { next() { calls++; return { value: 3 }; } }; let result = value.next()?.value ?? 7; result ||= 9; result &&= 4; result ??= 8; console.log(result ** 2, calls, value.missing?.value ?? 'fallback');";
    let mut inputs = vec![(text.into(), false)];
    let mut names = vec![];
    for variant in transformer() {
        for path in ["input.cjs", "input.mjs"] {
            let input = source(path, text);
            check(&variant, &input);
            for minify in [false, true] {
                let mut driver = variant.driver();
                driver.compress = minify.then(CompressOptions::default);
                driver.mangle = minify;
                driver.remove_whitespace = minify;
                let output = driver.run(&input.path, text, input.source_type).unwrap();
                inputs.push((output, input.source_type.is_module()));
                names.push(format!("{} {} minify={minify}", variant.name, path));
            }
        }
    }
    let outputs = execute_many(&inputs);
    for (name, actual) in names.iter().zip(&outputs[1..]) {
        assert_eq!(actual, &outputs[0], "{name}");
    }
}

#[test]
fn helper_modes_and_jsx_modes() {
    for mode in [HelperLoaderMode::Runtime, HelperLoaderMode::External] {
        let mut options = TransformOptions::from_target("es2015").unwrap();
        options.helper_loader.mode = mode;
        let output = Driver { transform: Some(options), ..Driver::default() }
            .run(
                Path::new("input.mjs"),
                "export async function work() { return await 1; }",
                SourceType::mjs(),
            )
            .unwrap();
        assert!(match mode {
            HelperLoaderMode::Runtime =>
                output.contains("@oxc-project/runtime/helpers/asyncToGenerator"),
            HelperLoaderMode::External => output.contains("babelHelpers.asyncToGenerator"),
            HelperLoaderMode::Inline => unreachable!(),
        });
    }
    for variant in transformer() {
        let options = variant.options.transform.as_ref().unwrap();
        let output = variant
            .driver()
            .run(Path::new("input.tsx"), "export const view = <div />;", SourceType::tsx())
            .unwrap();
        if !options.jsx.jsx_plugin {
            assert!(output.contains("<div"), "{}", variant.name);
        } else if options.jsx.runtime == JsxRuntime::Classic {
            assert!(
                output.contains(options.jsx.pragma.as_deref().unwrap_or("React.createElement")),
                "{}",
                variant.name
            );
        } else if options.jsx.development {
            assert!(output.contains("jsxDEV"), "{}", variant.name);
        } else {
            assert!(output.contains("jsx("), "{}", variant.name);
        }
    }
}

#[test]
fn typescript_and_decorator_options_change_output() {
    let variants = transformer();
    let text = "import { value } from './value.ts'; export { value };";
    for (name, expected) in [
        ("Transformer(rewrite-import-extensions)", "./value.js"),
        ("Transformer(remove-import-extensions)", "./value"),
    ] {
        let variant = variants.iter().find(|variant| variant.name == name).unwrap();
        let input = source("input.ts", text);
        check(variant, &input);
        let output = variant.driver().run(&input.path, text, input.source_type).unwrap();
        assert!(output.contains(&format!("\"{expected}\"")), "{output}");
    }
    let text = "function decorate(value: any) { return value; } @decorate export class Value { constructor(value: number) {} field: number = 1; }";
    for metadata in [false, true] {
        let mut options = crate::transformer::transform_options();
        options.decorator.emit_decorator_metadata = metadata;
        options.helper_loader.mode = HelperLoaderMode::External;
        let input = source("input.ts", text);
        let variant =
            Variant::new("decorators", Driver { transform: Some(options), ..Driver::default() });
        check(&variant, &input);
        let output = variant.driver().run(&input.path, text, input.source_type).unwrap();
        assert!(output.contains("decorate"));
        // Constructor metadata is emitted for a decorated class.
        assert_eq!(output.contains("design:paramtypes"), metadata, "{output}");
    }
}

#[test]
fn mangler_keep_names_preserves_observable_names() {
    let text = "function NamedFunction() {} class NamedClass {} console.log(NamedFunction.name, NamedClass.name);";
    let variant =
        minifier().into_iter().find(|variant| variant.name == "Mangler(keep-names)").unwrap();
    let input = source("input.mjs", text);
    check(&variant, &input);
    let output = variant.driver().run(&input.path, text, input.source_type).unwrap();
    assert_eq!(execute(&output, true), execute(text, true));
}

#[test]
fn expected_target_warnings_do_not_hide_errors() {
    for ignore in [false, true] {
        let mut driver = Driver {
            transform: Some(TransformOptions::from_target("es2015").unwrap()),
            ignore_target_warnings: ignore,
            ..Driver::default()
        };
        assert_eq!(
            driver
                .run(Path::new("input.mjs"), "export const value = 1n;", SourceType::mjs())
                .is_ok(),
            ignore
        );
        assert!(driver.run(Path::new("input.mjs"), "export const = ;", SourceType::mjs()).is_err());
    }
}
