// Lumen-Lang entry point.
// Routes to the stream or microcode kernel binary based on --kernel.
// Usage: lumen-lang [--kernel stream|microcode] <file> [--lang <language>] [program args...]
// Default kernel: microcode.

use std::env;
use std::process;

const KERNELS: [&str; 2] = ["stream", "microcode"];
const DEFAULT_KERNEL: &str = "microcode";

fn main() {
    let args: Vec<String> = env::args().collect();
    let (kernel, remaining) = parse_kernel_arg(&args);
    run_kernel(&kernel, &remaining);
}

fn usage(program: &str) -> ! {
    eprintln!("Usage: {} [--kernel stream|microcode] <file> [--lang <language>] [program args...]", program);
    process::exit(1);
}

fn parse_kernel_arg(args: &[String]) -> (String, Vec<String>) {
    let program = args.first().map(String::as_str).unwrap_or("lumen-lang");
    if args.len() < 2 {
        usage(program);
    }
    if args[1] == "--kernel" {
        if args.len() < 4 {
            usage(program);
        }
        let kernel = args[2].to_lowercase();
        if !KERNELS.contains(&kernel.as_str()) {
            eprintln!("Error: Unknown kernel '{}'. Use one of: {}", kernel, KERNELS.join(", "));
            process::exit(1);
        }
        return (kernel, args[3..].to_vec());
    }
    (DEFAULT_KERNEL.to_string(), args[1..].to_vec())
}

/// Execute the kernel binary that sits beside this executable.
fn run_kernel(kernel: &str, args: &[String]) {
    let mut binary_path = env::current_exe()
        .expect("Failed to get current executable path")
        .parent()
        .expect("Failed to get parent directory")
        .to_path_buf();
    binary_path.push(if cfg!(windows) { format!("{kernel}.exe") } else { kernel.to_string() });

    match process::Command::new(&binary_path).args(args).status() {
        Ok(status) => process::exit(status.code().unwrap_or(1)),
        Err(e) => {
            eprintln!("Error: Failed to execute {} kernel at {:?}: {}", kernel, binary_path, e);
            eprintln!("Make sure to build with 'cargo build' first");
            process::exit(1);
        }
    }
}
