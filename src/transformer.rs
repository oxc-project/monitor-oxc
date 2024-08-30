use std::path::{Path, PathBuf};

use oxc::span::SourceType;

use crate::{driver::default_transformer_options, Case, Driver};

pub struct TransformerRunner;

impl Case for TransformerRunner {
    fn name(&self) -> &'static str {
        "Transformer"
    }

    fn save_file(&self, path: &Path, _source_type: SourceType) -> Option<PathBuf> {
        let new_extension = path.extension().unwrap().to_string_lossy().replace('t', "j");
        let new_path = path.with_extension(new_extension);
        Some(new_path)
    }

    fn driver(&self) -> Driver {
        Driver { transform: Some(default_transformer_options()), ..Driver::default() }
    }
}
