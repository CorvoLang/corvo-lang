use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(
    name = "corvo",
    about = "Corvo Programming Language",
    settings = &[
        structopt::clap::AppSettings::TrailingVarArg,
        structopt::clap::AppSettings::AllowLeadingHyphen,
        structopt::clap::AppSettings::DisableHelpFlags,
    ],
    after_help = "Corvo CLI help (no script):\n  corvo -h | corvo --help\n\nExamples:\n  corvo script.corvo              Run a file\n  corvo script.corvo -lh /        Run a script with args (e.g. coreutils/ls.corvo; same flag ordering as GNU ls)\n  corvo --repl                    Start interactive REPL\n  corvo --eval 'sys.echo(\"hi\")'  Evaluate an expression\n  corvo --compile script.corvo    Compile to standalone executable\n  corvo --check script.corvo      Check syntax\n  corvo --lint script.corvo       Analyse code for errors and unknown functions"
)]
struct Args {
    #[structopt(help = "Corvo file to execute or compile")]
    file: Option<PathBuf>,

    #[structopt(short, long, help = "Start the REPL")]
    repl: bool,

    #[structopt(short, long, help = "Print version")]
    version: bool,

    #[structopt(short, long, help = "Evaluate a string")]
    eval: Option<String>,

    #[structopt(long, help = "Check syntax without executing")]
    check: bool,

    #[structopt(
        long,
        help = "Analyse code for errors and unknown functions (like cargo clippy)"
    )]
    lint: bool,

    #[structopt(long, help = "Compile to standalone executable")]
    compile: bool,

    #[structopt(long, help = "Transpile to a Rust project directory")]
    transpile: bool,

    #[structopt(
        short,
        long,
        help = "Output path for compiled executable",
        parse(from_os_str)
    )]
    output: Option<PathBuf>,

    #[structopt(long, help = "Use debug build mode (faster compile)")]
    debug: bool,

    #[structopt(
        long,
        help = "Generate a binary that aborts when run under a debugger, tracer, or dynamic analysis tool (gdb, LLDB, strace, rr, WinDbg, Valgrind)"
    )]
    no_debug: bool,

    /// Arguments forwarded to the script (`os.argv()`); may start with `-` (GNU ls-style flags after FILE)
    #[structopt(name = "SCRIPT_ARGS", allow_hyphen_values = true)]
    script_args: Vec<String>,
}

/// When the user runs exactly `corvo -h` or `corvo --help`, show Corvo's CLI help.
/// Any other invocation (including `corvo script.corvo -h`) is parsed normally so
/// script flags like GNU `ls -h` are forwarded via `SCRIPT_ARGS`.
fn maybe_print_corvo_help_only() -> bool {
    let argv: Vec<String> = std::env::args().collect();
    if argv.len() == 2 && (argv[1] == "-h" || argv[1] == "--help") {
        Args::clap().print_help().unwrap();
        println!();
        return true;
    }
    false
}

fn main() {
    if maybe_print_corvo_help_only() {
        return;
    }

    let args = Args::from_args();

    if args.version {
        println!("Corvo Language v{}", env!("CARGO_PKG_VERSION"));
        return;
    }

    if args.repl {
        corvo_lang::run_repl();
        return;
    }

    if let Some(source) = args.eval {
        // In eval mode there is no file path, so we cannot show a source-context
        // snippet. Use the no-source variant which still prints the error category,
        // message, and any available help text.
        match corvo_lang::run_source(&source) {
            Ok(_) => {}
            Err(e) => {
                if let Some(code) = e.process_exit_code() {
                    std::process::exit(code);
                }
                corvo_lang::diagnostic::print_error_no_source(&e);
                std::process::exit(e.exit_code());
            }
        }
        return;
    }

    if let Some(file) = args.file {
        if args.compile {
            compile_file(&file, args.output.as_deref(), args.debug, args.no_debug);
        } else if args.transpile {
            transpile_file(&file, args.output.as_deref());
        } else if args.lint {
            lint_file(&file);
        } else if args.check {
            check_syntax(&file);
        } else {
            run_file(&file, args.script_args);
        }
        return;
    }

    print_usage();
    std::process::exit(1);
}

fn run_file(file: &std::path::Path, script_args: Vec<String>) {
    let source = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: Cannot read '{}': {}", file.display(), e);
            std::process::exit(1);
        }
    };

    let mut state = corvo_lang::RuntimeState::new();
    state.set_script_argv(script_args);
    match corvo_lang::run_source_with_state(&source, &mut state) {
        Ok(_) => {}
        Err(e) => {
            if let Some(code) = e.process_exit_code() {
                std::process::exit(code);
            }
            let filename = file.display().to_string();
            corvo_lang::diagnostic::print_error(&e, &source, &filename);
            std::process::exit(e.exit_code());
        }
    }
}

fn compile_file(
    file: &std::path::Path,
    output: Option<&std::path::Path>,
    debug: bool,
    no_debug: bool,
) {
    let source = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: Cannot read '{}': {}", file.display(), e);
            std::process::exit(1);
        }
    };

    // Determine output path
    let output_path = match output {
        Some(p) => p.to_path_buf(),
        None => {
            let stem = file.file_stem().unwrap_or_default().to_string_lossy();
            if cfg!(target_os = "windows") {
                PathBuf::from(format!("{}.exe", stem))
            } else {
                PathBuf::from(stem.to_string())
            }
        }
    };

    let mut compiler = corvo_lang::compiler::Compiler::new(source, file.to_path_buf());
    if debug {
        compiler = compiler.with_debug();
    }
    if no_debug {
        compiler = compiler.with_no_debug();
    }

    eprintln!(
        "Pre-executing {} to capture static values...",
        file.display()
    );
    match compiler.pre_execute() {
        Ok(_) => {
            let statics = compiler.static_count();
            if statics > 0 {
                eprintln!("Captured {} static value(s)", statics);
            }
        }
        Err(e) => {
            eprintln!("warning: Pre-execution error: {}", e);
        }
    }

    eprintln!("Compiling {}...", file.display());

    match compiler.compile(&output_path) {
        Ok(binary) => {
            eprintln!("Compiled successfully: {}", binary.display());
        }
        Err(e) => {
            eprintln!("error: Compilation failed: {}", e);
            std::process::exit(1);
        }
    }
}

fn transpile_file(file: &std::path::Path, output: Option<&std::path::Path>) {
    let source = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: Cannot read '{}': {}", file.display(), e);
            std::process::exit(1);
        }
    };

    // Determine output directory
    let output_dir = match output {
        Some(p) => p.to_path_buf(),
        None => {
            let stem = file.file_stem().unwrap_or_default().to_string_lossy();
            PathBuf::from(stem.to_string())
        }
    };

    let mut compiler = corvo_lang::compiler::Compiler::new(source, file.to_path_buf());

    eprintln!(
        "Pre-executing {} to capture static values...",
        file.display()
    );
    match compiler.pre_execute() {
        Ok(_) => {}
        Err(e) => {
            eprintln!("warning: Pre-execution error: {}", e);
        }
    }

    eprintln!(
        "Transpiling {} to Rust project in {}...",
        file.display(),
        output_dir.display()
    );

    match compiler.transpile(&output_dir) {
        Ok(_) => {
            eprintln!("Transpiled successfully to {}", output_dir.display());
            eprintln!("To run: cd {} && cargo run", output_dir.display());
        }
        Err(e) => {
            eprintln!("error: Transpilation failed: {}", e);
            std::process::exit(1);
        }
    }
}

fn check_syntax(file: &std::path::Path) {
    let source = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: Cannot read '{}': {}", file.display(), e);
            std::process::exit(1);
        }
    };

    let filename = file.display().to_string();

    match corvo_lang::lexer::Lexer::new(&source).tokenize() {
        Ok(tokens) => match corvo_lang::parser::Parser::new(tokens).parse() {
            Ok(_) => println!("Syntax OK: {}", file.display()),
            Err(e) => {
                corvo_lang::diagnostic::print_error(&e, &source, &filename);
                std::process::exit(e.exit_code());
            }
        },
        Err(e) => {
            corvo_lang::diagnostic::print_error(&e, &source, &filename);
            std::process::exit(e.exit_code());
        }
    }
}

/// Run the full lint pass on a Corvo file.
///
/// This performs lexing + parsing (reporting any errors with rich diagnostics)
/// and then walks the AST to detect issues such as unknown function calls,
/// offering "did you mean?" suggestions – similar to `cargo clippy`.
fn lint_file(file: &std::path::Path) {
    use corvo_lang::diagnostic::{lint_program, print_error, LintSeverity};

    let source = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: Cannot read '{}': {}", file.display(), e);
            std::process::exit(1);
        }
    };

    let filename = file.display().to_string();

    // ── Lex ──────────────────────────────────────────────────────────────────
    let tokens = match corvo_lang::lexer::Lexer::new(&source).tokenize() {
        Ok(t) => t,
        Err(e) => {
            print_error(&e, &source, &filename);
            std::process::exit(e.exit_code());
        }
    };

    // ── Parse ─────────────────────────────────────────────────────────────────
    let program = match corvo_lang::parser::Parser::new(tokens).parse() {
        Ok(p) => p,
        Err(e) => {
            print_error(&e, &source, &filename);
            std::process::exit(e.exit_code());
        }
    };

    // ── Static analysis ───────────────────────────────────────────────────────
    let diagnostics = lint_program(&program);

    if diagnostics.is_empty() {
        println!("corvo: `{}` - no issues found!", file.display());
        return;
    }

    let mut has_error = false;
    for diag in &diagnostics {
        if diag.severity == LintSeverity::Error {
            has_error = true;
        }
        eprintln!("{}: {}", diag.severity, diag.message);
        if let Some(ref help) = diag.help {
            eprintln!("  help: {}", help);
        }
    }

    let errors = diagnostics
        .iter()
        .filter(|d| d.severity == LintSeverity::Error)
        .count();
    let warnings = diagnostics
        .iter()
        .filter(|d| d.severity == LintSeverity::Warning)
        .count();

    eprintln!();
    if errors > 0 && warnings > 0 {
        eprintln!(
            "corvo: {} error(s) and {} warning(s) found in `{}`",
            errors,
            warnings,
            file.display()
        );
    } else if errors > 0 {
        eprintln!("corvo: {} error(s) found in `{}`", errors, file.display());
    } else {
        eprintln!(
            "corvo: {} warning(s) found in `{}`",
            warnings,
            file.display()
        );
    }

    if has_error {
        std::process::exit(1);
    }
}

fn print_usage() {
    eprintln!("Usage: corvo [OPTIONS] [FILE] [SCRIPT_ARGS]...");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  -r, --repl           Start the REPL");
    eprintln!("  -e, --eval <EXPR>    Evaluate an expression");
    eprintln!("  -c, --compile        Compile to standalone executable");
    eprintln!("      --transpile      Transpile to a Rust project directory");
    eprintln!("  -o, --output <PATH>  Output path (for --compile or --transpile)");
    eprintln!("  -v, --version        Print version");
    eprintln!("      --check          Check syntax without executing");
    eprintln!("      --lint           Analyse code for errors (like cargo clippy)");
    eprintln!("      --debug          Use debug build mode (faster compile)");
    eprintln!("      --no-debug       Generate a binary that aborts under debuggers/tracers");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  corvo script.corvo               Run a file");
    eprintln!("  corvo script.corvo -lh /         Run with script args (GNU ls-style flags after the file)");
    eprintln!(
        "  corvo -h                         Corvo CLI help (must be the only argument after corvo)"
    );
    eprintln!("  corvo --repl                     Start interactive REPL");
    eprintln!("  corvo --compile script.corvo     Compile to executable");
    eprintln!("  corvo --compile script.corvo -o myapp");
    eprintln!("  corvo --compile --no-debug script.corvo  Compile with anti-debug protection");
    eprintln!("  corvo --eval 'sys.echo(\"hi\")'   Evaluate an expression");
    eprintln!("  corvo --lint script.corvo        Analyse code for issues");
}
