use std::path::{Path, PathBuf};

use oxc::span::SourceType;

use crate::{driver::default_transformer_options, Case, Driver};

pub struct ManglerRunner;

impl Case for ManglerRunner {
    fn name(&self) -> &'static str {
        "Mangler"
    }

    fn save_file(&self, path: &Path, source_type: SourceType) -> Option<PathBuf> {
        source_type.is_javascript().then(|| path.to_path_buf())
    }

    fn driver(&self) -> Driver {
        Driver { transform: Some(default_transformer_options()), mangle: true, ..Driver::default() }
    }
}
