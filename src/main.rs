// Lumen-Lang Main Entry Point
// Routes between stream and microcode kernels based on --kernel parameter
// Usage: lumen-lang [--kernel stream|microcode] <file> [--lang <language>]

use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    // Parse --kernel parameter
    let (kernel_type, remaining_args) = parse_kernel_arg(&args);

    // Default to stream kernel
    let kernel = if kernel_type.is_empty() {
        "stream"
    } else {
        &kernel_type
    };

    // Route to appropriate kernel executable
    match kernel {
        "stream" => run_stream_kernel(&remaining_args),
        "microcode" => run_microcode_kernel(&remaining_args),
        _ => {
            eprintln!("Error: Unknown kernel '{}'. Use 'stream' (default) or 'microcode'.", kernel);
            eprintln!("Usage: {} [--kernel stream|microcode] <file> [--lang <language>]", args[0]);
            process::exit(1);
        }
    }
}

fn parse_kernel_arg(args: &[String]) -> (String, Vec<String>) {
    if args.len() < 2 {
        eprintln!("Usage: {} [--kernel stream|microcode] <file> [--lang <language>]", args.get(0).unwrap_or(&"lumen-lang".to_string()));
        process::exit(1);
    }

    // Check if first argument is --kernel
    if args.len() >= 3 && args[1] == "--kernel" {
        let kernel = args[2].to_lowercase();
        // Return the kernel and remaining args (excluding the binary name, --kernel, and kernel type)
        let remaining: Vec<String> = args.iter().skip(3).cloned().collect();
        (kernel, remaining)
    } else {
        // No --kernel specified, default to stream
        // Return empty kernel type and all args except the binary name
        let remaining: Vec<String> = args.iter().skip(1).cloned().collect();
        ("".to_string(), remaining)
    }
}

fn run_stream_kernel(args: &[String]) {
    // Execute the stream kernel binary with the remaining arguments
    // The stream kernel will handle language detection and file processing
    let mut binary_path = std::env::current_exe()
        .expect("Failed to get current executable path")
        .parent()
        .expect("Failed to get parent directory")
        .to_path_buf();

    let binary_name = if cfg!(windows) { "stream.exe" } else { "stream" };
    binary_path.push(binary_name);

    let mut cmd = std::process::Command::new(&binary_path);
    cmd.args(args);

    match cmd.status() {
        Ok(status) => {
            process::exit(status.code().unwrap_or(1));
        }
        Err(e) => {
            eprintln!("Error: Failed to execute stream kernel at {:?}: {}", binary_path, e);
            eprintln!("Make sure to build with 'cargo build' first");
            process::exit(1);
        }
    }
}

fn run_microcode_kernel(args: &[String]) {
    // Execute the microcode kernel binary with the remaining arguments
    // The microcode kernel will handle language detection and file processing
    let mut binary_path = std::env::current_exe()
        .expect("Failed to get current executable path")
        .parent()
        .expect("Failed to get parent directory")
        .to_path_buf();

    let binary_name = if cfg!(windows) { "microcode.exe" } else { "microcode" };
    binary_path.push(binary_name);

    let mut cmd = std::process::Command::new(&binary_path);
    cmd.args(args);

    match cmd.status() {
        Ok(status) => {
            process::exit(status.code().unwrap_or(1));
        }
        Err(e) => {
            eprintln!("Error: Failed to execute microcode kernel at {:?}: {}", binary_path, e);
            eprintln!("Make sure to build with 'cargo build' first");
            process::exit(1);
        }
    }
}
