use oxc::{
    allocator::Allocator,
    parser::{Parser, ParserReturn},
};
use oxc_formatter::{FormatOptions, Formatter, detect_code_removal, get_parse_options};

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
        let options = get_parse_options();

        let ParserReturn { program, errors, .. } =
            Parser::new(&allocator, source_text, *source_type).with_options(options).parse();
        if !errors.is_empty() {
            // Skip files that fail to parse, already reported in `FormatterRunner`
            return Ok(());
        }

        let source_text2 = Formatter::new(&allocator, FormatOptions::default()).build(&program);

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
