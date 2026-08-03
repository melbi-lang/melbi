//! The PEG grammar, and the Pratt table that gives its operators precedence.
//!
//! The grammar is deliberately *flat*: `expression` matches a run of prefixes,
//! primaries and postfixes, and precedence is applied afterwards by the Pratt
//! parser below rather than encoded as a tower of grammar rules. That keeps the
//! `.pest` file readable and puts every precedence decision in one table.

use pest::pratt_parser::{Assoc, Op, PrattParser};
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

/// Build the operator-precedence table, lowest precedence first.
///
/// This is constructed per parse rather than stored in a `static`. The original
/// used `lazy_static!`, which needs `std` (or a spin lock) — and this crate is
/// `no_std`. Building it costs one small allocation against a whole parse.
//
// TODO: revisit if profiling says otherwise; a `OnceLock` equivalent would need
// a dependency purely for this.
pub(super) fn pratt_parser() -> PrattParser<Rule> {
    PrattParser::new()
        // --- lowest precedence ---
        // Lambda, and the two block postfixes.
        .op(Op::prefix(Rule::lambda_op)) // `(...) =>`
        .op(Op::postfix(Rule::where_op) | Op::postfix(Rule::match_op)) // `where {}`, `match {}`
        // Error handling.
        .op(Op::infix(Rule::otherwise_op, Assoc::Right)) // `otherwise`
        // Logical.
        .op(Op::prefix(Rule::if_op)) // `if ... then ... else`
        .op(Op::infix(Rule::or, Assoc::Left)) // `or`
        .op(Op::infix(Rule::and, Assoc::Left)) // `and`
        .op(Op::prefix(Rule::not)) // `not`
        // Comparison and membership.
        .op(Op::infix(Rule::eq, Assoc::Left)
            | Op::infix(Rule::neq, Assoc::Left)
            | Op::infix(Rule::lt, Assoc::Left)
            | Op::infix(Rule::gt, Assoc::Left)
            | Op::infix(Rule::le, Assoc::Left)
            | Op::infix(Rule::ge, Assoc::Left)
            | Op::infix(Rule::in_op, Assoc::Left)
            | Op::infix(Rule::not_in, Assoc::Left))
        // Arithmetic.
        .op(Op::infix(Rule::add, Assoc::Left) | Op::infix(Rule::sub, Assoc::Left)) // `+`, `-`
        .op(Op::infix(Rule::mul, Assoc::Left) | Op::infix(Rule::div, Assoc::Left)) // `*`, `/`
        .op(Op::prefix(Rule::neg) | Op::prefix(Rule::some_op)) // `-`, `some`
        .op(Op::infix(Rule::pow, Assoc::Right)) // `^`
        // Postfix.
        .op(Op::postfix(Rule::call_op)) // `()`
        .op(Op::postfix(Rule::index_op)) // `[]`
        .op(Op::postfix(Rule::field_op)) // `.`
        .op(Op::postfix(Rule::cast_op)) // `as`
    // --- highest precedence ---
}

/// The pattern grammar's precedence table.
///
/// Patterns have exactly one operator today, so this exists to run patterns
/// through the same Pratt machinery as expressions rather than because they need
/// precedence.
pub(super) fn pattern_pratt_parser() -> PrattParser<Rule> {
    PrattParser::new().op(Op::prefix(Rule::pattern_some))
}
