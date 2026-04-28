# Corvo

**Write scripts like prose. Ship them like binaries. Trust them like Rust.**

Corvo is a modern scripting language that compiles to standalone Rust binaries. It is deliberately stripped of the things that make scripting languages fragile — no package manager, no dependency graph, no `import` statement, no complex function signatures to maintain. What remains is a language that is easy to read, audit, and ship anywhere.

## 🌟 Key Features

* **Compile to Standalone Binaries:** Corvo compiles your scripts into native, self-contained Rust executables. No need to install interpreters or runtimes on target machines.
* **Batteries Included:** Built-in support for HTTP (client and server), JSON, YAML, CSV, Cryptography, File System operations, OS integrations, and more.
* **Zero Dependencies:** No package managers (no `npm`, `pip`, or `cargo` required for scripts). Everything you need is built into the language.
* **Familiar & Clean Syntax:** Designed to be highly readable and easy to audit, using straightforward variables (`@name = "Corvo"`) and procedures.
* **Robust Coreutils:** Corvo ships with its own implementations of standard Unix utilities (like `ls`, `cat`, `cp`, `rm`, `chmod`) written entirely in Corvo, proving the language's capability.
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

# Transpile to a Rust project
corvo --transpile script.corvo
```

## 🏗️ Architecture & Internals

Corvo is entirely written in Rust and is designed with a modern compiler architecture:
* **Interpreter:** Executes scripts dynamically via the CLI or REPL.
* **Transpiler:** Converts Corvo Abstract Syntax Tree (AST) into idiomatic Rust code.
* **Compiler:** Orchestrates the transpilation and uses `rustc` under the hood to generate the final standalone binary.

To explore the compiler, transpiler, and interpreter, check the [`src/`](src/) directory. For an extensive collection of Corvo scripts, see the [`examples/`](examples/) and [`coreutils/`](coreutils/) directories.
