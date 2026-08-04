//! Walking the `pest` parse tree and building the [`parsed`] AST.
//!
//! Ported from `core/src/parser/parser.rs`. The shape is the same — a Pratt
//! parser over a flat grammar, one method per construct — but three things the
//! original had to do are simply gone:
//!
//! **No span side-table.** The original allocated a bare `Expr` and then
//! recorded its span in an `AnnotatedSource` keyed by the node's *address*, then
//! looked spans back up by pointer while combining them. Every node here is
//! built with its span in hand, so combining two is `left.data().span` and
//! `right.data().span`.
//!
//! **No `reslice`.** The original re-derived each `&str` against the arena's
//! copy of the source by pointer arithmetic, to move a borrow of the input into
//! the arena's lifetime. [`TreeBuilder::alloc_str`] takes any `&str` and returns
//! the builder's own, so slices are passed straight through.
//!
//! **No tuples standing in for nodes.** Bindings, map entries, match arms and
//! type fields were `(&str, &Expr)`-style pairs in a slice, so none of them had
//! a span. Each is now its own tree with its own span.
//!
//! [`parsed`]: crate::ast::parsed

use alloc::format;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::cell::Cell;

use pest::Parser as _;
use pest::iterators::{Pair, Pairs};
use pest::pratt_parser::PrattParser;

use super::error::{ParseError, ParseErrorKind};
use super::grammar::{ExpressionParser, Rule, pattern_pratt_parser, pratt_parser};
use crate::ast::parsed::{
    Binding, BindingKind, Data, Expr, ExprKind, LiteralKind, MapEntry, MapEntryKind, MatchArm,
    MatchArmKind, Pattern, PatternKind, TypeExpr, TypeExprKind, TypeField, TypeFieldKind,
};
use crate::ast::{BinaryOp, BoolOp, ComparisonOp, UnaryOp};
use crate::literal::bytes::unescape_bytes;
use crate::literal::string::unescape_string;
use crate::{Span, Tree, TreeBuilder, TreeDescriptor, TreeNode};

/// How deep expressions may nest before the parser gives up.
///
/// This is a guard against stack exhaustion on hostile input — the parse tree
/// walk is recursive — not a limit anyone should meet while writing Melbi.
pub const DEFAULT_MAX_PARSE_DEPTH: usize = 500;

/// Knobs for [`parse_with_options`].
///
/// A struct rather than more parameters on [`parse`], so that adding the next
/// knob does not change every call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseOptions {
    /// See [`DEFAULT_MAX_PARSE_DEPTH`].
    pub max_depth: usize,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            max_depth: DEFAULT_MAX_PARSE_DEPTH,
        }
    }
}

/// Parse `source` into an expression tree allocated in `builder`.
///
/// The whole program is one expression, so this returns one tree.
pub fn parse<B: TreeBuilder>(builder: &B, source: &str) -> Result<Tree<B, Expr>, ParseError> {
    parse_with_options(builder, source, ParseOptions::default())
}

/// [`parse`], with the nesting limit under the caller's control.
pub fn parse_with_options<B: TreeBuilder>(
    builder: &B,
    source: &str,
    options: ParseOptions,
) -> Result<Tree<B, Expr>, ParseError> {
    let mut pairs = ExpressionParser::parse(Rule::main, source).map_err(ParseError::from_pest)?;

    let whole_source = Span::new(0, source.len() as u32);
    let main = pairs
        .next()
        .ok_or_else(|| malformed(whole_source, "the `main` rule matched nothing"))?;

    ParseContext::new(builder, options).parse_expr(main)
}

/// The state threaded through the walk.
struct ParseContext<'builder, B: TreeBuilder> {
    builder: &'builder B,
    pratt: PrattParser<Rule>,
    pattern_pratt: PrattParser<Rule>,
    /// How many levels of [`parse_expr`](Self::parse_expr) are currently on the
    /// stack. A `Cell` because the walk takes `&self` throughout.
    depth: Cell<usize>,
    max_depth: usize,
}

impl<'builder, B: TreeBuilder> ParseContext<'builder, B> {
    fn new(builder: &'builder B, options: ParseOptions) -> Self {
        Self {
            builder,
            pratt: pratt_parser(),
            pattern_pratt: pattern_pratt_parser(),
            depth: Cell::new(0),
            max_depth: options.max_depth,
        }
    }

    // --- building blocks -----------------------------------------------------

    /// Allocate a node of any parsed tree.
    ///
    /// One method for all seven descriptors: they agree on
    /// [`Data`], so the only thing that varies is the kind.
    fn node<D>(&self, span: Span, kind: D::Kind<B>) -> Tree<B, D>
    where
        D: TreeDescriptor<Data = Data>,
    {
        TreeNode::new(Data::new(span), kind).alloc(self.builder)
    }

    /// Parse every pair of `pairs` with `parse_one` and allocate the results as
    /// one list.
    ///
    /// The results are collected before allocating because
    /// [`TreeBuilder::alloc_list`] needs a known length and cannot fail
    /// part-way, while parsing a child can.
    fn node_list<D, F>(
        &self,
        pairs: Pairs<'_, Rule>,
        parse_one: F,
    ) -> Result<B::List<D>, ParseError>
    where
        D: TreeDescriptor<Data = Data>,
        F: Fn(&Self, Pair<'_, Rule>) -> Result<Tree<B, D>, ParseError>,
    {
        let items = pairs
            .map(|pair| parse_one(self, pair))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(self.builder.alloc_list(items))
    }

    /// Take the next child, or report that the grammar and this parser disagree.
    fn next_child<'i>(
        &self,
        inner: &mut Pairs<'i, Rule>,
        span: Span,
        what: &str,
    ) -> Result<Pair<'i, Rule>, ParseError> {
        inner
            .next()
            .ok_or_else(|| malformed(span, &format!("missing {what}")))
    }

    /// Enter one level of nesting, refusing to go past [`ParseOptions::max_depth`].
    ///
    /// Balanced by [`leave`](Self::leave). Note that a failure here does *not*
    /// enter the level, so the caller must not leave it.
    fn enter(&self, span: Span) -> Result<(), ParseError> {
        let depth = self.depth.get();
        if depth >= self.max_depth {
            return Err(ParseError::new(
                ParseErrorKind::MaxDepthExceeded {
                    depth,
                    max_depth: self.max_depth,
                },
                span,
            ));
        }
        self.depth.set(depth + 1);
        Ok(())
    }

    fn leave(&self) {
        self.depth.set(self.depth.get() - 1);
    }

    /// Charge the depth budget for the prefix operators in one flat run.
    ///
    /// # Why this is needed
    ///
    /// [`enter`](Self::enter) counts levels of [`parse_expr`](Self::parse_expr),
    /// which covers everything that nests through a *rule* — parentheses, call
    /// arguments, array elements. Prefix operators do not: the grammar is flat,
    /// so `- - - 1` is a single `expression` pair holding three `neg` children,
    /// and `parse_expr` is entered once. The Pratt parser, however, recurses
    /// once per prefix operator before any closure of ours runs, so the native
    /// stack grows while the counter stands still. Around 8000 prefixes
    /// overflows the stack outright.
    ///
    /// The recursion reaches exactly the longest *consecutive* run of prefixes,
    /// so that is what is charged. Charging the total instead would reject
    /// `-a + -b + …` — wide, but not deep — once it passed the limit.
    fn charge_prefix_run(
        &self,
        pairs: &Pairs<'_, Rule>,
        is_prefix: fn(Rule) -> bool,
    ) -> Result<(), ParseError> {
        let mut run = 0usize;
        let mut longest = 0usize;
        let mut longest_at = None;

        // `Pairs` is a cheap cursor, so cloning to look ahead costs nothing.
        for pair in pairs.clone() {
            if is_prefix(pair.as_rule()) {
                run += 1;
                if run > longest {
                    longest = run;
                    longest_at = Some(span_of(&pair));
                }
            } else {
                run = 0;
            }
        }

        let depth = self.depth.get() + longest;
        if depth > self.max_depth {
            return Err(ParseError::new(
                ParseErrorKind::MaxDepthExceeded {
                    depth,
                    max_depth: self.max_depth,
                },
                longest_at.unwrap_or_default(),
            ));
        }

        Ok(())
    }

    // --- expressions ---------------------------------------------------------

    /// Dispatch on a pair that stands for a whole expression.
    fn parse_expr(&self, pair: Pair<'_, Rule>) -> Result<Tree<B, Expr>, ParseError> {
        let span = span_of(&pair);
        self.enter(span)?;

        let result = match pair.as_rule() {
            Rule::main => self.parse_main(pair),
            Rule::expression => self.parse_expression(pair),
            Rule::grouped => self.parse_grouped(pair),
            Rule::ident => Ok(self.node(span, ExprKind::Ident(self.alloc_str(&pair)))),
            Rule::none => Ok(self.node(span, ExprKind::None)),
            Rule::array => self.parse_array(pair),
            Rule::record => self.parse_record(pair),
            Rule::map => self.parse_map(pair),
            Rule::format_string => self.parse_format_string(pair),
            Rule::integer | Rule::float | Rule::boolean | Rule::string | Rule::bytes => {
                let literal = self.parse_literal(pair, LiteralPosition::Expression)?;
                Ok(self.node(span, ExprKind::Literal(literal)))
            }
            other => Err(malformed(span, &format!("unhandled rule {other:?}"))),
        };

        self.leave();
        result
    }

    /// `main = { SOI ~ expression ~ EOI }` — unwrap to the expression.
    fn parse_main(&self, pair: Pair<'_, Rule>) -> Result<Tree<B, Expr>, ParseError> {
        let span = span_of(&pair);
        let mut inner = pair.into_inner();
        let expression = self.next_child(&mut inner, span, "the top-level expression")?;
        self.parse_expr(expression)
    }

    /// Apply precedence to one flat run of prefixes, primaries and postfixes.
    fn parse_expression(&self, pair: Pair<'_, Rule>) -> Result<Tree<B, Expr>, ParseError> {
        let inner = pair.into_inner();
        self.charge_prefix_run(&inner, is_expression_prefix)?;

        self.pratt
            .map_primary(|primary| {
                // The extent is taken from the *pair*, so a `grouped` primary
                // reports its parentheses even though the tree it builds does
                // not. See [`Operand`].
                let extent = span_of(&primary);
                let tree = self.parse_expr(primary)?;
                Ok(Operand { tree, extent })
            })
            .map_prefix(|op, rhs| {
                let rhs = rhs?;
                // A prefix reaches from the operator to the end of its operand.
                let span = Span::new(span_of(&op).start(), rhs.extent.end());
                let tree = match op.as_rule() {
                    Rule::neg => self.node(span, unary(UnaryOp::Neg, rhs.tree)),
                    Rule::not => self.node(span, unary(UnaryOp::Not, rhs.tree)),
                    Rule::some_op => self.node(span, ExprKind::Some(rhs.tree)),
                    Rule::if_op => self.parse_if(op, rhs.tree, span)?,
                    Rule::lambda_op => self.parse_lambda(op, rhs.tree, span),
                    other => {
                        return Err(malformed(
                            span,
                            &format!("unknown prefix operator {other:?}"),
                        ));
                    }
                };
                Ok(Operand::spanning(tree, span))
            })
            .map_infix(|left, op, right| {
                let (left, right) = (left?, right?);
                let span = Span::new(left.extent.start(), right.extent.end());
                let (left, right) = (left.tree, right.tree);
                let kind = match op.as_rule() {
                    Rule::and => boolean(BoolOp::And, left, right),
                    Rule::or => boolean(BoolOp::Or, left, right),
                    Rule::add => binary(BinaryOp::Add, left, right),
                    Rule::sub => binary(BinaryOp::Sub, left, right),
                    Rule::mul => binary(BinaryOp::Mul, left, right),
                    Rule::div => binary(BinaryOp::Div, left, right),
                    Rule::pow => binary(BinaryOp::Pow, left, right),
                    Rule::eq => comparison(ComparisonOp::Eq, left, right),
                    Rule::neq => comparison(ComparisonOp::Neq, left, right),
                    Rule::lt => comparison(ComparisonOp::Lt, left, right),
                    Rule::gt => comparison(ComparisonOp::Gt, left, right),
                    Rule::le => comparison(ComparisonOp::Le, left, right),
                    Rule::ge => comparison(ComparisonOp::Ge, left, right),
                    Rule::in_op => comparison(ComparisonOp::In, left, right),
                    Rule::not_in => comparison(ComparisonOp::NotIn, left, right),
                    Rule::otherwise_op => ExprKind::Otherwise {
                        primary: left,
                        fallback: right,
                    },
                    other => {
                        return Err(malformed(
                            span,
                            &format!("unknown infix operator {other:?}"),
                        ));
                    }
                };
                Ok(Operand::spanning(self.node(span, kind), span))
            })
            .map_postfix(|left, op| {
                let left = left?;
                // A postfix reaches from the start of its operand to the end of
                // the operator — which for `where {…}` is the closing brace.
                let span = Span::new(left.extent.start(), span_of(&op).end());
                let left = left.tree;
                let tree = match op.as_rule() {
                    Rule::call_op => self.parse_call(left, op, span)?,
                    Rule::index_op => self.parse_index(left, op, span)?,
                    Rule::field_op => self.parse_field(left, op, span)?,
                    Rule::cast_op => self.parse_cast(left, op, span)?,
                    Rule::where_op => self.parse_where(left, op, span)?,
                    Rule::match_op => self.parse_match(left, op, span)?,
                    other => {
                        return Err(malformed(
                            span,
                            &format!("unknown postfix operator {other:?}"),
                        ));
                    }
                };
                Ok(Operand::spanning(tree, span))
            })
            .parse(inner)
            .map(|operand| operand.tree)
    }

    /// `if_op = { "if" ~ expression ~ "then" ~ expression ~ "else" }`, with the
    /// else-branch arriving as the prefix operator's operand.
    fn parse_if(
        &self,
        op: Pair<'_, Rule>,
        else_branch: Tree<B, Expr>,
        span: Span,
    ) -> Result<Tree<B, Expr>, ParseError> {
        let op_span = span_of(&op);
        let mut inner = op.into_inner();

        let cond = self.next_child(&mut inner, op_span, "the `if` condition")?;
        let cond = self.parse_expr(cond)?;
        let then_branch = self.next_child(&mut inner, op_span, "the `then` branch")?;
        let then_branch = self.parse_expr(then_branch)?;

        Ok(self.node(
            span,
            ExprKind::If {
                cond,
                then_branch,
                else_branch,
            },
        ))
    }

    /// `lambda_op = { "(" ~ lambda_params? ~ ")" ~ "=>" }`, with the body
    /// arriving as the prefix operator's operand.
    fn parse_lambda(&self, op: Pair<'_, Rule>, body: Tree<B, Expr>, span: Span) -> Tree<B, Expr> {
        let params = match op.into_inner().next() {
            Some(list) => {
                let names = list
                    .into_inner()
                    .map(|param| self.alloc_str(&param))
                    .collect::<Vec<_>>();
                self.builder.alloc_str_list(names)
            }
            None => self.builder.alloc_str_list([]),
        };

        self.node(span, ExprKind::Lambda { params, body })
    }

    fn parse_call(
        &self,
        callable: Tree<B, Expr>,
        op: Pair<'_, Rule>,
        span: Span,
    ) -> Result<Tree<B, Expr>, ParseError> {
        let args = self.node_list(op.into_inner(), Self::parse_expr)?;
        Ok(self.node(span, ExprKind::Call { callable, args }))
    }

    fn parse_index(
        &self,
        value: Tree<B, Expr>,
        op: Pair<'_, Rule>,
        span: Span,
    ) -> Result<Tree<B, Expr>, ParseError> {
        let op_span = span_of(&op);
        let mut inner = op.into_inner();
        let index = self.next_child(&mut inner, op_span, "the index expression")?;
        let index = self.parse_expr(index)?;
        Ok(self.node(span, ExprKind::Index { value, index }))
    }

    fn parse_field(
        &self,
        value: Tree<B, Expr>,
        op: Pair<'_, Rule>,
        span: Span,
    ) -> Result<Tree<B, Expr>, ParseError> {
        let op_span = span_of(&op);
        let mut inner = op.into_inner();
        let field = self.next_child(&mut inner, op_span, "the field name")?;
        let field = self.alloc_str(&field);
        Ok(self.node(span, ExprKind::Field { value, field }))
    }

    fn parse_cast(
        &self,
        expr: Tree<B, Expr>,
        op: Pair<'_, Rule>,
        span: Span,
    ) -> Result<Tree<B, Expr>, ParseError> {
        let op_span = span_of(&op);
        let mut inner = op.into_inner();
        let ty = self.next_child(&mut inner, op_span, "the target type")?;
        let ty = self.parse_type_expr(ty)?;
        Ok(self.node(span, ExprKind::Cast { expr, ty }))
    }

    fn parse_where(
        &self,
        expr: Tree<B, Expr>,
        op: Pair<'_, Rule>,
        span: Span,
    ) -> Result<Tree<B, Expr>, ParseError> {
        let bindings = self.node_list(op.into_inner(), Self::parse_binding)?;
        Ok(self.node(span, ExprKind::Where { expr, bindings }))
    }

    fn parse_match(
        &self,
        scrutinee: Tree<B, Expr>,
        op: Pair<'_, Rule>,
        span: Span,
    ) -> Result<Tree<B, Expr>, ParseError> {
        let arms = self.node_list(op.into_inner(), Self::parse_match_arm)?;
        Ok(self.node(span, ExprKind::Match { scrutinee, arms }))
    }

    /// `grouped = { "(" ~ expression ~ ")" }`.
    ///
    /// The parentheses leave no node behind, so the result is the inner
    /// expression, carrying the inner expression's span: the brackets fall
    /// outside it. That is deliberate — the tree already represents the
    /// precedence the brackets were written for, and the `grouped` production
    /// disappears with them, so there is nothing for a bracket to belong to.
    ///
    /// Parents still see the brackets, because they combine [`Operand::extent`]
    /// rather than node spans. Without that, this choice would leave a parent
    /// spanning `2 + 3) * 4`.
    fn parse_grouped(&self, pair: Pair<'_, Rule>) -> Result<Tree<B, Expr>, ParseError> {
        let span = span_of(&pair);
        let mut inner = pair.into_inner();
        let expression = self.next_child(&mut inner, span, "the parenthesised expression")?;
        self.parse_expr(expression)
    }

    fn parse_array(&self, pair: Pair<'_, Rule>) -> Result<Tree<B, Expr>, ParseError> {
        let span = span_of(&pair);
        let items = self.node_list(pair.into_inner(), Self::parse_expr)?;
        Ok(self.node(span, ExprKind::Array(items)))
    }

    fn parse_record(&self, pair: Pair<'_, Rule>) -> Result<Tree<B, Expr>, ParseError> {
        let span = span_of(&pair);
        let fields = self.node_list(pair.into_inner(), Self::parse_binding)?;
        Ok(self.node(span, ExprKind::Record(fields)))
    }

    fn parse_map(&self, pair: Pair<'_, Rule>) -> Result<Tree<B, Expr>, ParseError> {
        let span = span_of(&pair);
        let entries = self.node_list(pair.into_inner(), Self::parse_map_entry)?;
        Ok(self.node(span, ExprKind::Map(entries)))
    }

    /// `f"a { x } b"` — alternating literal text and interpolated expressions.
    ///
    /// The grammar emits only the segments that are actually present, while
    /// [`ExprKind::FormatStr`] requires `strs.len() == exprs.len() + 1`, so an
    /// empty string is inserted wherever two holes abut or one sits at an end.
    fn parse_format_string(&self, pair: Pair<'_, Rule>) -> Result<Tree<B, Expr>, ParseError> {
        let span = span_of(&pair);
        let mut strs = Vec::new();
        let mut exprs = Vec::new();
        let mut last_was_text = false;

        for segment in pair.into_inner() {
            // Raw, because a literal piece of a format string may end in a
            // space that is part of the string: `f"Hello { name }"`.
            let segment_span = raw_span_of(&segment);
            match segment.as_rule() {
                Rule::format_text | Rule::format_text_single => {
                    let text = unescape_string(segment.as_str(), true).map_err(|error| {
                        invalid_literal(segment_span, &format!("invalid format string: {error}"))
                    })?;
                    strs.push(self.builder.alloc_str(&text));
                    last_was_text = true;
                }
                Rule::format_expr => {
                    if !last_was_text {
                        strs.push(self.builder.alloc_str(""));
                    }
                    let mut inner = segment.into_inner();
                    let expr =
                        self.next_child(&mut inner, segment_span, "the interpolated expression")?;
                    exprs.push(self.parse_expr(expr)?);
                    last_was_text = false;
                }
                other => {
                    return Err(malformed(
                        segment_span,
                        &format!("unknown format-string segment {other:?}"),
                    ));
                }
            }
        }

        if !last_was_text {
            strs.push(self.builder.alloc_str(""));
        }

        Ok(self.node(
            span,
            ExprKind::FormatStr {
                strs: self.builder.alloc_str_list(strs),
                exprs: self.builder.alloc_list(exprs),
            },
        ))
    }

    // --- the small trees -----------------------------------------------------

    /// `binding = { ident ~ "=" ~ expression }`, in a `where` block or a record.
    fn parse_binding(&self, pair: Pair<'_, Rule>) -> Result<Tree<B, Binding>, ParseError> {
        let span = span_of(&pair);
        let mut inner = pair.into_inner();

        let name = self.next_child(&mut inner, span, "the binding name")?;
        let name = self.alloc_str(&name);
        let value = self.next_child(&mut inner, span, "the binding value")?;
        let value = self.parse_expr(value)?;

        Ok(self.node(span, BindingKind { name, value }))
    }

    /// `map_entry = { expression ~ ":" ~ expression }`.
    fn parse_map_entry(&self, pair: Pair<'_, Rule>) -> Result<Tree<B, MapEntry>, ParseError> {
        let span = span_of(&pair);
        let mut inner = pair.into_inner();

        let key = self.next_child(&mut inner, span, "the map key")?;
        let key = self.parse_expr(key)?;
        let value = self.next_child(&mut inner, span, "the map value")?;
        let value = self.parse_expr(value)?;

        Ok(self.node(span, MapEntryKind { key, value }))
    }

    /// `match_arm = { pattern ~ "->" ~ expression }`.
    fn parse_match_arm(&self, pair: Pair<'_, Rule>) -> Result<Tree<B, MatchArm>, ParseError> {
        let span = span_of(&pair);
        let mut inner = pair.into_inner();

        let pattern = self.next_child(&mut inner, span, "the arm's pattern")?;
        let pattern = self.parse_pattern(pattern)?;
        let body = self.next_child(&mut inner, span, "the arm's body")?;
        let body = self.parse_expr(body)?;

        Ok(self.node(span, MatchArmKind { pattern, body }))
    }

    // --- patterns ------------------------------------------------------------

    fn parse_pattern(&self, pair: Pair<'_, Rule>) -> Result<Tree<B, Pattern>, ParseError> {
        let span = span_of(&pair);
        self.enter(span)?;

        let inner = pair.into_inner();
        // `some (some (some …))` recurses in the Pratt parser exactly as the
        // expression prefixes do.
        if let Err(error) = self.charge_prefix_run(&inner, is_pattern_prefix) {
            self.leave();
            return Err(error);
        }

        let result = self
            .pattern_pratt
            .map_primary(|primary| {
                // As for expressions: a `pattern_grouped` primary reports its
                // brackets even though the tree it builds does not.
                let extent = span_of(&primary);
                let tree = self.parse_pattern_primary(primary)?;
                Ok(Operand { tree, extent })
            })
            .map_prefix(|op, rhs| {
                let rhs = rhs?;
                let span = Span::new(span_of(&op).start(), rhs.extent.end());
                match op.as_rule() {
                    Rule::pattern_some => Ok(Operand::spanning(
                        self.node(span, PatternKind::Some(rhs.tree)),
                        span,
                    )),
                    other => Err(malformed(
                        span,
                        &format!("unknown prefix pattern operator {other:?}"),
                    )),
                }
            })
            .parse(inner)
            .map(|operand| operand.tree);

        self.leave();
        result
    }

    fn parse_pattern_primary(&self, pair: Pair<'_, Rule>) -> Result<Tree<B, Pattern>, ParseError> {
        let span = span_of(&pair);
        match pair.as_rule() {
            // `pattern_grouped = { "(" ~ pattern ~ ")" }`. As with `grouped` on
            // the expression side, the brackets leave no node behind: the
            // result is the inner pattern, carrying the inner pattern's span.
            Rule::pattern_grouped => {
                let mut inner = pair.into_inner();
                let pattern = self.next_child(&mut inner, span, "the parenthesised pattern")?;
                self.parse_pattern(pattern)
            }
            Rule::pattern_wildcard => Ok(self.node(span, PatternKind::Wildcard)),
            Rule::pattern_var => Ok(self.node(span, PatternKind::Binding(self.alloc_str(&pair)))),
            Rule::pattern_none => Ok(self.node(span, PatternKind::None)),
            Rule::integer | Rule::float | Rule::boolean | Rule::string | Rule::bytes => {
                let literal = self.parse_literal(pair, LiteralPosition::Pattern)?;
                Ok(self.node(span, PatternKind::Literal(literal)))
            }
            other => Err(malformed(span, &format!("unknown pattern {other:?}"))),
        }
    }

    // --- type syntax ---------------------------------------------------------

    /// `type_expr = { record_type | type_path ~ type_params? }`.
    fn parse_type_expr(&self, pair: Pair<'_, Rule>) -> Result<Tree<B, TypeExpr>, ParseError> {
        let span = span_of(&pair);

        if pair.as_rule() != Rule::type_expr {
            return Err(malformed(
                span,
                &format!("expected a type expression, got {:?}", pair.as_rule()),
            ));
        }

        let mut inner = pair.into_inner();
        let first = self.next_child(&mut inner, span, "the type")?;

        match first.as_rule() {
            Rule::record_type => {
                let fields = self.node_list(first.into_inner(), Self::parse_type_field)?;
                Ok(self.node(span, TypeExprKind::Record(fields)))
            }
            Rule::type_path => {
                let path = self.alloc_str(&first);
                // `type_params` is a silent rule, so the parameters are direct
                // children of `type_expr` rather than nested one level down.
                let params = self.node_list(inner, Self::parse_type_expr)?;
                let kind = if params.is_empty() {
                    TypeExprKind::Path(path)
                } else {
                    TypeExprKind::Parametrized { path, params }
                };
                Ok(self.node(span, kind))
            }
            other => Err(malformed(
                span_of(&first),
                &format!("unexpected {other:?} in a type expression"),
            )),
        }
    }

    /// `type_field = { ident ~ ":" ~ type_expr }`, one field of a `Record[…]`.
    fn parse_type_field(&self, pair: Pair<'_, Rule>) -> Result<Tree<B, TypeField>, ParseError> {
        let span = span_of(&pair);
        let mut inner = pair.into_inner();

        let name = self.next_child(&mut inner, span, "the field name")?;
        let name = self.alloc_str(&name);
        let ty = self.next_child(&mut inner, span, "the field type")?;
        let ty = self.parse_type_expr(ty)?;

        Ok(self.node(span, TypeFieldKind { name, ty }))
    }

    // --- literals ------------------------------------------------------------

    /// Build a literal, shared by the expression and pattern positions.
    ///
    /// The original had two near-identical copies of each of these — one for
    /// expressions and one for patterns — which is also why the pattern copies
    /// reported subtly different messages. The only real difference is the unit
    /// suffix, which `position` decides.
    fn parse_literal(
        &self,
        pair: Pair<'_, Rule>,
        position: LiteralPosition,
    ) -> Result<LiteralKind<B>, ParseError> {
        let span = span_of(&pair);
        match pair.as_rule() {
            Rule::boolean => match pair.as_str() {
                "true" => Ok(LiteralKind::Bool(true)),
                "false" => Ok(LiteralKind::Bool(false)),
                other => Err(invalid_literal(
                    span,
                    &format!("`{other}` is not a boolean literal"),
                )),
            },
            Rule::string => {
                // The grammar guarantees a quote at each end.
                let text = pair.as_str();
                let contents = &text[1..text.len() - 1];
                let unescaped = unescape_string(contents, false).map_err(|error| {
                    invalid_literal(span, &format!("invalid string literal: {error}"))
                })?;
                Ok(LiteralKind::Str(self.builder.alloc_str(&unescaped)))
            }
            Rule::bytes => {
                // `b"…"` or `b'…'`: two characters of prefix, one of suffix.
                let text = pair.as_str();
                let contents = &text[2..text.len() - 1];
                let unescaped = unescape_bytes(contents).map_err(|error| {
                    invalid_literal(span, &format!("invalid bytes literal: {error}"))
                })?;
                Ok(LiteralKind::Bytes(self.builder.alloc_bytes(&unescaped)))
            }
            Rule::integer => self.parse_integer(pair, position),
            Rule::float => self.parse_float(pair, position),
            other => Err(malformed(span, &format!("{other:?} is not a literal"))),
        }
    }

    /// `integer = ${ integer_number ~ suffix? }`, in any base.
    fn parse_integer(
        &self,
        pair: Pair<'_, Rule>,
        position: LiteralPosition,
    ) -> Result<LiteralKind<B>, ParseError> {
        let span = span_of(&pair);
        let mut inner = pair.into_inner();

        let number = self.next_child(&mut inner, span, "the integer")?;
        // Underscores are decoration; the sign, if any, stays attached.
        let text = number.as_str().replace('_', "");

        let mut digits_of = number.into_inner();
        let base_marker = self.next_child(&mut digits_of, span, "the integer's digits")?;
        let (radix, digits) = match base_marker.as_rule() {
            Rule::dec_integer => (10, text),
            Rule::bin_integer => (2, text.replacen("0b", "", 1)),
            Rule::oct_integer => (8, text.replacen("0o", "", 1)),
            Rule::hex_integer => (16, text.replacen("0x", "", 1)),
            other => {
                return Err(malformed(span, &format!("unknown integer form {other:?}")));
            }
        };

        let value = i64::from_str_radix(&digits, radix).map_err(|_| {
            invalid_literal(span, "integer literal does not fit in a 64-bit integer")
        })?;

        let suffix = self.parse_suffix(inner.next(), span, position)?;
        Ok(LiteralKind::Int { value, suffix })
    }

    /// `float = ${ float_number ~ suffix? }`.
    fn parse_float(
        &self,
        pair: Pair<'_, Rule>,
        position: LiteralPosition,
    ) -> Result<LiteralKind<B>, ParseError> {
        let span = span_of(&pair);
        let mut inner = pair.into_inner();

        let number = self.next_child(&mut inner, span, "the number")?;
        let value: f64 = number
            .as_str()
            .replace('_', "")
            .parse()
            .map_err(|_| invalid_literal(span, "invalid floating-point literal"))?;

        let suffix = self.parse_suffix(inner.next(), span, position)?;
        Ok(LiteralKind::Float { value, suffix })
    }

    /// ``suffix = ${ "`" ~ expression ~ "`" }`` — a unit of measurement.
    ///
    /// A whole expression, because a unit may be a product, quotient or power
    /// as in ``9.81`m/s^2` ``. Narrowing that to the sub-language of units is a
    /// later pass's job, not the grammar's.
    fn parse_suffix(
        &self,
        suffix: Option<Pair<'_, Rule>>,
        literal_span: Span,
        position: LiteralPosition,
    ) -> Result<Option<Tree<B, Expr>>, ParseError> {
        let Some(suffix) = suffix else {
            return Ok(None);
        };

        if position == LiteralPosition::Pattern {
            return Err(invalid_literal(
                literal_span,
                "a unit suffix is not allowed in a pattern literal",
            ));
        }

        let span = span_of(&suffix);
        let mut inner = suffix.into_inner();
        let expr = self.next_child(&mut inner, span, "the unit expression")?;
        Ok(Some(self.parse_expr(expr)?))
    }

    /// Intern a pair's text into the builder.
    fn alloc_str(&self, pair: &Pair<'_, Rule>) -> B::Str {
        self.builder.alloc_str(pair.as_str())
    }
}

/// What the Pratt parser passes between its closures: a built subtree, plus the
/// source it actually occupies.
///
/// # Why the extent is not just the tree's span
///
/// Parentheses group but do not appear in the tree — the nesting already says
/// what they said — so the node built for `(2 + 3)` spans `2 + 3` and the
/// brackets fall outside it. A parent combining its children's *node* spans
/// would then start inside a bracket: in `1 + (2 + 3) * 4`, the multiplication
/// would span `2 + 3) * 4`, which is not a piece of syntax anyone can underline.
/// That is the bug `test_grouped_expression_span_bug` in
/// `core/src/parser/parser.rs` records, and it is a bug whichever way the
/// grouped node's own span goes.
///
/// Keeping the two apart settles it: a node's span stays free of brackets it
/// does not represent, while parents combine extents and so stay balanced.
struct Operand<B: TreeBuilder, D: TreeDescriptor<Data = Data>> {
    tree: Tree<B, D>,
    extent: Span,
}

impl<B: TreeBuilder, D: TreeDescriptor<Data = Data>> Operand<B, D> {
    /// An operand whose extent is exactly its own span — every node built by an
    /// operator, since an operator's span already covers all it consumed.
    fn spanning(tree: Tree<B, D>, extent: Span) -> Self {
        Self { tree, extent }
    }
}

/// Where a literal appears, which is the only thing that differs between the two
/// positions: a pattern may not carry a unit suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LiteralPosition {
    Expression,
    Pattern,
}

/// A pest span as one of ours, with trailing whitespace removed.
///
/// `pest` reports `usize` byte offsets; [`Span`] holds `u32`, capping a source
/// at 4 GiB. Saturating rather than wrapping keeps a truncated span pointing
/// past the end rather than back at the beginning.
///
/// # Why it trims
///
/// A non-atomic rule skips whitespace *between* its terms, and the last such
/// skip lands inside the rule's span even though nothing matched after it. So
/// the `binding` in `{ x = 1, y = 2 }` reports `y = 2 ` — trailing space
/// included — because `expression` tried for an infix operator, skipped the
/// space, and only then gave up. Underlining that space in "unused binding `y`"
/// would be wrong, so the span stops at the last non-whitespace byte.
///
/// This is safe for every rule whose text can end in whitespace *meaningfully* —
/// a string literal ends with its quote, not its content — with the single
/// exception of `format_text`, which is why [`raw_span_of`] exists.
//
// TODO: a trailing comment gets swept into the span the same way, and is not
// trimmed here: `{ x = 1 // note\n }` reports the comment as part of the
// binding. Stripping it needs more care than `trim_end`, since `//` inside a
// string literal must not be mistaken for one.
fn span_of(pair: &Pair<'_, Rule>) -> Span {
    let span = raw_span_of(pair);
    let text = pair.as_str();
    let trimmed = text.trim_end().len();
    Span::new(
        span.start(),
        span.start() + u32::try_from(trimmed).unwrap_or(u32::MAX),
    )
}

/// A pest span as one of ours, exactly as reported.
///
/// Only for atomic rules whose matched text may legitimately end in whitespace —
/// `format_text`, where `f"Hello { name }"` has a literal piece of `"Hello "`.
fn raw_span_of(pair: &Pair<'_, Rule>) -> Span {
    let span = pair.as_span();
    Span::new(
        span.start().try_into().unwrap_or(u32::MAX),
        span.end().try_into().unwrap_or(u32::MAX),
    )
}

/// The rules of `prefix_op`, which are the ones the Pratt parser recurses on.
fn is_expression_prefix(rule: Rule) -> bool {
    matches!(
        rule,
        Rule::neg | Rule::not | Rule::if_op | Rule::lambda_op | Rule::some_op
    )
}

/// The rules of `pattern_prefix`.
fn is_pattern_prefix(rule: Rule) -> bool {
    matches!(rule, Rule::pattern_some)
}

fn malformed(span: Span, message: &str) -> ParseError {
    ParseError::new(
        ParseErrorKind::Malformed {
            message: message.to_string(),
        },
        span,
    )
}

fn invalid_literal(span: Span, message: &str) -> ParseError {
    ParseError::new(
        ParseErrorKind::InvalidLiteral {
            message: message.to_string(),
        },
        span,
    )
}

// Small constructors, so the big `match` in `map_infix` stays one line per
// operator.

fn unary<B: TreeBuilder>(op: UnaryOp, expr: Tree<B, Expr>) -> ExprKind<B> {
    ExprKind::Unary { op, expr }
}

fn binary<B: TreeBuilder>(op: BinaryOp, left: Tree<B, Expr>, right: Tree<B, Expr>) -> ExprKind<B> {
    ExprKind::Binary { op, left, right }
}

fn boolean<B: TreeBuilder>(op: BoolOp, left: Tree<B, Expr>, right: Tree<B, Expr>) -> ExprKind<B> {
    ExprKind::Boolean { op, left, right }
}

fn comparison<B: TreeBuilder>(
    op: ComparisonOp,
    left: Tree<B, Expr>,
    right: Tree<B, Expr>,
) -> ExprKind<B> {
    ExprKind::Comparison { op, left, right }
}
