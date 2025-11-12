use oxc::{
    allocator::Allocator,
    parser::{Parser, ParserReturn},
};
use oxc_formatter::{FormatOptions, Formatter, get_parse_options};

use crate::{Case, Diagnostic, Driver, NodeModulesRunner, Source};

pub struct FormatterRunner;

impl Case for FormatterRunner {
    fn name(&self) -> &'static str {
        "Formatter"
    }

    fn enable_runtime_test(&self) -> bool {
        false
    }

    fn driver(&self) -> Driver {
        unreachable!()
    }

    fn idempotency_test(&self, source: &Source) -> Result<String, Vec<Diagnostic>> {
        let Source { path, source_type, source_text } = source;

        let allocator = Allocator::new();
        let options = get_parse_options();

        let ParserReturn { program: program1, errors: errors1, .. } =
            Parser::new(&allocator, source_text, *source_type).with_options(options).parse();
        if !errors1.is_empty() {
            return Err(vec![Diagnostic {
                case: self.name(),
                path: path.clone(),
                message: format!("Parse error in original source: {errors1:?}"),
            }]);
        }

        let source_text2 = Formatter::new(&allocator, FormatOptions::default()).build(&program1);

        let ParserReturn { program: program2, errors: errors2, .. } =
            Parser::new(&allocator, &source_text2, *source_type).with_options(options).parse();
        if !errors2.is_empty() {
            return Err(vec![Diagnostic {
                case: self.name(),
                path: path.clone(),
                message: format!("Parse error after formatting: {errors2:?}"),
            }]);
        }

        let source_text3 = Formatter::new(&allocator, FormatOptions::default()).build(&program2);

        if source_text2 != source_text3 {
            return Err(vec![Diagnostic {
                case: self.name(),
                path: path.clone(),
                message: NodeModulesRunner::get_diff(&source_text2, &source_text3, false),
            }]);
        }

        Ok(source_text3)
    }
}
