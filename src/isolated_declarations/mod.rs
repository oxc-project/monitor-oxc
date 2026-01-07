use std::{fs, path::PathBuf, process::ExitCode};

use project_root::get_project_root;
use walkdir::WalkDir;

use crate::NodeModulesRunner;

use oxc::{
    allocator::Allocator,
    codegen::{Codegen, CodegenOptions, CommentOptions},
    isolated_declarations::{IsolatedDeclarations, IsolatedDeclarationsOptions},
    parser::Parser,
    span::SourceType,
};

#[must_use]
pub fn test(path_to_vue: Option<PathBuf>) -> ExitCode {
    let root = path_to_vue.unwrap_or_else(|| get_project_root().unwrap().join("../core"));
    let temp_dir = root.join("temp");

    if !temp_dir.exists() {
        println!("Please provide path to vue repository.");
        println!("And run `tsc -p tsconfig.build.json --noCheck` in the vue repository.");
        return ExitCode::FAILURE;
    }

    let include = [
        // "packages/global.d.ts",
        "packages/vue/src",
        "packages/vue-compat/src",
        "packages/compiler-core/src",
        "packages/compiler-dom/src",
        "packages/runtime-core/src",
        "packages/runtime-dom/src",
        "packages/reactivity/src",
        "packages/shared/src",
        // "packages/global.d.ts",
        "packages/compiler-sfc/src",
        "packages/compiler-ssr/src",
        "packages/server-renderer/src",
    ];

    let mut exit_code = ExitCode::SUCCESS;
    for entry in WalkDir::new(root.join("packages")) {
        let dir_entry = entry.unwrap();
        let path = dir_entry.path();
        let path_str = path.to_string_lossy();
        if !include.iter().any(|i| path_str.contains(i)) {
            continue;
        }
        let Ok(source_type) = SourceType::from_path(path) else {
            continue;
        };
        if !source_type.is_typescript() || source_type.is_typescript_definition() {
            continue;
        }

        let source_text = fs::read_to_string(path).unwrap();
        let printed = {
            let allocator = Allocator::default();
            let ret = Parser::new(&allocator, &source_text, source_type).parse();
            let id = IsolatedDeclarations::new(
                &allocator,
                IsolatedDeclarationsOptions { strip_internal: true },
            )
            .build(&ret.program);
            Codegen::new()
                .with_options(CodegenOptions {
                    comments: CommentOptions { jsdoc: true, ..CommentOptions::disabled() },
                    ..CodegenOptions::default()
                })
                .build(&id.program)
                .code
        };

        let root_str = root.to_string_lossy();
        let read_path = temp_dir
            .join(path_str.strip_prefix(root_str.as_ref()).unwrap().strip_prefix("/").unwrap())
            .with_extension("")
            .with_extension("d.ts");

        let tsc_output = {
            let allocator = Allocator::default();
            let source_type = SourceType::d_ts();
            let source_text =
                fs::read_to_string(&read_path).unwrap_or_else(|e| panic!("{e}\n{read_path:?}"));
            let ret = Parser::new(&allocator, &source_text, source_type).parse();
            Codegen::new()
                .with_options(CodegenOptions {
                    comments: CommentOptions { jsdoc: true, ..CommentOptions::disabled() },
                    ..CodegenOptions::default()
                })
                .build(&ret.program)
                .code
        };

        if tsc_output.trim() != printed.trim() {
            exit_code = ExitCode::FAILURE;
            println!();
            println!("{}", path.to_string_lossy());
            println!("{}", NodeModulesRunner::get_diff(&printed, &tsc_output, true));
            println!();
        }
    }

    exit_code
}
