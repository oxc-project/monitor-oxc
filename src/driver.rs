use std::path::Path;

use oxc::{
    allocator::Allocator,
    codegen::{CodeGenerator, CommentOptions},
    mangler::ManglerBuilder,
    parser::{Parser, ParserReturn},
    span::SourceType,
    transformer::{TransformOptions, Transformer},
};

use crate::Diagnostic;

#[derive(Default)]
pub struct Driver {
    transform: bool,
    mangle: bool,
}

impl Driver {
    #[must_use]
    pub fn with_transform(mut self) -> Self {
        self.transform = true;
        self
    }

    #[must_use]
    pub fn with_mangle(mut self) -> Self {
        self.transform = true;
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
                .allow_return_outside_function(true)
                .parse();

        if !errors.is_empty() {
            let message = errors
                .into_iter()
                .map(|e| e.with_source_code(source_text.to_string()).to_string())
                .collect::<Vec<_>>()
                .join("\n");
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

        let mangler = self.mangle.then(|| ManglerBuilder::default().debug(true).build(&program));

        let source = CodeGenerator::new()
            .enable_comment(
                source_text,
                trivias,
                CommentOptions { preserve_annotate_comments: true },
            )
            .with_mangler(mangler)
            .build(&program)
            .source_text;

        Ok(source)
    }
}
