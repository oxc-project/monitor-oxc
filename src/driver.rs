use std::path::Path;

use oxc::{
    allocator::Allocator,
    codegen::{CodeGenerator, CodegenOptions, CommentOptions},
    mangler::ManglerBuilder,
    minifier::{CompressOptions, Compressor},
    parser::{ParseOptions, Parser, ParserReturn},
    span::SourceType,
    transformer::{TransformOptions, Transformer},
};

use crate::Diagnostic;

#[allow(clippy::struct_excessive_bools)]
#[derive(Default)]
pub struct Driver {
    transform: bool,
    compress: bool,
    mangle: bool,
    remove_whitespace: bool,
}

impl Driver {
    #[must_use]
    pub fn with_transform(mut self) -> Self {
        self.transform = true;
        self
    }

    #[must_use]
    pub fn with_compress(mut self) -> Self {
        self.compress = true;
        self
    }

    #[must_use]
    pub fn with_mangle(mut self) -> Self {
        self.transform = true;
        self
    }

    #[must_use]
    pub fn with_remove_whitespace(mut self) -> Self {
        self.remove_whitespace = true;
        self
    }

    pub fn run(
        self,
        path: &Path,
        source_text: &str,
        source_type: SourceType,
    ) -> Result<String, Diagnostic> {
        let allocator = Allocator::default();
        let ParserReturn { mut program, errors, trivias, .. } =
            Parser::new(&allocator, source_text, source_type)
                .with_options(ParseOptions {
                    allow_return_outside_function: true,
                    ..ParseOptions::default()
                })
                .parse();

        if !errors.is_empty() {
            let message = errors
                .into_iter()
                .map(|e| e.with_source_code(source_text.to_string()).to_string())
                .collect::<Vec<_>>()
                .join("\n");
            // ignore flow files
            if message.contains("Flow is not supported") {
                return Ok(String::new());
            }
            return Err(Diagnostic { case: "Parse Error", path: path.to_path_buf(), message });
        }

        if self.transform {
            Transformer::new(
                &allocator,
                path,
                source_type,
                source_text,
                trivias.clone(),
                TransformOptions::default(),
            )
            .build(&mut program);
        }

        if self.compress {
            Compressor::new(&allocator, CompressOptions::default()).build(&mut program);
        }

        let mangler = self.mangle.then(|| ManglerBuilder::default().debug(true).build(&program));

        let comment_options = CommentOptions { preserve_annotate_comments: true };

        let source = CodeGenerator::new()
            .with_options(CodegenOptions {
                minify: self.remove_whitespace,
                ..CodegenOptions::default()
            })
            .enable_comment(source_text, trivias, comment_options)
            .with_mangler(mangler)
            .build(&program)
            .source_text;

        Ok(source)
    }
}
