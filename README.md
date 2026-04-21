# Corvo

![corvo-logo](./corvo.jpg)

**Write scripts like prose. Ship them like binaries. Trust them like Rust.**

Corvo is a modern scripting language that compiles to standalone Rust binaries. It is deliberately stripped of the things that make scripting languages fragile — no package manager, no dependency graph, no `import` statement, no complex function signatures to maintain. What remains is a language that is easy to read, audit, and ship anywhere.

```corvo
# Fetch an API, parse JSON, write to disk — no imports, no setup, just script
if (fs.exists("/tmp/output.json")) {
    sys.echo("Output file already exists, skipping fetch.")
} else {
    @res  = http.get(url: "https://api.example.com/data")
    @data = json.parse(@res["response_body"])
    fs.write("/tmp/output.json", json.stringify(@data))
    sys.echo("Written ${map.len(@data)} entries")
}
```

---

## Three ideas that set Corvo apart

### 1 · Zero dependencies. Zero supply-chain risk. One file.

Most scripting ecosystems hand you a package manager on day one. Corvo does the opposite: there is no `import`, no `require`, no `pip install`, no `npm install`, no lock file, no vulnerability scanner to run before you can ship.

Every capability — HTTP, JSON, YAML, CSV, XML, HCL, cryptography, DNS, SSH, rsync, LLM prompts, filesystem, subprocesses — lives in the standard library that ships with the interpreter. A Corvo script is always a single `.corvo` file with zero external dependencies. There is no supply chain to attack.

| Module | What you get |
|---|---|
| `http` | GET, POST, PUT, DELETE |
| `fs` | read/write, stat, chmod/chown, find, mktemp, realpath, path utilities |
| `json` / `yaml` / `csv` / `xml` / `hcl` | parse + stringify |
| `crypto` | sha256/md5/sha512, crc32, AES-GCM encryption, UUID |
| `os` | uptime, load_average, users, env vars, OS info, tty mode |
| `sys` | echo, print, read_all, sleep, subprocess (exec), nice, sync |
| `math` | human_bytes, parse_size, range, max/min, random, floor/ceil/round |
| `time` | unix_now, format_local/utc, parse_date, boot_time |
| `string` | concat, split, replace, substring, index_of, base64/hex/base32 |
| `list` | push, pop, slice, sort_version, sort_maps_by_key, columnate |
| `map` | get, set, keys, values, merge, remove |
| `args` | GNU-style and dig-style argument parser |

### 2 · Explicit Control Flow. No hidden call stack.

Corvo is a procedural language in the truest sense: execution starts at line one and ends at the last line. There is no `def`, no `fn`, no `function`, no recursion. The control flow you see is exactly what happens.

Branching is handled by three clean constructs:

**`if / else`** — for deterministic switching and Boolean logic.

```corvo
if (os.get_env("ENV") == "production") {
    @host = "api.prod.example.com"
} else {
    @host = "localhost"
}
```

**`match`** — for clean value-based switching without chains of `else if`:

```corvo
@label = match(os.get_env("ENV", "dev")) {
    "prod"    => "Production",
    "staging" => "Staging",
    _         => "Development"
}
```

**`try / fallback`** — for error-driven and assertion-driven branching. Assertions fire the fallback on failure, making error handling structural rather than nested:

```corvo
try {
    @config = fs.read("/etc/app/config.json")
} fallback {
    @config = fs.read("~/.config/app/config.json")
} fallback {
    @config = json.stringify({"mode": "default"})
}
```

Reusable logic lives in **procedures** — blocks stored in variables and invoked via `.call()`. No global namespace pollution, no recursion, no surprises.

### 3 · Real async, backed by Rust

`async_browse` spawns one OS thread per list item and runs a procedure on each concurrently. Shared accumulator variables are mutex-protected; list appends use an atomic delta-merge.

```corvo
@urls = ["https://api.a.com", "https://api.b.com"]
@results = list.new()

@fetch = procedure(@url, @acc) {
    @res = http.get(url: @url)
    @acc = list.push(@acc, @res["status_code"])
}

async_browse(@urls, @fetch, @url, shared @results)
sys.echo("Status codes: ${@results}")
```

---

## Compile to a standalone binary

Corvo scripts can be compiled into self-contained executables with no runtime dependency:

```bash
corvo --compile deploy.corvo --output deploy
./deploy   # runs anywhere, no interpreter needed
```

Use a `prep` block to bake compile-time constants (encrypted in the binary):

```corvo
prep {
    static.set("API_KEY", os.get_env("PROD_API_KEY"))
    static.set("BUILD_DATE", time.format_local(time.unix_now(), 0, "%Y-%m-%d"))
}
sys.echo("Compiled on ${static.get("BUILD_DATE")}")
```

---

## Quick start

```bash
git clone https://github.com/KeanuReadmes/corvo-lang
cd corvo-lang
cargo build --release
```

```bash
corvo script.corvo                      # run a file
corvo --lint script.corvo               # lint without running
corvo --compile script.corvo -o out     # compile to binary
```

---

## Language guide

### Variables and shorthand
- `@name = val` sets a variable.
- `@name` gets its value.
- `@name++`, `@name--`, `@name += x`, `@name -= x` are supported.

### Types
String, Number, Boolean, List, Map, Null.

### Loops and Iteration
- `loop { ... }` with `terminate` for exit.
- `browse(list, @idx, @val) { ... }` for iteration.
- `async_browse(...)` for concurrent iteration.

### Assertions
Used inside `try` blocks to trigger `fallback`:
`assert_eq`, `assert_neq`, `assert_gt`, `assert_lt`, `assert_match`.

---

## Example: coreutils re-implemented in Corvo

The `coreutils/` directory contains high-performance, GNU-compatible re-implementations of standard tools like `ls`, `cp`, `rm`, `cat`, `head`, `tail`, `wc`, and more — all written in Corvo, passing a 270+ case parity test suite against the GNU originals.

---

## Install

```bash
git clone https://github.com/KeanuReadmes/corvo-lang
cd corvo-lang
cargo build --release
sudo cp target/release/corvo /usr/local/bin/
```

## License
MIT
