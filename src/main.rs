mod ast;
mod parser;
mod eval;

use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: lumen <file.lm>");
        return;
    }
    let filename = &args[1];
    let src = fs::read_to_string(filename).expect("Unable to read file");
    let stmts = parser::parse_program(&src);
    eval::eval_program(&stmts);
}
