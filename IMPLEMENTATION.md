# Corvo Language Reference (AI Assistant Context)

## 1. AI Generation Directives
When generating Corvo code, you **must** strictly adhere to the following language rules:
* **NO USER-DEFINED FUNCTIONS:** Do not generate `def`, `fn`, `function`, or lambda expressions. All operations must use the built-in library calls. Use `procedure` blocks when reusable logic is needed (see Section 3).
* **NO IF/ELSE STATEMENTS:** Do not generate `if`, `elif`, `else`, or `switch`. For simple value-based branching, use the `match` expression. For error handling and complex conditional logic, use `try { ... } fallback { ... }` combined with `assert_*` commands.
* **NO ASSIGNMENT OPERATORS:** Do not use `=` for assignment. State is strictly managed via `var.set()` and `static.set()` (the latter only inside a `prep` block).
* **STRING INTERPOLATION:** Use `${}` for string interpolation (e.g., `sys.echo("Value: ${var.get("key")}")`).
* **NAMED PARAMETERS:** Library functions support Python-like named parameters for clarity (e.g., `http.get(url: "https://...")`).
* **IMMUTABILITY OF METHODS:** Type methods (like `string.replace`) do not mutate the variable in place; they return a new value that must be reassigned via `var.set()`.
* **VARIABLE SHORTHAND:** `@name` is shorthand for `var.get("name")`; `@name = value` is shorthand for `var.set("name", value)`. Only use `@` for regular runtime variables.
* **BROWSE BINDINGS:** Inside a `browse` block, the key and value bindings declared in the header with `@` are accessed with the `@` prefix (e.g., `@key`, `@value`), just like regular runtime variables.
* **PREP BLOCK:** `static.set()` is **only** allowed inside a `prep { }` block. The `prep` block must appear before all other statements. Variables created inside `prep` are not available outside it.

## 2. Type System & Type Methods
Corvo is strongly and limited typed. A variable's type is inferred on its first assignment and cannot change. Type methods are called using the type namespace (e.g., `string.method(value, ...)`).

### `string`
* `string.concat(str1: string, str2: string) -> string`
* `string.replace(target: string, old: string, new: string) -> string`
* `string.replace_first(target: string, old: string, new: string) -> string`
* `string.split(target: string, delimiter: string) -> list`
* `string.trim(target: string) -> string`
* `string.trim_start(target: string) -> string`
* `string.trim_end(target: string) -> string`
* `string.contains(target: string, substr: string) -> boolean`
* `string.starts_with(target: string, prefix: string) -> boolean`
* `string.ends_with(target: string, suffix: string) -> boolean`
* `string.to_lower(target: string) -> string`
* `string.to_upper(target: string) -> string`
* `string.len(target: string) -> number`  (UTF-8 byte length; use `string.chars` + `list.len` for Unicode character count)
* `string.reverse(target: string) -> string`
* `string.is_empty(target: string) -> boolean`
* `string.pad_start(target: string, width: number, fill?: string) -> string`
* `string.pad_end(target: string, width: number, fill?: string) -> string`
* `string.substring(target: string, start: number, end?: number) -> string`
* `string.index_of(target: string, needle: string, start?: number) -> number`
* `string.char_at(target: string, index: number) -> string`
* `string.repeat(target: string, count: number) -> string`
* `string.count(target: string, needle: string) -> number`
* `string.chars(target: string) -> list`

### `number`
* `number.to_string(num: number) -> string`
* `number.parse(str: string) -> number` (Fails if invalid, triggering fallback)
* `number.is_nan(num: number) -> boolean`

### `list`
* `list.push(target: list, item: any) -> list` (Returns a new list with the item appended)
* `list.pop(target: list) -> list` (Returns a new list without the last item)
* `list.get(target: list, index: number) -> any`
* `list.set(target: list, index: number, value: any) -> list` (Returns a new list with the item at index replaced)
* `list.first(target: list) -> any`
* `list.last(target: list) -> any`
* `list.len(target: list) -> number`
* `list.is_empty(target: list) -> boolean`
* `list.contains(target: list, item: any) -> boolean`
* `list.reverse(target: list) -> list`
* `list.join(target: list, delimiter: string) -> string`
* `list.sort_version(target: list) -> list` (GNU `strverscmp`-compatible sort; elements compared via string form)
* `list.sort_maps_by_key(target: list, key: string, reverse?: boolean) -> list`

### `map`
* `map.keys(target: map) -> list`
* `map.values(target: map) -> list`
* `map.len(target: map) -> number`
* `map.is_empty(target: map) -> boolean`
* `map.has_key(target: map, key: string) -> boolean`
* `map.get(target: map, key: string, default: any) -> any`
* `map.set(target: map, key: string, value: any) -> map` (Returns a new map with the updated key)
* `map.remove(target: map, key: string) -> map` (Returns a new map without the specified key)
* `map.merge(target: map, other: map) -> map` (Returns a new map with keys from both maps)

## 3. State Management

### 3.1 Runtime Variables (`var`)
Used for dynamic data that changes during execution.
```corvo
var.set("target_dir", "/var/www/html")
# Type method usage
var.set("target_dir", string.concat(var.get("target_dir"), "/public"))

# @ shorthand: @name = var.get("name"), @name = val = var.set("name", val)
@target_dir = "/var/www/html"
sys.echo(@target_dir)

# Compound assignment shorthands (number)
@count = 0
@count++        # @count = @count + 1
@count--        # @count = @count - 1
@count += 5     # @count = @count + 5
@count -= 2     # @count = @count - 2

# Compound assignment shorthands (string)
@msg = "hello"
@msg += " world"   # concatenate
@msg -= "hello"    # remove all occurrences

# or= shorthand: assign the first truthy candidate (errors are skipped)
@cfg = map.new()
@host or= (@cfg["host"], "localhost")   # "localhost" — missing key is skipped
@flag or= (false, false, true)          # true
```

### 3.2 Compile-Time Constants (`static`)
Used for configuration and immutable values. These are baked into the compiled binary. `static.set()` is **only** allowed inside a `prep` block, which must appear before all other statements. Variables created inside `prep` are not available outside it.
```corvo
prep {
    static.set("appName", "Colonia_Agent")
    static.set("db_password", os.get_env("DB_PASS"))
}
```

### 3.3 Browse Bindings (`@`)
Inside a `browse` block, the key and value names declared in the block header (prefixed with `@`) are accessed with the `@` prefix, just like any other runtime variable.
```corvo
browse(@items, @k, @v) {
    sys.echo("${@k} -> ${@v}")
}
```

## 4. Control Flow

### 4.1 Try / Fallback (Ifless Architecture)
Conditionals and error handling are unified. Execution proceeds linearly until an `assert_*` fails or a command errors out, which instantly triggers the next `fallback` block.

**Built-in Assertions:**
* `assert_eq(a, b)`: Asserts `a` equals `b`.
* `assert_neq(a, b)`: Asserts `a` does not equal `b`.
* `assert_gt(a, b)`: Asserts `a` > `b`.
* `assert_lt(a, b)`: Asserts `a` < `b`.
* `assert_match(regex: string, target: string)`: Asserts string matches regex.

### 4.2 Loops (`loop` & `terminate`)
Corvo supports a single, infinite loop construct. The only way to exit is by calling `terminate`.

### 4.3 Browse (`browse`)
`browse` iterates over a list or map, exposing a key and value variable on each iteration. Variable names for the key and value are chosen by the caller and declared with `@`.

**Syntax:** `browse(<expr>, @<key_ident>, @<value_ident>) { <body> }`

* For a **list**: `@key` is the zero-based numeric index; `@value` is the element.
* For a **map**: `@key` is the string key; `@value` is the associated value. Keys are visited in sorted order.
* Inside the block, access bindings with the `@` prefix: `@key`, `@value`. Use `${@name}` for string interpolation.
* Pass a browse-bound value to a nested `browse` (or any function) using `@name`.
* `terminate` exits the browse block early (same semantics as inside `loop`).
* Browse blocks may be nested.
* Using `browse` on a non-list/non-map value raises a type error.

```corvo
# List example — use @idx and @fruit to access the browse bindings
@fruits = ["apple", "banana", "cherry"]
browse(@fruits, @idx, @fruit) {
    sys.echo("${@idx}: ${@fruit}")
}

# Map example — use @key and @val
@config = {"host": "localhost", "port": 8080}
browse(@config, @key, @val) {
    sys.echo("${@key} = ${@val}")
}

# Early exit — use @v in assert_eq and echo
@items = [1, 2, 3, 4, 5]
browse(@items, @k, @v) {
    sys.echo(@v)
    try {
        assert_eq(@v, 3)
        terminate
    } fallback {}
}

# Nested browse — pass @row (browse-bound) to inner browse
@matrix = [[1, 2], [3, 4]]
browse(@matrix, @row_idx, @row) {
    browse(@row, @col_idx, @cell) {
        sys.echo("[${@row_idx}][${@col_idx}] = ${@cell}")
    }
}
```

### 4.4 Async Browse (`async_browse`)

`async_browse` is the parallel counterpart of `browse`.  It spawns one OS thread per list element and runs a procedure on each element concurrently.

**Syntax:**
```
async_browse(@list, @proc, @item_binding [, shared @var1, shared @var2, ...])
```

* `@list` — any expression that evaluates to a list.
* `@proc` — a variable holding a `procedure` value.
* `@item_binding` — the name bound to the current list element inside each thread.
* `shared @var` — outer variable shared between all threads (optional, can be repeated).

**Procedure signature:**  the procedure must accept exactly `1 + len(shared_vars)` parameters.  The first parameter receives the item; subsequent parameters receive the shared variables in the order they are listed.

**Write-back semantics for shared variables:**

| Shared var type | Semantics |
|---|---|
| `list` | Delta-merge: items appended during the procedure body are atomically added to the mutex-protected list. |
| `number` | Delta-merge: the numeric difference added by the thread is summed into the shared total. |
| `string` | Delta-merge: the string suffix appended by the thread is concatenated to the shared value. |
| Other types | Last-writer-wins: the thread's final value replaces the current value. |

The procedure body runs **without holding any lock**, so I/O-bound operations execute in true parallel.  The delta-merge step is protected by a per-variable mutex.

```corvo
@files = ["a.txt", "b.txt", "c.txt"]
@results = list.new()

@read_file = procedure(@path, @acc) {
    @content = fs.read(@path)
    @acc = list.push(@acc, @content)
}

async_browse(@files, @read_file, @path, shared @results)
# @results contains all three file contents (order non-deterministic)
```

**Example file:** [`examples/async_browse.corvo`](examples/async_browse.corvo)

### 4.5 Match Expression (`match`)
`match` is the primary substitute for `if/else` chains in Corvo. It evaluates an expression against a list of literal patterns and returns the value of the first matching arm. Use `_` as a catch-all wildcard.

**Syntax:**
```corvo
match(<expr>) {
    <pattern> => <value>,
    <pattern> => <value>,
    _ => <default_value>
}
```

* Patterns must be string, number, or boolean literals, or `_` (wildcard).
* Arms are evaluated in declaration order; the first match wins.
* If no arm matches and there is no wildcard, a runtime error is raised.
* `match` is an expression — it returns a value and can be used anywhere an expression is expected, including on the right-hand side of `@name =` or `var.set()`.

```corvo
# Assign a label based on a status code
@label = match(@status_code) {
    200 => "OK",
    404 => "Not Found",
    500 => "Internal Server Error",
    _ => "Unknown"
}

# Choose a config file based on the environment
@config_path = match(os.get_env("ENV", "dev")) {
    "prod"    => "/etc/app/config.json",
    "staging" => "/etc/app/staging.json",
    _         => "./config.dev.json"
}

# Readable string-based branching (replaces if/else chains)
@greeting = match(@lang) {
    "en" => "Hello",
    "es" => "Hola",
    "fr" => "Bonjour",
    _    => "Hi"
}
```

**Example file:** [`examples/match.corvo`](examples/match.corvo)

## 5. Comprehensive Standard Library

*(Note: All libraries support the `?` operator for in-code/REPL documentation, e.g., `fs.read?`)*

### Core & System (`sys`, `os`, `math`)
* `sys.echo(msg: string)`: Prints to stdout.
* `sys.read_line(prompt: string) -> string`: Reads user input from stdin.
* `sys.sleep(ms: number)`: Pauses execution.
* `sys.panic(msg: string)`: Terminates with a non-zero exit code.
* `sys.exit(code?: number)`: Terminates with the given exit code (default `0`).
* `sys.exec(cmd: list, input?: string, check?: boolean, timeout?: number, cwd?: string, env?: map) -> map`: Executes a process directly without a shell. The first argument is a list of strings where the first element is the program and the remaining elements are its arguments (e.g., `["ls", "-la", "/tmp"]`). Returns `{"stdout": string, "stderr": string, "code": number}`. Named parameters: `input` (data piped to stdin), `check` (error on non-zero exit), `timeout` (kill after N seconds, triggers fallback on timeout), `cwd` (working directory), `env` (environment variables). Use `sys.exec` when you need direct process invocation, piping, timeouts, or environment control.
* `os.get_env(key: string, default: string) -> string`
* `os.set_env(key: string, value: string)`
* `os.exec(cmd: string, args: list) -> map`: Simple process execution without a shell. Returns `{"stdout": string, "stderr": string, "code": number}`. Use `os.exec` for direct process invocation when you have a command and its arguments as separate values and do not need shell features.
* `os.info() -> map`: Returns `{"os": string, "arch": string, "hostname": string}`.
* `os.argv() -> list`: Arguments for the Corvo program (trailing tokens after the script when using `corvo file.corvo …`, or `std::env::args().skip(1)` for a compiled binary). Empty in the REPL and in `run_source` unless set via `RuntimeState::set_script_argv` / `run_source_with_script_argv`.
* `args.parse(argv: list, config?: map) -> map`: Generic configurable argv parser. Returns `{"positional": list[string], "options": map, "plus"?: map, "at_servers"?: list}`. Supports GNU coreutils style (aliases, short clusters, long options `--k=v` or `--k v`, optional values, accumulate), dnsutils/dig style (`+flag`, `+noflag`, `+key=val`, `@server`), and usbutils style (colon-compound short values). See `CHEATSHEET.md` for the full config key reference.
* `args.scan(argv: list) -> map`: Zero-config wrapper around `args.parse` for backward compatibility. Returns `{"positional": list[string], "options": map}`.
* `math.add(a: number, b: number) -> number`
* `math.sub(a: number, b: number) -> number`
* `math.mul(a: number, b: number) -> number`
* `math.div(a: number, b: number) -> number`
* `math.mod(a: number, b: number) -> number`
* `math.max(a: number, b: number, ...numbers) -> number`
* `math.human_bytes(bytes: number, si?: boolean) -> string`: Human-readable byte size (binary prefixes by default; `si: true` uses decimal SI prefixes).

### Time (`time`)
* `time.format_local(seconds: number, nanoseconds?: number, format: string) -> string`: `strftime` in the local timezone (honours `TZ`).
* `time.unix_now() -> number`: Seconds since the Unix epoch.

### Hashing & Encryption (`crypto`)
* `crypto.hash(algorithm: string, data: string) -> string` (Supports "md5", "sha256", "sha512")
* `crypto.hash_file(algorithm: string, path: string) -> string` (Hashes a file's contents; supports "md5", "sha256", "sha512")
* `crypto.checksum(path: string) -> string` (SHA-256 checksum of a file; returns 64-char hex string)
* `crypto.encrypt(data: string, key: string) -> string` (AES-GCM)
* `crypto.decrypt(data: string, key: string) -> string`
* `crypto.uuid() -> string` (Generates a UUIDv4)

### File System (`fs`)
* `fs.read(path: string) -> string`
* `fs.write(path: string, content: string) -> boolean`
* `fs.append(path: string, content: string) -> boolean`
* `fs.delete(path: string) -> boolean`
* `fs.exists(path: string) -> boolean`
* `fs.mkdir(path: string, recursive: boolean) -> boolean`
* `fs.list_dir(path: string) -> list`
* `fs.copy(src: string, dest: string) -> boolean`
* `fs.move(src: string, dest: string) -> boolean`
* `fs.stat(path: string) -> map`: Returns `{"size": number, "is_dir": boolean, "permissions": string, "modified_at": number}`.
* `fs.read_meta(path: string, follow_symlinks?: boolean) -> map`: Rich metadata for a path (mode, inode, nlink, uid/gid, user/group names on Unix, symlink target, block count, timestamp fields, etc.). Non-Unix builds use placeholders for unavailable fields.
* `fs.read_dir_meta(path: string, follow_symlinks?: boolean) -> list`: Directory listing; each entry is a map with the same shape as `fs.read_meta`.
* `fs.read_link(path: string) -> string`: Symlink target.

### Networking & Web (`http`, `net`, `dns`)
* `http.get(url: string, headers: map) -> map`: Returns `{"status_code": number, "response_body": string, "headers": map}`.
* `http.post(url: string, body: string, headers: map) -> map`
* `http.put(url: string, body: string, headers: map) -> map`
* `http.delete(url: string, headers: map) -> map`
* `net.tcp_listen(address: string) -> map`: Returns a `tcp_listener` handle (`kind`, `id`, `local_addr`).
* `net.tcp_accept(listener: map) -> map`: Blocks until a connection arrives; returns `tcp_stream` (`kind`, `id`, `local_addr`, `peer_addr`).
* `net.tcp_close_listener(listener: map) -> null`
* `net.tcp_connect(address: string) -> map`: Client-side `tcp_stream` handle.
* `net.tcp_read(stream: map, max_bytes: number) -> string`: Lossy UTF-8 for arbitrary bytes.
* `net.tcp_write(stream: map, data: string) -> null`
* `net.tcp_close(stream: map) -> null`
* `dns.resolve(hostname: string) -> list`: Returns list of IP addresses.
* `dns.lookup(ip: string) -> string`: Returns hostname.

### Remote Operations (`ssh`, `rsync`)
* `ssh.exec(host: string, user: string, key_path: string, cmd: string) -> map`: Returns `{"stdout": string, "stderr": string, "code": number}`.
* `ssh.scp_upload(host: string, user: string, key_path: string, local_path: string, remote_path: string) -> boolean`
* `ssh.scp_download(host: string, user: string, key_path: string, remote_path: string, local_path: string) -> boolean`
* `rsync.sync(source: string, dest: string, options: list) -> boolean`

### Data Parsing (`json`, `yaml`, `hcl`, `csv`, `xml`)
* `json.parse(data: string) -> map`
* `json.stringify(data: map) -> string`
* `yaml.parse(data: string) -> map`
* `yaml.stringify(data: map) -> string`
* `hcl.parse(data: string) -> map` (Essential for parsing Terraform configs)
* `hcl.stringify(data: map) -> string`
* `csv.parse(data: string, delimiter: string) -> list`
* `xml.parse(data: string) -> map`

### Templating (`template`)
* `template.render(template: string, data: map) -> string`: Render a [Handlebars](https://handlebarsjs.com) template string using the key/value pairs in `data`. Missing keys render as empty strings. Example: `template.render("Hello, {{name}}!", {"name": "World"})` → `"Hello, World!"`.
* `template.render_file(path: string, data: map) -> string`: Load a Handlebars template from the file at `path` and render it with `data`.

### Security & Crypto (`crypto`)
* `crypto.hash(algorithm: string, data: string) -> string` (Supports "md5", "sha256", "sha512")
* `crypto.hash_file(algorithm: string, path: string) -> string` (Hashes a file's contents; supports "md5", "sha256", "sha512")
* `crypto.checksum(path: string) -> string` (SHA-256 checksum of a file; returns 64-char hex string)
* `crypto.encrypt(data: string, key: string) -> string` (AES-GCM)
* `crypto.decrypt(data: string, key: string) -> string`
* `crypto.uuid() -> string` (Generates a UUIDv4)

### Artificial Intelligence (`llm`)
* `llm.model(name: string, provider: string, options: map) -> string`: Tests and build a connection string for the other llm functions. The provider can be "openai", "gemini", "azure", "local (llama.cpp)", "ollama". The options map can include parameters like API keys, model versions, etc.
* `llm.prompt(model: string, prompt: string, tokens: number) -> string`: Executes a prompt against the specified model (the connection string returned by `llm.model`) and returns the generated text.
* `llm.embed(model: string, text: string) -> list`: Returns a vector embedding for the given text using the specified model.
* `llm.chat(id: string, model: string, messages: list, tokens: number) -> map`: Executes a chat conversation with the specified model. The `messages` parameter is a list of maps, each containing `{"role": "user|assistant|system", "content": string}`. The function returns the assistant's response.

## 3. Procedures

Corvo does not have user-defined functions. Instead it has **procedures** — named, callable blocks that operate on variables by reference. Procedures are ideal for eliminating code duplication without introducing return values or closures.

### Syntax

```
@proc_name = procedure(@param1, @param2, ...) {
    # body — may contain any statements except prep blocks
}
@proc_name.call(@arg1, @arg2, ...)
```

### Semantics

* **Definition**: `procedure(@p1, @p2, ...) { body }` evaluates to a `procedure` value and is assigned to a variable with `@` like any other value.
* **Invocation**: `.call(...)` must be used as a **statement** (not in an expression context). It accepts the same number of arguments as the parameter list.
* **Pass-by-reference (copy-in / copy-out)**: Each argument that is a plain `@variable` is copied into the corresponding parameter name before the body runs. After the body finishes, the updated parameter value is written back to the caller's variable. Arguments that are literals or expressions are copied in but not written back.
* **Parameter isolation**: Parameter variable names are removed from state after the call returns. They do not leak into the outer scope.
* **No return value**: Procedures return `null` and cannot be used on the right-hand side of an assignment.

### Example

```corvo
@add = procedure(@a, @b, @out) {
    @out = math.add(@a, @b)
}

@n1 = 10
@n2 = 21
@total = 0
@add.call(@n1, @n2, @total)
sys.echo(@total)   # 31
```

See [`examples/procedure.corvo`](examples/procedure.corvo) for more examples.

## 6. Transpiler (Corvo to Rust)

Corvo features a transpiler that converts `.corvo` source code directly into a standalone Rust project. Unlike the default compiler (which embeds the source and runs it via an internal interpreter), the transpiler generates native Rust code that maps Corvo constructs to Rust standard library and Corvo-runtime calls.

### 6.1 Transpilation Process
1. **Source Loading**: The transpiler reads the `.corvo` source.
2. **Pre-execution**: `prep { }` blocks are executed using the internal interpreter to resolve `static.set()` values.
3. **Rust Generation**:
    - **Variables**: State is managed via a `RuntimeState` object passed between functions.
    - **Procedures**: Transpiled into Rust closures (`NativeProcedure`).
    - **Control Flow**:
        - `loop`: Transpiled into Rust `loop { ... }`.
        - `try/fallback`: Transpiled into `Result`-based pattern matching (using `ifLet` or `match`).
        - `async_browse`: Transpiled into multi-threaded Rust code that uses `Arc<Mutex<Value>>` for shared variables and handles concurrent write-backs via `merge_shared_writeback`.
4. **Project Scaffolding**: A temporary or permanent Cargo project is created, with a generated `main.rs`, `Cargo.toml`, and appropriate dependencies.
5. **Compilation**: The resulting Rust code is compiled using `cargo build`.

### 6.2 Key Transpilation Mappings
- **List Literals**: Converted to `Value::List(vec![...])`.
- **Map Literals**: Converted to `Value::Map(HashMap::from([...]))`.
- **String Interpolation**: Converted to `format!()` calls within the Rust code.
- **Shared Variables**: Wrapped in `Arc<Mutex<Value>>` before spawning threads in `async_browse`.

### 6.3 Usage
```bash
corvo --transpile script.corvo -o my_rust_project
cd my_rust_project
cargo run
```

