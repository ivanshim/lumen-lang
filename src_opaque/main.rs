// Opaque Kernel: Language-Agnostic Structure Processing
// Demonstrates pure kernel with language-specific semantic interpretation
// Built on opaque analysis architecture: kernel does orchestration, languages do interpretation
// Kernel uses Box<dyn Any> for complete semantic blindness

mod kernel;
mod languages;

use kernel::{Kernel, Token};

fn main() {
    println!("=== Opaque Kernel: Language-Agnostic Structure Processing ===\n");

    // Example 1: Lumen (indentation-based)
    println!("--- Lumen Example (Indentation-based) ---\n");

    let lumen_kernel = Kernel::new(languages::lumen::lumen_structure());

    let lumen_tokens = vec![
        Token::new("keyword_if", "if"),
        Token::new("identifier", "x"),
        Token::new("newline", "\n"),
        Token::new("indent", "    "),  // 4 spaces = depth 1
        Token::new("identifier", "print"),
        Token::new("newline", "\n"),
        // Implicit dedent back to 0
    ];

    println!("Input tokens:");
    for (i, token) in lumen_tokens.iter().enumerate() {
        println!("  {}: {} = {:?}", i, token.name, token.lexeme);
    }
    println!();

    match lumen_kernel.process_structure(lumen_tokens) {
        Ok(output) => {
            println!("Output tokens (after structure processing):");
            for (i, token) in output.iter().enumerate() {
                println!("  {}: {} = {:?}", i, token.name, token.lexeme);
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }
    println!();

    // Example 2: Python (indentation + colon)
    println!("--- Python Example (Indentation + Colon) ---\n");

    let python_kernel = Kernel::new(languages::python_core::python_structure());

    let python_tokens = vec![
        Token::new("keyword_if", "if"),
        Token::new("identifier", "x"),
        Token::new("colon", ":"),
        Token::new("newline", "\n"),
        Token::new("indent", "    "),  // 4 spaces = depth 1
        Token::new("identifier", "print"),
        Token::new("newline", "\n"),
    ];

    println!("Input tokens:");
    for (i, token) in python_tokens.iter().enumerate() {
        println!("  {}: {} = {:?}", i, token.name, token.lexeme);
    }
    println!();

    match python_kernel.process_structure(python_tokens) {
        Ok(output) => {
            println!("Output tokens (after structure processing):");
            for (i, token) in output.iter().enumerate() {
                println!("  {}: {} = {:?}", i, token.name, token.lexeme);
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }
    println!();

    // Example 3: Rust (explicit braces - pass through)
    println!("--- Rust Example (Explicit Braces) ---\n");

    let rust_kernel = Kernel::new(languages::rust_core::rust_structure());

    let rust_tokens = vec![
        Token::new("keyword_if", "if"),
        Token::new("identifier", "x"),
        Token::new("brace_open", "{"),
        Token::new("identifier", "print"),
        Token::new("brace_close", "}"),
    ];

    println!("Input tokens:");
    for (i, token) in rust_tokens.iter().enumerate() {
        println!("  {}: {} = {:?}", i, token.name, token.lexeme);
    }
    println!();

    match rust_kernel.process_structure(rust_tokens) {
        Ok(output) => {
            println!("Output tokens (after structure processing - unchanged):");
            for (i, token) in output.iter().enumerate() {
                println!("  {}: {} = {:?}", i, token.name, token.lexeme);
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }
    println!();

    // Example 4: Multi-level indentation
    println!("--- Multi-Level Indentation Example ---\n");

    let lumen_kernel = Kernel::new(languages::lumen::lumen_structure());

    let nested_tokens = vec![
        Token::new("keyword_if", "if"),
        Token::new("identifier", "x"),
        Token::new("newline", "\n"),
        Token::new("indent", "    "),  // depth 1
        Token::new("keyword_if", "if"),
        Token::new("identifier", "y"),
        Token::new("newline", "\n"),
        Token::new("indent", "        "),  // depth 2 (8 spaces)
        Token::new("identifier", "print"),
        Token::new("newline", "\n"),
        Token::new("indent", "    "),  // back to depth 1
        Token::new("identifier", "else_part"),
        Token::new("newline", "\n"),
        Token::new("indent", ""),  // back to depth 0
        Token::new("keyword_end", "end"),
    ];

    println!("Input tokens (nested structure):");
    for (i, token) in nested_tokens.iter().enumerate() {
        let indent_level = if token.lexeme.is_empty() { 0 } else { token.lexeme.len() / 4 };
        println!("  {}: {} = {:?} (indent level: {})", i, token.name, token.lexeme, indent_level);
    }
    println!();

    match lumen_kernel.process_structure(nested_tokens) {
        Ok(output) => {
            println!("Output tokens:");
            for (i, token) in output.iter().enumerate() {
                if token.name.starts_with("marker") {
                    println!("  {}: {} = {:?} [INSERTED]", i, token.name, token.lexeme);
                } else {
                    println!("  {}: {} = {:?}", i, token.name, token.lexeme);
                }
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }

    println!("\n=== All examples completed successfully ===");
}
