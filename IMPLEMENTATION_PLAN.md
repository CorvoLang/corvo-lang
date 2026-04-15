# Corvo Language - Implementation Plan

## Project Overview
Corvo is a programming language designed for simplicity and readability, serving as a modern alternative to Shell scripting/Coreutils. Key characteristics:
- Functionless (built-in functions only)
- ifless (no if/else statements)
- Try/Fallback error handling
- Strongly and limited typed (string, number, boolean, list, map)
- Compiled language written in Rust
- REPL support
- Cross-platform compatibility

## Project Structure
```
corvo-lang/
├── src/
│   ├── main.rs              # Entry point
│   ├── lib.rs               # Library root
│   ├── lexer/                # Tokenizer
│   │   ├── mod.rs
│   │   └── token.rs
│   ├── parser/              # AST Parser
│   │   ├── mod.rs
│   │   └── ast/
│   │       ├── mod.rs
│   │       ├── expr.rs
│   │       └── stmt.rs
│   ├── ast/                  # Abstract Syntax Tree types
│   │   ├── mod.rs
│   │   └── node.rs
│   ├── compiler/             # Compiler/Interpreter
│   │   ├── mod.rs
│   │   ├── evaluator.rs
│   │   └── bytecode.rs
│   ├── standard_lib/          # Built-in functions
│   │   ├── mod.rs
│   │   ├── sys.rs
│   │   ├── os.rs
│   │   ├── math.rs
│   │   ├── crypto.rs
│   │   ├── fs.rs
│   │   ├── http.rs
│   │   ├── dns.rs
│   │   ├── ssh.rs
│   │   ├── rsync.rs
│   │   ├── json.rs
│   │   ├── yaml.rs
│   │   ├── hcl.rs
│   │   ├── csv.rs
│   │   ├── xml.rs
│   │   └── llm.rs
│   ├── repl/                 # REPL implementation
│   │   └── mod.rs
│   ├── type_system/          # Type definitions and methods
│   │   ├── mod.rs
│   │   ├── value.rs
│   │   ├── types.rs
│   │   └── type_methods.rs
│   ├── runtime/              # Runtime state management
│   │   ├── mod.rs
│   │   ├── state.rs
│   │   └── scope.rs
│   └── error.rs              # Error handling
├── tests/                    # Integration tests
├── examples/                 # Example Corvo programs
└── Cargo.toml
```

---

## Phase 1: Project Setup & Core Infrastructure

### 1.1 Initialize Rust Project
- [x] Create `Cargo.toml` with dependencies:
  - `miette` for error reporting
  - `rustyline` for REPL
  - `serde`/`serde_json` for serialization
  - `reqwest` for HTTP
  - `native-tls` for HTTPS
  - `ssh2` for SSH
  - `rsync` wrapper or command execution
  - `quick-xml` for XML parsing
  - `csv` for CSV parsing
  - `uuid` for UUID generation
  - `aes-gcm` for encryption
  - `sha2`, `md5` for hashing
  - `tokio` for async operations
- [x] Setup project structure (directories)
- [x] Create `.gitignore`
- [x] Basic `lib.rs` and `main.rs` scaffold

### 1.2 Error Handling Module (`src/error.rs`)
- [x] Define `CorvoError` enum with variants:
  - `LexingError`
  - `ParsingError`
  - `TypeError`
  - `RuntimeError`
  - `AssertionError`
  - `FileSystemError`
  - `NetworkError`
  - `UnknownFunction`
  - `StaticModificationError`
- [x] Implement `Display` and `Debug`
- [x] Create `CorvoResult<T>` type alias
- [x] Add source location tracking

### 1.3 Span/Location Tracking (`src/span.rs`)
- [x] Define `Span` struct with `start: Position`, `end: Position`
- [x] Define `Position` struct with `line`, `column`, `offset`
- [x] Implement `Span::merge()`
- [x] Implement `Display` for nice error messages

---

## Phase 2: Lexer (Tokenizer)

### 2.1 Token Types (`src/lexer/token.rs`)
- [x] Define `TokenType` enum:
  - Keywords: `static`, `var`, `try`, `fallback`, `loop`, `browse`, `terminate`, `assert_eq`, `assert_neq`, `assert_gt`, `assert_lt`, `assert_match`
  - Operators: `{`, `}`, `(`, `)`, `[`, `]`, `,`, `.`, `:`
  - Literals: `String`, `Number`, `Boolean`, `Identifier`
  - Special: `StringInterpolation`
- [x] Define `Token` struct with `token_type`, `span`
- [x] Implement `Display` for tokens

### 2.2 Lexer Implementation (`src/lexer/tokenizer.rs`)
- [x] Create `Lexer` struct with source, position, etc.
- [x] Implement `next_token() -> Token`
- [x] Handle string literals with double quotes
- [x] Handle string interpolation `${...}` (produces `StringInterpolation` tokens)
- [x] Handle numbers (integers and floats)
- [x] Handle identifiers and keywords
- [x] Handle named parameters `name: value` (at parser level)
- [x] Handle comments (`#` to end of line)
- [x] Skip whitespace
- [x] Handle errors gracefully

### 2.3 Lexer Tests (52 tests)
- [x] Test string tokenization (8 tests: plain, empty, escapes, multiline, unterminated)
- [x] Test string interpolation parsing (6 tests: simple, complex expr, multiple, math, escaped dollar)
- [x] Test number tokenization (5 tests: integer, float, zero, trailing dot)
- [x] Test keyword recognition (3 tests: basic, all keywords, booleans)
- [x] Test named parameters (via parser tests)
- [x] Test comments (3 tests: standalone, before statement, inline)
- [x] Test edge cases (empty strings, special chars, illegal chars, whitespace-only, empty input)
- [x] Test next_token() sequential lexing
- [x] Test token spans and display

---

## Phase 3: AST (Abstract Syntax Tree)

### 3.1 Expression Types (`src/ast/expr.rs`)
- [x] Define `Expr` enum:
  - `Literal(Value)`
  - `VarGet { name: String }`
  - `StaticGet { name: String }`
  - `StringInterpolation { parts: Vec<Expr> }`
  - `FunctionCall { name: String, args: Vec<Expr>, named_args: HashMap<String, Expr> }`
  - `IndexAccess { target: Box<Expr>, index: Box<Expr> }`
  - Note: `BinaryOp` not needed - Corvo uses function calls (`math.add`) instead of operators (`+`)

### 3.2 Statement Types (`src/ast/stmt.rs`)
- [x] Define `Stmt` enum:
  - `StaticSet { name: String, value: Expr }`
  - `VarSet { name: String, value: Expr }`
  - `ExprStmt { expr: Expr }`
  - `TryBlock { body: Vec<Stmt>, fallbacks: Vec<FallbackBlock> }`
  - `Loop { body: Vec<Stmt> }`
  - `Browse { iterable: Expr, key: String, value: String, body: Vec<Stmt> }`
  - `Terminate`
  - `Assert { kind: AssertKind, args: Vec<Expr> }`

### 3.3 AST Definitions (`src/ast/node.rs`)
- [x] Define `Program` struct with `statements: Vec<Stmt>`
- [x] Define `AssertKind` enum: `Eq`, `Neq`, `Gt`, `Lt`, `Match`

---

## Phase 4: Parser

### 4.1 Parser Implementation (`src/parser/recursive_descent.rs`)
- [x] Create `Parser` struct with tokens and position
- [x] Implement `parse() -> Program`
- [x] Implement `parse_statement() -> Stmt`
- [x] Implement `parse_expression() -> Expr` with `parse_postfix()` for index access
- [x] Parse try/fallback blocks (including multiple fallbacks, nested)
- [x] Parse loop blocks
- [x] Parse browse blocks (`browse(expr, key, value) { body }`)
- [x] Parse named parameters (mixed positional + named)
- [x] Parse string interpolation
- [x] Parse list literals `[1, 2, 3]`
- [x] Parse map literals `{"key": "value"}`
- [x] Parse var.set(), var.get(), static.set(), static.get()
- [x] Parse index access `target[index]` (including chained `a[0][1]`)
- [x] Error messages include token span information

### 4.2 Parser Tests (33 tests)
- [x] Test static variable assignment
- [x] Test var assignment
- [x] Test try/fallback parsing (single, multiple, nested)
- [x] Test loop parsing (single stmt, multiple stmts)
- [x] Test all assertion kinds (eq, neq, gt, lt, match)
- [x] Test function calls with named args
- [x] Test method calls (string.concat, etc.)
- [x] Test mixed positional/named arguments
- [x] Test multiple statements
- [x] Test comments handling (standalone, between statements)
- [x] Test list literals (empty, with items)
- [x] Test map literals (empty, with entries)
- [x] Test index access (single, chained)
- [x] Test terminate statement
- [x] Test error cases (missing paren, brace, comma, bad token)
- [x] Test edge cases (empty input, whitespace only, comments only)
- [x] Test complex program with nested structures

---

## Phase 5: Type System

### 5.1 Value Types (`src/type_system/value.rs`)
- [x] Define `Value` enum:
  - `String(String)`
  - `Number(f64)`
  - `Boolean(bool)`
  - `List(Vec<Value>)`
  - `Map(HashMap<String, Value>)`
  - `Null`
- [x] Implement `Display`, `Debug`, `Clone`, `Default`
- [x] Implement `PartialEq`, `Serialize`, `Deserialize`
- [x] Add type checking methods (`as_string`, `as_number`, `as_bool`, `as_list`, `as_map`, `is_truthy`, `r#type()`)

### 5.2 Type Definitions (`src/type_system/types.rs`)
- [x] Define `Type` enum matching Value variants
- [x] Implement `Type::from_value()`
- [x] Implement `Type::parse_name()`
- [x] Implement `Type::as_str()` and `Display`

### 5.3 Type Methods (`src/type_system/type_methods.rs`) - 47 tests
- [x] `string` methods (12 methods, 12 tests):
  - [x] `concat`, `replace`, `split`, `trim`, `contains`
  - [x] `starts_with`, `ends_with`
  - [x] `to_lower`, `to_upper`, `len`, `reverse`, `is_empty`
- [x] `number` methods (9 methods, 9 tests):
  - [x] `to_string`, `parse`, `is_nan`, `is_infinite`, `is_finite`
  - [x] `abs`, `floor`, `ceil`, `round`, `sqrt`
- [x] `list` methods (10 methods, 13 tests):
  - [x] `push`, `pop`, `get`, `set`, `first`, `last`
  - [x] `len`, `is_empty`, `contains`, `reverse`, `join`
- [x] `map` methods (9 methods, 13 tests):
  - [x] `keys`, `values`, `len`, `is_empty`, `has_key`
  - [x] `get` (with default), `set`, `remove`, `merge`

---

## Phase 6: Runtime

### 6.1 Runtime State (`src/runtime/state.rs`) - 22 tests
- [x] Create `RuntimeState` struct (with `Clone`)
- [x] `vars: HashMap<String, Value>`
- [x] `statics: HashMap<String, Value>`
- [x] Implement `var_get()`, `var_set()`, `var_remove()`, `has_var()`, `var_keys()`, `var_count()`, `clear_vars()`
- [x] Implement `static_get()`, `static_set()`, `static_remove()`, `has_static()`, `static_keys()`, `static_count()`, `clear_statics()`
- [x] Implement `is_empty()`, `total_count()`, `Default`, `Clone`
- [x] Tests for all operations (get, set, overwrite, remove, keys, count, clear, independence)

### 6.2 Scope Management (`src/runtime/scope.rs`) - 13 tests
- [x] Create `Scope` struct (with `Clone`)
- [x] Implement `get()`, `set()`, `define()`, `contains()`
- [x] Implement `with_parent()`, `is_root()`, `depth()`
- [x] Implement `local_keys()`, `local_count()`
- [x] Handle variable shadowing (child overrides parent)
- [x] Handle `set()` propagation to parent for existing vars
- [x] Tests: new, with_parent, nested, shadowing, contains, local_keys

---

## Phase 7: Standard Library

### 7.1 Core System (`src/standard_lib/sys.rs`) - 9 tests
- [x] `sys.echo(msg: string)`
- [x] `sys.read_line(prompt: string)`
- [x] `sys.sleep(ms: number)`
- [x] `sys.panic(msg: string)`

### 7.2 OS Operations (`src/standard_lib/os.rs`) - 8 tests
- [x] `os.get_env(key: string, default: string)`
- [x] `os.set_env(key: string, value: string)`
- [x] `os.exec(cmd: string)` -> Map {stdout, stderr, code}
- [x] `os.info()` -> Map {os, arch, hostname}

### 7.3 Math Operations (`src/standard_lib/math.rs`) - 13 tests
- [x] `math.add(a, b)`
- [x] `math.sub(a, b)`
- [x] `math.mul(a, b)`
- [x] `math.div(a, b)` (with zero-division check)
- [x] `math.mod(a, b)` (with zero-division check)

### 7.4 File System (`src/standard_lib/fs.rs`) - 9 tests
- [x] `fs.read(path)`
- [x] `fs.write(path, content)`
- [x] `fs.append(path, content)`
- [x] `fs.delete(path)`
- [x] `fs.exists(path)`
- [x] `fs.mkdir(path, recursive)`
- [x] `fs.list_dir(path)`
- [x] `fs.copy(src, dest)`
- [x] `fs.move(src, dest)`
- [x] `fs.stat(path)` -> Map {size, is_dir, permissions, modified_at}

### 7.5 Networking (`src/standard_lib/http.rs`, `dns.rs`)
- HTTP:
  - [x] `http.get(url, headers)`
  - [x] `http.post(url, body, headers)`
  - [x] `http.put(url, body, headers)`
  - [x] `http.delete(url, headers)`
- DNS:
  - [x] `dns.resolve(hostname)`
  - [x] `dns.lookup(ip)`

### 7.6 Remote Operations (`src/standard_lib/ssh.rs`, `rsync.rs`)
- [ ] Not yet implemented (deferred)

### 7.7 Data Parsing (`src/standard_lib/json.rs`, `yaml.rs`, `hcl.rs`, `csv.rs`, `xml.rs`) - 12 json tests
- [x] `json.parse(data)` - full serde_json roundtrip
- [x] `json.stringify(data)` - pretty-printed output
- [x] `yaml.parse(data)`, `yaml.stringify(data)`
- [x] `hcl.parse(data)`, `hcl.stringify(data)` (stubs)
- [x] `csv.parse(data, delimiter)`
- [x] `xml.parse(data)`

### 7.8 Security (`src/standard_lib/crypto.rs`) - 8 tests
- [x] `crypto.hash(algorithm, data)` (md5, sha256, sha512)
- [x] `crypto.encrypt(data, key)` (XOR + base64)
- [x] `crypto.decrypt(data, key)` (roundtrip tested)
- [x] `crypto.uuid()` (v4 UUID)

### 7.9 AI/LLM (`src/standard_lib/llm.rs`)
- [x] `llm.model(name, provider, options)` (stub)
- [x] `llm.prompt(model, prompt, tokens)` (stub)
- [x] `llm.embed(model, text)` (stub)
- [x] `llm.chat(id, model, messages, tokens)` (stub)

---

## Phase 8: Evaluator/Interpreter

### 8.1 Expression Evaluation (`src/compiler/evaluator.rs`)
- [x] Implement `eval_expr(expr, state) -> Value`
- [x] Handle all expression types (Literal, VarGet, StaticGet, StringInterpolation, FunctionCall, IndexAccess)
- [x] Implement string interpolation (multi-part, expressions inside ${})
- [x] Implement function calls with named args
- [x] Implement type methods (string.*, number.*, list.*, map.*)
- [x] Internal `__list__` and `__map__` for literal syntax

### 8.2 Statement Execution (`src/compiler/evaluator.rs`)
- [x] Implement `exec_stmt(stmt, state)`
- [x] Handle static variable assignment
- [x] Handle var assignment
- [x] Implement try/fallback logic (multiple fallbacks, nested)
- [x] Implement loop execution (with error recovery)
- [x] Implement browse execution (iterate list by index, map by sorted key)
- [x] Implement terminate (breaks loop/browse + program)
- [x] Implement assertions

### 8.3 Built-in Assertions - 48 evaluator tests (up from 21)
- [x] `assert_eq(a, b)` - pass + fail test
- [x] `assert_neq(a, b)` - pass + fail test
- [x] `assert_gt(a, b)` - pass + fail test
- [x] `assert_lt(a, b)` - pass + fail test
- [x] `assert_match(regex, target)` - pass + fail test
- [x] Error cases: var_not_found, static_not_found, unknown_function, index_out_of_bounds, div_by_zero
- [x] Complex programs: counter loop, nested try, method chaining

---

## Phase 9: REPL

### 9.1 REPL Implementation (`src/repl/mod.rs`) - 8 tests
- [x] `Repl` struct with persistent state and evaluator
- [x] `exit` / `quit` commands
- [x] `help` command with comprehensive language reference
- [x] `vars` command - list all defined variables
- [x] `reset` command - clear all state
- [x] Multi-line input (brace counting for try/fallback, loop blocks)
- [x] Persistent state across commands
- [x] Error recovery (errors don't crash REPL)

### 9.2 Help System
- [x] Built-in help with language reference
- [x] Example code snippets
- [x] Module documentation (sys, math, string, list, map, fs, json, crypto, http)

### 9.3 API
- [x] `run_source_with_state(source, state)` - execute with existing state
- [x] `run_repl()` - start interactive REPL

---

## Phase 10: CLI

### 10.1 Command-line Interface (`src/main.rs`)
- [x] Parse command-line arguments (via structopt)
- [x] `corvo <file>` - Execute file
- [x] `corvo --repl` / `-r` - Start REPL
- [x] `corvo --version` / `-v` - Print version
- [x] `corvo --eval <expr>` / `-e` - Evaluate expression from command line
- [x] `corvo --check <file>` - Check syntax without executing
- [x] Comprehensive help with examples
- [x] Proper exit codes for different error types

---

## Phase 11: Testing

### 11.1 Unit Tests - 308 tests
- [x] Lexer tests (67 tests)
- [x] Parser tests (33 tests)
- [x] Type system tests (64 tests: value, type, type_methods)
- [x] Evaluator tests (48 tests)
- [x] Standard library tests (58 tests: sys, os, math, fs, json, crypto)
- [x] Runtime tests (35 tests: state, scope)
- [x] REPL tests (8 tests)
- [x] Error tests (2 tests)
- [x] Span tests (3 tests)

### 11.2 Integration Tests - 50 tests (`tests/integration_test.rs`)
- [x] End-to-end programs (hello, arithmetic, strings, lists, maps)
- [x] try/fallback scenarios (success, failure, multiple fallbacks)
- [x] Error handling (parse, runtime, assertion errors)
- [x] Computation tests (factorial, fibonacci, accumulator)
- [x] Standard library integration (json, crypto, fs, os)
- [x] API tests (run_source, run_source_with_state)
- [x] browse block tests (list, map, empty, nested, terminate, type error)

### 11.3 Example Programs - 6 files in `examples/`
- [x] `hello.corvo` - Hello World
- [x] `variables.corvo` - Variables and arithmetic
- [x] `loop.corvo` - Loops with counters
- [x] `data_structures.corvo` - Lists and maps
- [x] `error_handling.corvo` - try/fallback patterns
- [x] `json_example.corvo` - JSON parsing
- [x] `crypto_example.corvo` - Hashing and encryption

---

## Phase 12: Compiler

### 12.1 Compilation to Executable (`src/compiler/builder.rs`) - 3 tests
- [x] `corvo --compile script.corvo` - Compile to standalone executable
- [x] `corvo --compile script.corvo -o myapp` - Custom output path
- [x] `corvo --compile --debug` - Debug build mode (faster compile)
- [x] Generates temporary Cargo project with embedded source
- [x] Links against corvo-lang crate via path dependency
- [x] Produces standalone executable (~5MB)
- [x] Compiled binary exits with proper error codes

### 12.2 How It Works
1. Reads the .corvo source file
2. Creates temporary Cargo project in /tmp/corvo_build
3. Generates main.rs with source embedded as string constant
4. Compiles with `cargo build --release` (or debug)
5. Copies resulting binary to target path
6. Binary executes embedded source on startup

---

## Phase 13: Documentation

- [x] Update `corvo.md` with language spec
- [x] Update `IMPLEMENTATION.md` with AI directives and Transpiler details
- [x] Add inline code documentation
- [x] Create CONTRIBUTING.md

---

## Phase 14: Transpiler

- [x] Implement Corvo-to-Rust code generation (`src/compiler/transpiler.rs`)
- [x] Support procedure-to-closure mapping
- [x] Support `async_browse` with mutex-protected shared variables
- [x] Implement complex write-back logic (delta-merge for List, Number, String)
- [x] Project scaffolding (Cargo.toml, main.rs templates)
- [x] Verify parity with interpreted mode via integration tests

---

## Implementation Order (TDD Approach)

1. **Setup**: Project structure, Cargo.toml
2. **Lexer**: Token types → Tokenizer → Tests
3. **AST**: Expression types → Statement types → Tests
4. **Parser**: Parser implementation → Tests
5. **Type System**: Value types → Type methods
6. **Runtime**: State management → Scope handling
7. **Evaluator**: Expression eval → Statement exec → Assertions
8. **Standard Lib**: Implement each library module
9. **REPL**: REPL implementation with help system
10. **CLI**: Command-line interface
11. **Integration**: Example programs, full testing

---

## Design Principles

- **TDD**: Write tests before implementation
- **Clean Code**: Small functions, single responsibility
- **Error Messages**: Clear, actionable, with source locations
- **Extensibility**: Easy to add new standard library functions
- **Performance**: Lazy evaluation where possible
- **Safety**: No memory leaks, proper error handling
