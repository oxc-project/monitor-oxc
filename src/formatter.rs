use rustc_hash::{FxHashMap, FxHashSet};

use oxc::{
    allocator::Allocator,
    ast::AstKind,
    ast::ast::Program,
    ast_visit::Visit,
    parser::{ParseOptions, Parser, ParserReturn},
};
use oxc_formatter::{FormatOptions, Formatter};

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
        let options = ParseOptions {
            preserve_parens: false,
            allow_return_outside_function: true,
            allow_v8_intrinsics: true,
            parse_regular_expression: false,
        };

        let ParserReturn { program: program1, errors: errors1, .. } =
            Parser::new(&allocator, source_text, *source_type).with_options(options).parse();
        if !errors1.is_empty() {
            return Err(vec![Diagnostic {
                case: self.name(),
                path: path.clone(),
                message: format!("Parse error in original source: {errors1:?}\n"),
            }]);
        }

        // Collect stats before formatting
        let stats_before = StatsCollector::new().collect(&program1);

        let source_text2 = Formatter::new(&allocator, FormatOptions::default()).build(&program1);

        let ParserReturn { program: program2, errors: errors2, .. } =
            Parser::new(&allocator, &source_text2, *source_type).with_options(options).parse();
        if !errors2.is_empty() {
            return Err(vec![Diagnostic {
                case: self.name(),
                path: path.clone(),
                message: format!("Parse error after formatting: {errors2:?}\n"),
            }]);
        }

        // Collect stats after formatting
        let stats_after = StatsCollector::new().collect(&program2);

        // Check for comment/node preservation
        if let Some(diff) = StatsChecker::diff(&stats_before, &stats_after) {
            return Err(vec![Diagnostic {
                case: self.name(),
                path: path.clone(),
                message: format!("Comment/Node preservation check failed:\n{diff}"),
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

// ---

type Counter = FxHashMap<String, usize>;

#[derive(Debug)]
struct StatsCollector {
    comment_count: usize,
    node_counts: Counter,
    object_key_counts: Counter,
}

impl StatsCollector {
    fn new() -> Self {
        Self {
            comment_count: 0,
            node_counts: FxHashMap::default(),
            object_key_counts: FxHashMap::default(),
        }
    }

    #[must_use]
    fn collect(mut self, program: &Program<'_>) -> Self {
        self.comment_count = program.comments.len();
        self.visit_program(program);
        self
    }
}

impl<'a> Visit<'a> for StatsCollector {
    fn enter_node(&mut self, kind: AstKind<'a>) {
        // `debug_name` contains the node type and its details.
        let node_name = kind.debug_name().to_string();

        #[allow(clippy::match_same_arms)]
        match kind {
            // `;` can be safely removed.
            // e.g. `;[]` -> `[]`
            AstKind::EmptyStatement(_) => {}
            // `JSXText` with only whitespace can be safely removed.
            // e.g.
            // ```
            // return (
            //   <div>
            //     {children}
            //   </div>
            // )
            // ```
            // -> `return (<div>{children}</div>)`
            AstKind::JSXText(t) if t.value.trim().is_empty() => {}

            // Object keys can be formatted differently based on quotes.
            // e.g. `{ "key": value }` -> `{ key: value }`
            // Therefore, we should count their value instead of their node type.
            AstKind::StringLiteral(_) | AstKind::IdentifierName(_) | AstKind::NumericLiteral(_) => {
                for prefix in ["StringLiteral(", "IdentifierName(", "NumericLiteral("] {
                    if let Some(rest) = node_name.strip_prefix(prefix)
                        && let Some(value) = rest.strip_suffix(')')
                    {
                        *self.object_key_counts.entry(value.to_string()).or_insert(0) += 1;
                        break;
                    }
                }
            }

            // NOTE: Useless `(` and `)` can be safely removed.
            // e.g. `(a)` -> `a`
            // e.g. `(a, b, (c, d))` -> `(a, b, c, d)`
            // e.g.  `(a?.b)?.c` -> `a?.b?.c`
            //
            // However, in some cases, they are necessary to preserve the semantics.
            // e.g. `return (a + b) * c;` -> `return a + b * c;`
            //
            // Therefore, we currently do not ignore them.
            //
            // In the first place, `ParenthesizedExpression` nodes are not generated,
            // since `preserve_parens: false` is set in `ParseOptions`.
            // AstKind::ParenthesizedExpression(_) => {}
            // AstKind::SequenceExpression(_) => {}
            // AstKind::ChainExpression(_) => {}

            // Other nodes should be preserved.
            _ => {
                *self.node_counts.entry(node_name).or_insert(0) += 1;
            }
        }
    }
}

struct StatsChecker;

impl StatsChecker {
    fn diff(before: &StatsCollector, after: &StatsCollector) -> Option<String> {
        let mut errors = Vec::new();

        if let Some(comment_diff) = Self::diff_comments(before.comment_count, after.comment_count) {
            errors.push(comment_diff);
        }

        if let Some(count_diffs) =
            Self::diff_counts(&before.object_key_counts, &after.object_key_counts, "Object key")
        {
            errors.extend(count_diffs);
        }
        if let Some(count_diffs) =
            Self::diff_counts(&before.node_counts, &after.node_counts, "Node")
        {
            errors.extend(count_diffs);
        }

        (!errors.is_empty()).then_some(errors.join("\n"))
    }

    fn diff_comments(before: usize, after: usize) -> Option<String> {
        (before != after).then_some(format!("Comment count mismatch: {before} -> {after}"))
    }

    fn diff_counts(before: &Counter, after: &Counter, label: &str) -> Option<Vec<String>> {
        let mut errors = Vec::new();

        let mut all_key_names: FxHashSet<_> = before.keys().collect();
        all_key_names.extend(after.keys());

        for key in all_key_names {
            let before_count = before.get(key).copied().unwrap_or(0);
            let after_count = after.get(key).copied().unwrap_or(0);

            if before_count != after_count {
                errors.push(format!(
                    "{label} count mismatch for '{key}': {before_count} -> {after_count}",
                ));
            }
        }

        (!errors.is_empty()).then_some(errors)
    }
}
