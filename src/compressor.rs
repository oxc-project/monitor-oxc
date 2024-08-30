use std::path::{Path, PathBuf};

use oxc::span::SourceType;

use crate::{driver::default_transformer_options, Case, Driver};

pub struct CompressorRunner;

impl Case for CompressorRunner {
    fn name(&self) -> &'static str {
        "Compressor"
    }

    fn save_file(&self, path: &Path, source_type: SourceType) -> Option<PathBuf> {
        source_type.is_javascript().then(|| path.to_path_buf())
    }

    fn driver(&self) -> Driver {
        // always compress js files
        let mut transform = default_transformer_options();

        // The compressor will remove unreachable code and the typescript plugin has a feature to remove unused imports.
        // There is a conflict between these two features, so we need to disable the typescript plugin's feature.
        transform.typescript.only_remove_type_imports = true;

        Driver { transform: Some(transform), compress: true, ..Driver::default() }
    }
}
