use oxc::allocator::Allocator;
use oxc_formatter::{detect_code_removal, format_program, parse_for_format, JsFormatOptions};

use crate::{Case, Diagnostic, Driver, Source};

// Another `FormatterRunner` for detecting code removal.
//
// Detection api is enabled by the feature flag "detect_code_removal".
// While the main FormatterRunner also has this capability,
// there are currently many reported idempotency issues.
// Therefore, for clarity, we separate this test to focus only on detecting code removal.

pub struct FormatterDCRRunner;

impl Case for FormatterDCRRunner {
    fn name(&self) -> &'static str {
        "Formatter(DetectCodeRemoval)"
    }

    fn enable_runtime_test(&self) -> bool {
        false
    }

    fn driver(&self) -> Driver {
        unreachable!()
    }

    fn test(&self, source: &Source) -> Result<(), Vec<Diagnostic>> {
        let Source { path, source_type, source_text, .. } = source;

        let allocator = Allocator::new();
        let parsed = parse_for_format(&allocator, source_text, *source_type);
        let errors = parsed.errors;
        if !errors.is_empty() {
            // Skip files that fail to parse, already reported in `FormatterRunner`
            return Ok(());
        }

        let source_text2 =
            format_program(&allocator, &parsed.program, JsFormatOptions::default(), None)
                .print()
                .map_err(|err| {
                    vec![Diagnostic {
                        case: self.name(),
                        path: path.clone(),
                        message: format!("Format error: {err:?}"),
                    }]
                })?
                .into_code();

        if let Some(diff) = detect_code_removal(source_text, &source_text2, *source_type) {
            return Err(vec![Diagnostic {
                case: self.name(),
                path: path.clone(),
                message: format!("Code removal detected:\n{diff}"),
            }]);
        }

        Ok(())
    }
}
