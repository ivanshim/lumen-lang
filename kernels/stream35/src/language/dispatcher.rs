// Lumen language dispatcher: registers the lexemes and the handlers.
//
// Every lexeme the lexer segments on comes from langs/lumen.json: the
// definition's symbols are recognised wherever they appear, and its reserved
// words only as whole words. Handlers are registered in priority order.

use crate::kernel::registry::TokenDefinition;
use super::definition::def;
use super::registry::Registry;

use super::expressions;
use super::postfix;
use super::statements;
use super::structure;

/// Register the language's lexemes and every handler
pub fn register_all(registry: &mut Registry) {
    let mut tokens: Vec<TokenDefinition> = Vec::new();
    for symbol in def().symbols() {
        tokens.push(TokenDefinition::recognize(symbol));
    }
    for word in def().reserved_words() {
        tokens.push(TokenDefinition::keyword(word.clone()));
    }
    for name in def().compound_builtins() {
        tokens.push(TokenDefinition::keyword(name));
    }
    registry.tokens.set_token_definitions(tokens);
    registry.tokens.set_word_chars(crate::language::word_char);

    // Core syntax (structural tokens - parentheses, indentation, etc.)
    structure::structural::register(registry);

    // A postfix language has no expressions or statement forms: every
    // word acts on the stack, and only the literals are shared.
    if def().postfix {
        expressions::literals::register(registry);
        postfix::register(registry);
        return;
    }

    // Expression features. Registration order matters: earlier registrations
    // have higher priority, so literals, operators and extern come before
    // the generic identifier handler.
    expressions::literals::register(registry);
    expressions::logic::register(registry);
    expressions::arithmetic::register(registry);
    expressions::comparison::register(registry);
    expressions::pipe::register(registry);
    expressions::extern_expr::register(registry);
    expressions::grouping::register(registry);
    expressions::array_literal::register(registry);
    expressions::array_index::register(registry);
    expressions::variable::register(registry);

    // Statement features. Keyword handlers come before assignment, which
    // matches any identifier; the expression statement is the fallback.
    statements::function_emit::register(registry);
    statements::push_stmt::register(registry);
    statements::let_mut_binding::register(registry);
    statements::let_binding::register(registry);
    statements::array_assign::register(registry);
    statements::control_if_else::register(registry);
    statements::control_while::register(registry);
    statements::control_for::register(registry);
    statements::control_until::register(registry);
    statements::system_memoization::register(registry);
    statements::assignment::register(registry);
    statements::flow_break::register(registry);
    statements::flow_continue::register(registry);
    statements::return_stmt::register(registry);
    statements::pass_stmt::register(registry);
    statements::block_stmt::register(registry);
    statements::functions::register(registry);
    statements::expr_stmt::register(registry);
}
