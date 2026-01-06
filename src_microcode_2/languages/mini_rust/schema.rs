use crate::schema::LanguageSchema;

pub fn get_schema() -> LanguageSchema {
    let mut schema = LanguageSchema::new();
    schema.multichar_lexemes = vec![
        "==", "!=", "<=", ">=", "**", "->",
        "let", "mut", "if", "else", "while", "for", "break", "continue", "return", "fn", "and", "or", "not",
        "print", "true", "false", "none",
    ];
    schema.word_boundary_keywords = vec![
        "let", "mut", "if", "else", "while", "for", "break", "continue", "return", "fn",
        "and", "or", "not", "print", "true", "false", "none",
    ];
    schema.terminators = vec!["\n", ";"];
    schema
}
