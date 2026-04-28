When generating or modifying code for the Corvo Language repository, please follow these strict guidelines to ensure code quality and architectural consistency:

1. **Context First**: Always base your Corvo syntax and standard library usage on the project's `README.md` and `CHEATSHEET.md`. Refer to the `examples/` and `coreutils/` directories for practical implementation patterns. Do not hallucinate Python, Bash, or JavaScript patterns; use Corvo's explicit syntax (e.g., variables start with `@`, procedures use `@name = procedure(...) {}`).

2. **Idiomatic Corvo**:
   - Prefer built-in shorthands (`@var++`, `@var += 5`, `@var or= (...)`).
   - Use `try` / `fallback` blocks for robust error handling.
   - When iterating, use `browse` or `async_browse` instead of manual index management.
   - Understand that Corvo has zero dependencies and no `import` mechanism; rely entirely on the built-in standard library.

3. **Implementing New Features (Internal Rust Architecture)**:
   - **New Stdlib Functions**: Implement in `src/standard_lib/<namespace>.rs`, using `Value` and `CorvoResult`. Register the function in `src/standard_lib/mod.rs`, add it to `KNOWN_FUNCTIONS` in `src/diagnostic.rs`, and update `CHEATSHEET.md`.
   - **New Syntax/Blocks**: Requires updates across the full pipeline: Lexer (`src/lexer/token.rs`), AST (`src/ast/`), Parser (`src/parser/recursive_descent.rs`), Linter (`src/diagnostic.rs`), Interpreter (`src/compiler/evaluator.rs`), and Transpiler (`src/compiler/transpiler.rs`).

4. **Testing Pipeline**: Every feature implementation, bug fix, or script creation must be fully verified. You must ensure:
   - **Unit Tests**: Implement and pass logic tests for all Rust source files.
   - **Compiling & Transpiling Tests**: Verify that any Corvo script works dynamically via the interpreter (`corvo <file.corvo>`), compiles to a standalone binary (`corvo --compile <file.corvo>`), and transpiles cleanly (`corvo --transpile <file.corvo>`).

5. **Pre-commit Standards**: Before finalizing your code or committing any file, you MUST run and pass the following quality checks:
   - `cargo fmt` (for Rust code)
   - `cargo clippy` (for Rust code)
   - `corvo --lint <file.corvo>` (for Corvo scripts)
