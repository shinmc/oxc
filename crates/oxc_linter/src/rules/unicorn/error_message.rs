use oxc_ast::{
    AstKind,
    ast::{Argument, CallExpression, Expression, NewExpression},
};
use oxc_diagnostics::OxcDiagnostic;
use oxc_macros::declare_oxc_lint;
use oxc_span::Span;

use crate::{AstNode, context::LintContext, rule::Rule, utils::BUILT_IN_ERRORS};

fn missing_message(ctor_name: &str, span: Span) -> OxcDiagnostic {
    OxcDiagnostic::warn(format!("Pass a message to the {ctor_name:1} constructor."))
        .with_label(span)
}

fn empty_message(span: Span) -> OxcDiagnostic {
    OxcDiagnostic::warn("Error message should not be an empty string.").with_label(span)
}

fn not_string(span: Span) -> OxcDiagnostic {
    OxcDiagnostic::warn("Error message should be a string.").with_label(span)
}

#[derive(Default, Debug, Clone)]
pub struct ErrorMessage;

declare_oxc_lint!(
    /// ### What it does
    ///
    /// Enforces providing a `message` when creating built-in `Error` objects to
    /// improve readability and debugging.
    ///
    /// ### Why is this bad?
    ///
    /// Throwing an `Error` without a message, like `throw new Error()`, provides no context
    /// on what went wrong, making debugging harder. A clear error message improves
    /// code clarity and helps developers quickly identify issues.
    ///
    /// ### Examples
    ///
    /// Examples of **incorrect** code for this rule:
    /// ```javascript
    /// throw Error()
    /// throw new TypeError()
    /// ```
    ///
    /// Examples of **correct** code for this rule:
    /// ```javascript
    /// throw new Error('Unexpected token')
    /// throw new TypeError('Number expected')
    /// ```
    ErrorMessage,
    unicorn,
    style
);

impl Rule for ErrorMessage {
    fn run<'a>(&self, node: &AstNode<'a>, ctx: &LintContext<'a>) {
        let (callee, span, args) = match node.kind() {
            AstKind::NewExpression(NewExpression {
                callee: Expression::Identifier(id),
                span,
                arguments,
                ..
            })
            | AstKind::CallExpression(CallExpression {
                callee: Expression::Identifier(id),
                span,
                arguments,
                ..
            }) => (id, *span, arguments),
            _ => return,
        };

        if !BUILT_IN_ERRORS.contains(&callee.name.as_str()) {
            return;
        }

        // If there is `SpreadElement` before message
        if matches!(args.first(), Some(Argument::SpreadElement(_))) {
            return;
        }

        let constructor_name = &callee.name;
        let message_argument_idx = usize::from(callee.name == "AggregateError");
        let message_argument = args.get(message_argument_idx);

        let Some(arg) = message_argument else {
            return ctx.diagnostic(missing_message(constructor_name, span));
        };

        let diagnostic = match arg {
            Argument::ArrayExpression(array_expr) => not_string(array_expr.span),
            Argument::ObjectExpression(object_expr) => not_string(object_expr.span),
            Argument::StringLiteral(lit) if lit.value.is_empty() => empty_message(lit.span),
            Argument::TemplateLiteral(template_lit)
                if template_lit.span.source_text(ctx.source_text()).len() == 2 =>
            {
                empty_message(template_lit.span)
            }
            _ => return,
        };

        ctx.diagnostic(diagnostic);
    }
}

#[test]
fn test() {
    use crate::tester::Tester;

    let pass = vec![
        "throw new Error('error')",
        "throw new TypeError('error')",
        "throw new MyCustomError('error')",
        "throw new MyCustomError()",
        "throw generateError()",
        "throw foo()",
        "throw err",
        "throw 1",
        "new Error(\"message\", 0, 0)",
        "new Error(foo)",
        "new Error(...foo)",
        "new AggregateError(errors, \"message\")",
        "new NotAggregateError(errors)",
        "new AggregateError(...foo)",
        "new AggregateError(...foo, \"\")",
        "new AggregateError(errors, ...foo)",
        "new AggregateError(errors, message, \"\")",
        "new AggregateError(\"\", message, \"\")",
    ];

    let fail = vec![
        "throw new Error()",
        "throw Error()",
        "throw new Error('')",
        "throw new Error(``)",
        "const foo = new TypeError()",
        "const foo = new SyntaxError()",
        "throw new Error([])",
        "throw new Error([foo])",
        "throw new Error({})",
        "throw new Error({foo})",
        "const error = new RangeError;",
        "new AggregateError(errors)",
        "AggregateError(errors)",
        "new AggregateError(errors, \"\")",
        "new AggregateError(errors, ``)",
        "new AggregateError(errors, \"\", extraArgument)",
        "new AggregateError(errors, [])",
        "new AggregateError(errors, [foo])",
        "new AggregateError(errors, {})",
        "new AggregateError(errors, {foo})",
        "const error = new AggregateError;",
    ];

    Tester::new(ErrorMessage::NAME, ErrorMessage::PLUGIN, pass, fail).test_and_snapshot();
}
