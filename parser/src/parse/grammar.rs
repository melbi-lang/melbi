//! The PEG grammar, and the Pratt table that gives its operators precedence.
//!
//! The grammar is deliberately *flat*: `expression` matches a run of prefixes,
//! primaries and postfixes, and precedence is applied afterwards by the Pratt
//! parser below rather than encoded as a tower of grammar rules. That keeps the
//! `.pest` file readable and puts every precedence decision in one table.

use pest::pratt_parser::{Assoc, ConstPrattParser, Op, pratt_precedence};
use pest_derive::Parser;

/// The generated parser. [`Rule`] comes with it, one variant per grammar rule.
//
// TODO: this grammar is a copy of `core/src/parser/expression.pest`, which is
// itself kept in sync by hand with `tree-sitter/grammar.js`. Two copies were
// already one too many; they collapse back to one when `melbi-core` starts using
// this crate.
#[derive(Parser)]
#[grammar = "grammar/expression.pest"]
pub struct ExpressionParser;

/// The operator-precedence table, lowest precedence first.
///
/// A `static`, because [`ConstPrattParser`] is built in a `const` context: no
/// allocation, no lazy initialisation, and so nothing that needs `std` — which
/// matters, as this crate is `no_std`. The const parameter is the number of
/// operators in the table, counting each alternative of a `|` chain separately.
pub(super) static PRATT_PARSER: ConstPrattParser<Rule, 27> =
    ConstPrattParser::new_const(pratt_precedence![
        // --- lowest precedence ---
        // Lambda, and the two block postfixes.
        Op::prefix(Rule::lambda_op), // `(...) =>`
        Op::postfix(Rule::where_op) | Op::postfix(Rule::match_op), // `where {}`, `match {}`
        // Error handling.
        Op::infix(Rule::otherwise_op, Assoc::Right), // `otherwise`
        // Logical.
        Op::prefix(Rule::if_op),           // `if ... then ... else`
        Op::infix(Rule::or, Assoc::Left),  // `or`
        Op::infix(Rule::and, Assoc::Left), // `and`
        Op::prefix(Rule::not),             // `not`
        // Comparison and membership.
        Op::infix(Rule::eq, Assoc::Left)
            | Op::infix(Rule::neq, Assoc::Left)
            | Op::infix(Rule::lt, Assoc::Left)
            | Op::infix(Rule::gt, Assoc::Left)
            | Op::infix(Rule::le, Assoc::Left)
            | Op::infix(Rule::ge, Assoc::Left)
            | Op::infix(Rule::in_op, Assoc::Left)
            | Op::infix(Rule::not_in, Assoc::Left),
        // Arithmetic.
        Op::infix(Rule::add, Assoc::Left) | Op::infix(Rule::sub, Assoc::Left), // `+`, `-`
        Op::infix(Rule::mul, Assoc::Left) | Op::infix(Rule::div, Assoc::Left), // `*`, `/`
        Op::prefix(Rule::neg) | Op::prefix(Rule::some_op),                     // `-`, `some`
        Op::infix(Rule::pow, Assoc::Right),                                    // `^`
        // Postfix.
        Op::postfix(Rule::call_op),  // `()`
        Op::postfix(Rule::index_op), // `[]`
        Op::postfix(Rule::field_op), // `.`
        Op::postfix(Rule::cast_op),  // `as`
                                     // --- highest precedence ---
    ]);

/// The pattern grammar's precedence table.
///
/// Patterns have exactly one operator today, so this exists to run patterns
/// through the same Pratt machinery as expressions rather than because they need
/// precedence.
pub(super) static PATTERN_PRATT_PARSER: ConstPrattParser<Rule, 1> =
    ConstPrattParser::new_const(pratt_precedence![Op::prefix(Rule::pattern_some)]);
