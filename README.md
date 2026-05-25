# Corvo

**Write scripts like prose. Ship them like binaries. Trust them like Rust.**

Corvo is a modern scripting language that compiles to standalone Rust binaries. It is deliberately stripped of the things that make scripting languages fragile — no package manager, no dependency graph, no `import` statement, no complex function signatures to maintain. What remains is a language that is easy to read, audit, and ship anywhere.

## 🌟 Key Features

* **Compile to Standalone Binaries:** Corvo compiles your scripts into native, self-contained Rust executables. No need to install interpreters or runtimes on target machines.
* **Batteries Included:** Built-in support for HTTP (client and server), JSON, YAML, CSV, Cryptography, File System operations, OS integrations, interactive PTY automation (`pex.*`, Unix), and more.
* **Zero Dependencies:** No package managers (no `npm`, `pip`, or `cargo` required for scripts). Everything you need is built into the language.
* **Familiar & Clean Syntax:** Designed to be highly readable and easy to audit, using straightforward variables (`@name = "Corvo"`) and procedures.
* **Robust Coreutils:** The [futils](https://github.com/CorvoLang/futils) repository contains complete reimplementations of standard Unix utilities (like `ls`, `cat`, `cp`, `rm`, `chmod`) written entirely in Corvo, proving the language's capability.
* **Built-in Linter & Safety:** Includes an AST-based linter (`--lint`) that catches issues before you run or compile your code, similar to Cargo Clippy.

## 📦 Installation

To install the Corvo CLI via Cargo, run:

```bash
cargo install corvo-lang
```

### Prerequisites

Corvo requires the **Rust toolchain** to be installed on your system to compile scripts into standalone binaries.

*   **Linux / macOS**: Open your terminal and run:
    ```bash
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    ```
*   **Windows**: Download and run [rustup-init.exe](https://rustup.rs/) and follow the on-screen instructions. You may also need to install the [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/).

## 🚀 Quick Start

### Hello World

```corvo
# hello.corvo
@welcome = procedure(@who, @line) {
    @line = string.concat("Welcome to ", @who, "!")
}

sys.echo("Hello, World!")

@name = "Corvo"
@msg = ""
@welcome.call(@name, @msg)
sys.echo(@msg)
```

### HTTP Requests

```corvo
# http_example.corvo
try {
    @headers = {"Accept": "application/json"}
    @res = http.get("https://httpbin.org/get", @headers)
    sys.echo("Status: ${map.get(@res, \"status_code\")}")
} fallback {
    sys.echo("(skipped — no network access)")
}
```

## 💻 CLI Usage

The `corvo` CLI provides several ways to execute, check, and compile your code:

```bash
# Run a script directly
corvo script.corvo

# Start the interactive REPL
corvo --repl

# Evaluate a snippet
corvo --eval 'sys.echo("Hello from Corvo!")'

# Check syntax without executing
corvo --check script.corvo

# Check for syntax errors and unknown functions
corvo --lint script.corvo

# Compile to a native standalone executable
corvo --compile script.corvo -o myapp

# Transpile to a Rust project (full runtime)
corvo --transpile script.corvo

# Transpile to a lean, optimized Rust project (Oxide mode)
# Achieves < 5MB binaries for most scripts.
# Re-running --oxide into the same output dir appends [[bin]] targets without dropping prior ones.
corvo --oxide script.corvo

# Wipe the persistent compile cache (target dir reused by --compile)
corvo --clean

# Run in-script run_test blocks (Unix + pex; integration tests)
corvo --run-test script.corvo
```

### Interactive PTY automation (`pex.*`)

For programs that expect prompts, passwords, or ongoing shell interaction, use the `pex.*` namespace instead of `os.exec` (which runs a one-shot command and captures stdout/stderr). `pex.*` drives a pseudo-terminal session on **Unix** (Linux and macOS) via the optional `stdlib-pex` feature. See [`examples/pex_example.corvo`](examples/pex_example.corvo) and the [`pex.*` entries in CHEATSHEET.md](CHEATSHEET.md).

### In-script tests (`run_test`)

Use `run_test(name, argv, @pex) { ... }` blocks to define integration tests that spawn `corvo <file> <argv…>` via pex and assert on PTY output. Normal runs skip these blocks; `corvo --run-test script.corvo` executes only top-level `run_test` blocks. Requires Unix + `stdlib-pex`. See [`examples/run_test_example.corvo`](examples/run_test_example.corvo).

### Docker release image

Tagged releases publish a multi-arch image to GitHub Container Registry (`ghcr.io/<owner>/corvo-lang`). The image is built from [`Dockerfile.release`](Dockerfile.release): it ships the `corvo` CLI plus a Rust toolchain and a warmed `CARGO_HOME` cache so `--transpile` and `--oxide` output can be built inside the container without re-fetching dependencies.

```bash
docker run --rm -v "$PWD:/workspace" -w /workspace ghcr.io/corvolang/corvo-lang:latest \
  corvo --oxide myscript.corvo -o build/
```

### Compile cache

`corvo --compile` reuses cargo's incremental build cache across invocations,
so the first compile pays the cost of building the runtime and its
dependencies once and every subsequent compile only rebuilds the per-script
`main.rs`. The cache lives at:

* `~/.cache/corvo/build/` on Linux
* `~/Library/Caches/com.Corvo.corvo/build/` on macOS
* `%LOCALAPPDATA%\Corvo\corvo\cache\build\` on Windows

Set the `CORVO_CACHE_DIR` environment variable to override the location (for
example in CI or sandboxed environments), or run `corvo --clean` to wipe it.

## 🏗️ Architecture & Internals

Corvo is entirely written in Rust and is designed with a modern compiler architecture:
* **Interpreter:** Executes scripts dynamically via the CLI or REPL.
* **Transpiler:** Converts Corvo Abstract Syntax Tree (AST) into idiomatic Rust code.
* **Compiler:** Orchestrates the transpilation and uses `rustc` under the hood to generate the final standalone binary.

To explore the compiler, transpiler, and interpreter, check the [`src/`](src/) directory. For an extensive collection of Corvo scripts, see the [`examples/`](examples/) directory. For complete CLI utility reimplementations, see the [`futils`](https://github.com/CorvoLang/futils) repository.

## 🔍 AGENTS.md

Please refer to the [`AGENTS.md`](AGENTS.md) or [`copilot-instructions.md`](copilot-instructions.md) files for detailed code guidelines for development in this repository.