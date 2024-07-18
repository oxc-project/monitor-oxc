use std::path::{Path, PathBuf};

use oxc::span::SourceType;

use crate::{Case, Driver};

pub struct CodegenRunner;

impl Case for CodegenRunner {
    fn name(&self) -> &'static str {
        "Codegen"
    }

    fn save_file(&self, path: &Path, source_type: SourceType) -> Option<PathBuf> {
        source_type.is_javascript().then(|| path.to_path_buf())
    }

    fn driver(&self) -> Driver {
        Driver::default()
    }
}
