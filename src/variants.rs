//! Additional corpus configurations. These never rewrite dependencies: external
//! helpers, preserved JSX and Fast Refresh output are not runtime entry points.
use std::path::PathBuf;

use oxc::{
    allocator::Allocator,
    codegen::{CodegenOptions, CommentOptions, IndentChar, LegalComment},
    mangler::{MangleOptions, MangleOptionsKeepNames},
    minifier::CompressOptions,
    parser::{ParseOptions, Parser},
    transformer::{
        HelperLoaderMode, JsxOptions, JsxRuntime, ReactRefreshOptions, RewriteExtensionsMode,
        TransformOptions,
    },
};

use crate::{Case, Diagnostic, Driver, NodeModulesRunner, Source};

/// Run every configuration for a file even when an earlier one fails. This
/// keeps a bug in one variant from hiding the rest of the matrix.
pub struct Suite {
    name: &'static str,
    variants: Vec<Variant>,
}

impl Suite {
    pub fn new(name: &'static str, variants: Vec<Variant>) -> Self {
        Self { name, variants }
    }
}

impl Case for Suite {
    fn name(&self) -> &'static str {
        self.name
    }

    fn enable_runtime_test(&self) -> bool {
        false
    }

    fn driver(&self) -> Driver {
        unreachable!()
    }

    fn test(&self, source: &Source) -> Result<Option<String>, Vec<Diagnostic>> {
        let mut diagnostics = vec![];
        for variant in &self.variants {
            if let Err(errors) = variant.check(source) {
                diagnostics.extend(errors);
            }
        }
        if diagnostics.is_empty() { Ok(None) } else { Err(diagnostics) }
    }
}

#[derive(Clone)]
pub struct Variant {
    name: &'static str,
    options: Driver,
    idempotent: bool,
    js_only: bool,
}

impl Variant {
    fn new(name: &'static str, options: Driver) -> Self {
        Self { name, options, idempotent: true, js_only: false }
    }

    fn check(&self, source: &Source) -> Result<(), Vec<Diagnostic>> {
        if self.js_only && !source.is_js_only() {
            return Ok(());
        }
        let output = self
            .driver()
            .run(&source.path, &source.source_text, source.source_type)
            .map_err(|mut errors| {
                for error in &mut errors {
                    error.case = self.name;
                }
                errors
            })?;
        // Reparse transformed TS as JavaScript so leftover type syntax cannot
        // silently pass a second TypeScript parse.
        let (path, source_type) = if let Some(transform) = &self.options.transform {
            let extension = source.path.extension().unwrap().to_string_lossy().replace('t', "j");
            (
                source.path.with_extension(extension),
                source.source_type.with_typescript(false).with_jsx(!transform.jsx.jsx_plugin),
            )
        } else {
            (source.path.clone(), source.source_type)
        };
        let source_type = if path.extension().is_some_and(|extension| extension == "js") {
            source_type.with_unambiguous(true).with_jsx(
                self.options.transform.as_ref().is_none_or(|options| !options.jsx.jsx_plugin),
            )
        } else {
            source_type
        };
        let allocator = Allocator::default();
        let parsed = Parser::new(&allocator, &output, source_type)
            .with_options(ParseOptions {
                allow_return_outside_function: true,
                parse_regular_expression: true,
                ..ParseOptions::default()
            })
            .parse();
        if !parsed.diagnostics.is_empty() {
            return Err(vec![Diagnostic {
                case: self.name,
                path: source.path.clone(),
                message: format!("Generated output failed to parse: {:?}", parsed.diagnostics),
            }]);
        }
        if self.idempotent {
            let second = self.driver().run(&path, &output, source_type)?;
            if output != second {
                return Err(vec![Diagnostic {
                    case: self.name,
                    path: source.path.clone(),
                    message: NodeModulesRunner::get_diff(&output, &second, false),
                }]);
            }
        }
        Ok(())
    }
}

impl Case for Variant {
    fn name(&self) -> &'static str {
        self.name
    }

    fn enable_runtime_test(&self) -> bool {
        false
    }

    fn test(&self, source: &Source) -> Result<Option<String>, Vec<Diagnostic>> {
        self.check(source)?;
        Ok(None)
    }

    fn driver(&self) -> Driver {
        self.options.clone()
    }
}

/// Cross quote style, whitespace, ASCII escaping and source-map generation.
/// Add comment policies and indentation separately to keep corpus cost bounded.
pub fn codegen() -> Vec<Variant> {
    const NAMES: [&str; 16] = [
        "Codegen(double,pretty,unicode,no-map)",
        "Codegen(single,pretty,unicode,no-map)",
        "Codegen(double,minify,unicode,no-map)",
        "Codegen(single,minify,unicode,no-map)",
        "Codegen(double,pretty,ascii,no-map)",
        "Codegen(single,pretty,ascii,no-map)",
        "Codegen(double,minify,ascii,no-map)",
        "Codegen(single,minify,ascii,no-map)",
        "Codegen(double,pretty,unicode,map)",
        "Codegen(single,pretty,unicode,map)",
        "Codegen(double,minify,unicode,map)",
        "Codegen(single,minify,unicode,map)",
        "Codegen(double,pretty,ascii,map)",
        "Codegen(single,pretty,ascii,map)",
        "Codegen(double,minify,ascii,map)",
        "Codegen(single,minify,ascii,map)",
    ];
    let mut variants: Vec<_> = NAMES
        .iter()
        .enumerate()
        .map(|(bits, name)| {
            Variant::new(
                name,
                Driver {
                    codegen: Some(CodegenOptions {
                        single_quote: bits & 1 != 0,
                        minify: bits & 2 != 0,
                        ascii_only: bits & 4 != 0,
                        source_map_path: (bits & 8 != 0).then(|| PathBuf::from("input.js")),
                        ..CodegenOptions::default()
                    }),
                    ..Driver::default()
                },
            )
        })
        .collect();
    for (name, comments) in [
        ("Codegen(no-comments)", CommentOptions::disabled()),
        (
            "Codegen(annotations-only)",
            CommentOptions { annotation: true, ..CommentOptions::disabled() },
        ),
        ("Codegen(jsdoc-only)", CommentOptions { jsdoc: true, ..CommentOptions::disabled() }),
        ("Codegen(normal-only)", CommentOptions { normal: true, ..CommentOptions::disabled() }),
        (
            "Codegen(legal-eof)",
            CommentOptions { legal: LegalComment::Eof, ..CommentOptions::disabled() },
        ),
        (
            "Codegen(legal-linked)",
            CommentOptions {
                legal: LegalComment::Linked("input.LEGAL.txt".into()),
                ..CommentOptions::disabled()
            },
        ),
        (
            "Codegen(legal-external)",
            CommentOptions { legal: LegalComment::External, ..CommentOptions::disabled() },
        ),
    ] {
        variants.push(Variant::new(
            name,
            Driver {
                codegen: Some(CodegenOptions { comments, ..CodegenOptions::default() }),
                ..Driver::default()
            },
        ));
    }
    variants.push(Variant::new(
        "Codegen(spaces,initial-indent)",
        Driver {
            codegen: Some(CodegenOptions {
                indent_char: IndentChar::Space,
                indent_width: 2,
                initial_indent: 1,
                ..CodegenOptions::default()
            }),
            ..Driver::default()
        },
    ));
    variants
}

/// Target-based lowering includes async transforms disabled in the runtime
/// preset. External helpers avoid adding ESM imports to `CommonJS` corpus files.
pub fn transformer() -> Vec<Variant> {
    let mut variants = vec![];
    for (name, target) in [
        ("Transformer(es2015)", "es2015"),
        ("Transformer(es2018)", "es2018"),
        ("Transformer(es2020)", "es2020"),
        ("Transformer(es2022)", "es2022"),
        ("Transformer(esnext)", "esnext"),
    ] {
        let mut options = TransformOptions::from_target(target).unwrap();
        options.helper_loader.mode = HelperLoaderMode::External;
        variants.push(Variant::new(
            name,
            Driver { transform: Some(options), ignore_target_warnings: true, ..Driver::default() },
        ));
    }
    for (name, runtime, development) in [
        ("Transformer(automatic,production)", JsxRuntime::Automatic, false),
        ("Transformer(automatic,development)", JsxRuntime::Automatic, true),
        ("Transformer(classic,production)", JsxRuntime::Classic, false),
        ("Transformer(classic,development)", JsxRuntime::Classic, true),
    ] {
        let mut options = crate::transformer::transform_options();
        options.jsx.runtime = runtime;
        options.jsx.development = development;
        // enable_all sets the development plugins as well as the flag.
        options.jsx.jsx_self_plugin = development;
        options.jsx.jsx_source_plugin = development;
        variants.push(Variant::new(name, Driver { transform: Some(options), ..Driver::default() }));
    }
    let preserve = TransformOptions { jsx: JsxOptions::disable(), ..TransformOptions::default() };
    variants.push(Variant::new(
        "Transformer(preserve-jsx)",
        Driver { transform: Some(preserve), ..Driver::default() },
    ));
    let mut typescript = crate::transformer::transform_options();
    typescript.typescript.only_remove_type_imports = false;
    typescript.typescript.optimize_const_enums = true;
    typescript.typescript.optimize_enums = true;
    typescript.typescript.remove_class_fields_without_initializer = true;
    typescript.assumptions.set_public_class_fields = true;
    variants.push(Variant::new(
        "Transformer(typescript,assignment-fields,enums)",
        Driver { transform: Some(typescript), ..Driver::default() },
    ));
    let mut all = TransformOptions::enable_all();
    all.jsx.refresh = None;
    all.helper_loader.mode = HelperLoaderMode::External;
    variants.push(Variant::new(
        "Transformer(all,external-helpers)",
        Driver { transform: Some(all), ..Driver::default() },
    ));
    for (name, mode) in [
        ("Transformer(rewrite-import-extensions)", RewriteExtensionsMode::Rewrite),
        ("Transformer(remove-import-extensions)", RewriteExtensionsMode::Remove),
    ] {
        let mut options = TransformOptions::default();
        options.typescript.rewrite_import_extensions = Some(mode);
        variants.push(Variant::new(name, Driver { transform: Some(options), ..Driver::default() }));
    }
    let mut classic = TransformOptions::default();
    classic.jsx.runtime = JsxRuntime::Classic;
    classic.jsx.pragma = Some("h".into());
    classic.jsx.pragma_frag = Some("Fragment".into());
    classic.jsx.throw_if_namespace = false;
    classic.jsx.pure = false;
    variants.push(Variant::new(
        "Transformer(custom-classic-jsx)",
        Driver { transform: Some(classic), ..Driver::default() },
    ));
    let mut decorator = crate::transformer::transform_options();
    decorator.decorator.emit_decorator_metadata = false;
    variants.push(Variant::new(
        "Transformer(decorators,no-metadata)",
        Driver { transform: Some(decorator), ..Driver::default() },
    ));
    let mut refresh = TransformOptions::default();
    refresh.jsx.development = true;
    refresh.jsx.refresh = Some(ReactRefreshOptions::default());
    let mut refresh = Variant::new(
        "Transformer(refresh,reparse)",
        Driver { transform: Some(refresh), ..Driver::default() },
    );
    refresh.idempotent = false;
    variants.push(refresh);
    variants
}

/// All combinations of compression, mangling and whitespace, plus the safest
/// compressor preset. Corpus checks are syntax/idempotency only; fixtures below
/// execute the same configurations to check observable behavior.
pub fn minifier() -> Vec<Variant> {
    const NAMES: [&str; 8] = [
        "Pipeline(codegen)",
        "Pipeline(compress)",
        "Pipeline(mangle)",
        "Pipeline(compress,mangle)",
        "Pipeline(whitespace)",
        "Pipeline(compress,whitespace)",
        "Pipeline(mangle,whitespace)",
        "Pipeline(compress,mangle,whitespace)",
    ];
    let mut variants: Vec<_> = NAMES
        .iter()
        .enumerate()
        .map(|(bits, name)| {
            let mut variant = Variant::new(
                name,
                Driver {
                    compress: (bits & 1 != 0).then(CompressOptions::default),
                    mangle: bits & 2 != 0,
                    remove_whitespace: bits & 4 != 0,
                    ..Driver::default()
                },
            );
            variant.js_only = true;
            variant
        })
        .collect();
    let mut safest = Variant::new(
        "Compressor(safest)",
        Driver { compress: Some(CompressOptions::safest()), ..Driver::default() },
    );
    safest.js_only = true;
    variants.push(safest);
    for (name, options) in [
        (
            "Mangler(keep-names)",
            MangleOptions {
                keep_names: MangleOptionsKeepNames::all_true(),
                ..MangleOptions::default()
            },
        ),
        (
            "Mangler(no-top-level)",
            MangleOptions { top_level: Some(false), ..MangleOptions::default() },
        ),
        ("Mangler(debug)", MangleOptions { debug: true, ..MangleOptions::default() }),
    ] {
        let mut variant =
            Variant::new(name, Driver { mangle_options: Some(options), ..Driver::default() });
        variant.js_only = true;
        variants.push(variant);
    }
    variants
}

#[cfg(test)]
mod tests;
