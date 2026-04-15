# Corvo

![corvo-logo](./corvo.jpg)

**Write scripts like prose. Ship them like binaries. Trust them like Rust.**

Corvo is a modern scripting language that compiles to standalone Rust binaries. It is deliberately stripped of the things that make scripting languages fragile — no package manager, no dependency graph, no `import` statement, no `if/else` tangles, no function signatures to maintain. What remains is a language that is easy to read, audit, and ship anywhere.

```corvo
# Fetch an API, parse JSON, write to disk — no imports, no setup, just script
@res  = http.get(url: "https://api.example.com/data")
@data = json.parse(@res["response_body"])
fs.write("/tmp/output.json", json.stringify(@data))
sys.echo("Written ${map.len(@data)} entries")
```

---

## Three ideas that set Corvo apart

### 1 · Zero dependencies. Zero supply-chain risk. One file.

Most scripting ecosystems hand you a package manager on day one. Corvo does the opposite: there is no `import`, no `require`, no `pip install`, no `npm install`, no lock file, no vulnerability scanner to run before you can ship.

Every capability — HTTP, JSON, YAML, CSV, XML, HCL, cryptography, DNS, SSH, rsync, LLM prompts, filesystem, subprocesses — lives in the standard library that ships with the interpreter. A Corvo script is always a single `.corvo` file with zero external dependencies. There is no supply chain to attack.

| Module | What you get |
|---|---|
| `http` | GET, POST, PUT, DELETE |
| `fs` | read, write, append, delete, copy, move, stat, list_dir |
| `json` / `yaml` / `csv` / `xml` / `hcl` | parse + stringify |
| `crypto` | sha256/md5/sha512 hashing, AES-GCM encrypt/decrypt, UUID |
| `dns` | resolve, reverse lookup |
| `ssh` | remote exec, SCP upload/download |
| `rsync` | directory sync |
| `llm` | prompt, embed, chat against any model/provider |
| `os` | env vars, OS info, shell exec |
| `sys` | echo, read_line, sleep, subprocess, exit |
| `math` | add, sub, mul, div, mod, max, human_bytes |
| `time` | unix_now, format_local |
| `string` | concat, split, replace, trim, upper, lower, contains, … |
| `list` | push, pop, get, set, join, sort, filter, map, … |
| `map` | get, set, keys, values, merge, remove, … |
| `args` | GNU-style and dig-style argument parser |

### 2 · No if/else. No function declarations. All in one go. Any small model can code it.

Corvo is a procedural language in the truest sense: execution starts at line one and ends at the last line. There is no `def`, no `fn`, no `function`, no recursion, no hidden call stack. The control flow you see is the control flow you get.

Branching is handled by two constructs:

**`try / fallback`** — for error-driven and assertion-driven branching. Assertions fire the fallback on failure:

```corvo
@port = 8080

try {
    assert_eq(os.get_env("ENV"), "production")
    @host = "api.prod.example.com"
} fallback {
    @host = "localhost"
}

sys.echo("Connecting to ${@host}:${@port}")
```

Chain as many fallbacks as you need:

```corvo
try {
    @config = fs.read("/etc/app/config.json")
} fallback {
    @config = fs.read("~/.config/app/config.json")
} fallback {
    @config = json.stringify({"mode": "default"})
}
```

**`match`** — for clean value-based switching without chains of `if / else if`:

```corvo
@label = match(os.get_env("ENV", "dev")) {
    "prod"    => "Production",
    "staging" => "Staging",
    _         => "Development"
}
sys.echo("Environment: ${@label}")
```

Reusable logic lives in **procedures** — blocks stored in variables and invoked via `.call()`. No global namespace pollution, no recursion, no surprises:

```corvo
@greet = procedure(@name, @msg) {
    @msg = "Hello, ${@name}!"
}

@out = ""
@greet.call("World", @out)
sys.echo(@out)   # Hello, World!
```

### 3 · Real async, backed by Rust

`async_browse` spawns one OS thread per list item and runs a procedure on each concurrently. Shared accumulator variables are mutex-protected; list appends use an atomic delta-merge so every thread's contributions land without data races.

```corvo
@urls = [
    "https://api.example.com/users",
    "https://api.example.com/orders",
    "https://api.example.com/products"
]
@results = list.new()

@fetch = procedure(@url, @acc) {
    @res = http.get(url: @url)
    @acc = list.push(@acc, @res["status_code"])
}

async_browse(@urls, @fetch, @url, shared @results)

sys.echo("Status codes: ${@results}")
```

All threads complete before execution continues — no callbacks, no `await`, no event loop to reason about. The parallelism is structural, not syntactic noise.

---

## Compile to a standalone binary

Corvo scripts can be compiled into self-contained executables with no runtime dependency:

```bash
corvo --compile deploy.corvo --output deploy
./deploy   # runs anywhere, no interpreter needed
```

Use a `prep` block to bake compile-time constants — including remote API responses, environment variables, or config files — directly into the binary:

```corvo
prep {
    static.set("VERSION", os.get_env("APP_VERSION", "1.0.0"))
    static.set("API_BASE", os.get_env("API_BASE", "https://api.example.com"))

    # Feature flags fetched once at compile time, encrypted into the binary
    @ff = http.get(string.concat(static.get("API_BASE"), "/feature-flags"))
    try {
        assert_eq(@ff["status_code"], 200)
        static.set("FLAGS", json.parse(@ff["response_body"]))
    } fallback {
        static.set("FLAGS", {"dark_mode": false, "beta_api": false})
    }
}

sys.echo("v${static.get("VERSION")} — ${static.get("API_BASE")}")
```

Static values are XOR-encrypted and stored as raw byte arrays in the compiled binary — they never appear as readable strings.

---

## Quick start

```bash
git clone https://github.com/KeanuReadmes/corvo-lang
cd corvo-lang
cargo build --release
# Binary is at target/release/corvo
```

```bash
corvo -e 'sys.echo("Hello, World!")'   # one-liner
corvo script.corvo                      # run a file
corvo --repl                            # interactive REPL
corvo --lint script.corvo               # lint without running
corvo --compile script.corvo -o out     # compile to standalone binary
corvo --transpile script.corvo -o out   # transpile to Rust project directory
```

---

## Language guide

### Variables and shorthand

Two namespaces, no assignment operator:

```corvo
# Runtime variable — long form
var.set("counter", 0)
var.set("counter", math.add(var.get("counter"), 1))

# Runtime variable — shorthand (@name = value, @name reads the value)
@counter = 0
@counter++
@counter += 5
sys.echo(@counter)   # 6

# Compile-time constant (inside prep only)
prep {
    static.set("HOST", os.get_env("HOST", "localhost"))
}
sys.echo(static.get("HOST"))
```

| Syntax | Meaning |
|---|---|
| `@name = val` | `var.set("name", val)` |
| `@name` | `var.get("name")` |
| `@name++` / `@name--` | increment / decrement by 1 |
| `@name += x` | add number or append string |
| `@name -= x` | subtract number or remove string occurrences |

### Types

Six types, inferred on first assignment and fixed thereafter:

```corvo
@name   = "Alice"              # string
@age    = 30                   # number
@active = true                 # boolean
@tags   = ["dev", "ops"]       # list
@meta   = {"region": "us-east"} # map
```

### String interpolation

Any expression inside `${...}`:

```corvo
@user  = "Alice"
@items = [10, 20, 30]
sys.echo("Hello ${@user}, you have ${list.len(@items)} items")
sys.echo("SHA-256: ${crypto.hash("sha256", @user)}")
```

### Loops

`loop` runs forever; `terminate` exits:

```corvo
@i   = 0
@sum = 0

loop {
    @sum += @i
    @i++
    try {
        assert_gt(@i, 10)
        terminate
    } fallback {}
}

sys.echo("Sum 1..10 = ${@sum}")
```

### Browse (iteration)

Iterate lists or maps with bound key/value variables (accessed with `@`):

```corvo
@scores = {"alice": 95, "bob": 87, "carol": 92}

browse(@scores, @name, @score) {
    sys.echo("${@name}: ${@score}")
}
```

```corvo
@files = fs.list_dir("/var/log")

browse(@files, @idx, @fname) {
    try {
        assert_match("\.log$", @fname)
        sys.echo("Log file: ${@fname}")
    } fallback {}
}
```

### Assertions

Used inside `try` blocks to trigger the `fallback`:

| Assertion | Condition |
|---|---|
| `assert_eq(a, b)` | `a == b` |
| `assert_neq(a, b)` | `a != b` |
| `assert_gt(a, b)` | `a > b` |
| `assert_lt(a, b)` | `a < b` |
| `assert_match(regex, s)` | `s` matches regex |

### Subprocess execution

No shell injection: pass a list of strings, get `{stdout, stderr, code}` back:

```corvo
@result = sys.exec(["git", "log", "--oneline", "-5"])
sys.echo(@result["stdout"])

# Timeout, working directory, env vars
sys.exec(["make", "build"], cwd: "/src", env: {"CC": "clang"}, timeout: 60, check: true)
```

---

## Example: parallel URL health checker

```corvo
@endpoints = [
    "https://api.example.com/health",
    "https://api.example.com/ready",
    "https://api.example.com/metrics"
]
@statuses = list.new()

@check = procedure(@url, @out) {
    try {
        @res = http.get(url: @url)
        @line = string.concat(@url, string.concat(" → ", number.to_string(@res["status_code"])))
        @out = list.push(@out, @line)
    } fallback {
        @out = list.push(@out, string.concat(@url, " → ERROR"))
    }
}

async_browse(@endpoints, @check, @url, shared @statuses)

browse(@statuses, @i, @line) {
    sys.echo(@line)
}
```

## Example: deploy script compiled to a binary

```corvo
prep {
    static.set("ENV",  os.get_env("DEPLOY_ENV",  "staging"))
    static.set("DEST", os.get_env("DEPLOY_DEST", "server:/app"))
    static.set("HOOK", os.get_env("SLACK_HOOK",  ""))
}

sys.echo("Deploying to ${static.get("ENV")}...")

try {
    sys.exec(
        ["rsync", "-avz", "--delete", "./build/", static.get("DEST")],
        timeout: 300,
        check: true
    )
    sys.echo("Deploy complete.")
} fallback {
    sys.echo("Deploy failed — sending alert...")
    try {
        assert_neq(static.get("HOOK"), "")
        http.post(
            url: static.get("HOOK"),
            body: json.stringify({"text": "Deploy to ${static.get("ENV")} failed!"})
        )
    } fallback {}
}
```

## Example: grep errors from a log file

```corvo
@lines  = string.split(fs.read("access.log"), "\n")
@errors = list.new()

browse(@lines, @_, @line) {
    try {
        assert_match("ERROR", @line)
        @errors = list.push(@errors, @line)
    } fallback {}
}

sys.echo("${list.len(@errors)} error lines found")
```

---

## CLI reference

| Flag | Description |
|---|---|
| `corvo <file>` | Run a script |
| `corvo -e '<expr>'` | Evaluate an inline expression |
| `corvo --repl` | Interactive REPL |
| `corvo --lint <file>` | Lint without executing |
| `corvo --compile <file> -o <out>` | Compile to a standalone binary (interpreter embedded) |
| `corvo --transpile <file> -o <out>` | Transpile to a native Rust project directory |
| `corvo --debug` | Use a debug build (faster compile time) |
| `corvo --version` | Print version |

---

## Error codes

Every error carries a source location and a unique exit code:

| Code | Error | When |
|---|---|---|
| 1 | Lexing | Invalid token |
| 2 | Parsing | Syntax error |
| 3 | Type | Type mismatch |
| 4 | Runtime | General runtime failure |
| 5 | Assertion | `assert_*` outside a try block |
| 6 | FileSystem | File I/O failure |
| 7 | Network | HTTP / DNS failure |
| 8 | UnknownFunction | Undefined function called |
| 9 | StaticModification | `static.set` outside `prep` |
| 10 | DivisionByZero | Division by zero |
| 11 | VariableNotFound | Undefined variable |
| 12 | StaticNotFound | Undefined static |
| 13 | Io | General I/O error |
| 14 | InvalidArgument | Bad function argument |

---

## Install

```bash
git clone https://github.com/KeanuReadmes/corvo-lang
cd corvo-lang
cargo build --release
sudo cp target/release/corvo /usr/local/bin/
```

Or use the install script:

```bash
curl -fsSL https://raw.githubusercontent.com/KeanuReadmes/corvo-lang/main/install.sh | bash
```

---

## Project layout

```
src/
  lexer/           Tokenizer
  parser/          Recursive-descent parser
  ast/             AST node definitions
  type_system/     Value enum + method dispatch
  standard_lib/    All built-in modules
  compiler/        Evaluator + standalone binary builder
  runtime/         Variable and static state
  diagnostic.rs    Linter
  main.rs          CLI entry point
examples/          One .corvo file per feature
coreutils/         Full GNU coreutils re-implementations (ls, …)
tests/             Integration test suite (60+ tests)
```

---

## License

MIT
