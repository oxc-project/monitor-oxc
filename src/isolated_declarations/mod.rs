use std::{env, fmt, fs, path::PathBuf, process::Command, str::FromStr};

use oxc::{
    allocator::Allocator, codegen::CodeGenerator, isolated_declarations::IsolatedDeclarations,
    parser::Parser, span::SourceType,
};
use ureq::get;

const FILES: &[&str] = &[
    "https://raw.githubusercontent.com/vuejs/core/refactor/isolated-decl/packages/runtime-core/src/renderer.ts",
    "https://raw.githubusercontent.com/vuejs/core/refactor/isolated-decl/packages/runtime-core/src/hydration.ts"
];

pub fn current_dir() -> PathBuf {
    let dir = env::current_dir().unwrap();
    dir.join("src/isolated_declarations")
}

pub fn test_isolated_declarations() {
    for url in FILES {
        let (path, original) = get_source_text(url).expect("Failed to download file");
        write_file("input.ts", &original);
        let dts = transform(&path, &original);
        write_file("output.d.ts", &dts);
        compare_with_tsc();
    }
}

fn compare_with_tsc() {
    Command::new("pnpm")
        .args(["tsx", &current_dir().join("./tsc.ts").to_string_lossy()])
        .spawn()
        .expect("Failed to run tsc");
}

fn transform(path: &str, source_text: &str) -> String {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(path).unwrap();
    let program = Parser::new(&allocator, source_text, source_type)
        .allow_return_outside_function(true)
        .parse()
        .program;
    let ret = IsolatedDeclarations::new(&allocator).build(&program);
    CodeGenerator::new().build(&ret.program).source_text
}

fn write_file(path: &str, source_text: &str) {
    fs::write(current_dir().join(path), source_text)
        .unwrap_or_else(|_| panic!("Failed to write {path}"));
}

fn err_to_string<E: fmt::Debug>(e: E) -> String {
    format!("{e:?}")
}

fn get_source_text(lib: &str) -> Result<(String, String), String> {
    let url = url::Url::from_str(lib).map_err(err_to_string)?;

    let segments = url.path_segments().ok_or_else(|| "lib url has no segments".to_string())?;

    let filename = segments.last().ok_or_else(|| "lib url has no segments".to_string())?;

    let file = current_dir().join(filename);

    if let Ok(code) = std::fs::read_to_string(&file) {
        println!("[{filename}] - using [{filename}]");
        Ok((filename.to_string(), code))
    } else {
        println!("[{filename}] - Downloading [{lib}] to [{filename}]");
        match get(lib).call() {
            Ok(response) => {
                let mut reader = response.into_reader();

                let _drop = std::fs::remove_file(&file);
                let mut writer = std::fs::File::create(&file).map_err(err_to_string)?;
                let _drop = std::io::copy(&mut reader, &mut writer);

                std::fs::read_to_string(file)
                    .map_err(err_to_string)
                    .map(|code| (filename.to_string(), code))
            }
            Err(e) => Err(format!("{e:?}")),
        }
    }
}
