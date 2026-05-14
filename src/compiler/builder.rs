use crate::lexer::token::{Token, TokenType};
use crate::runtime::RuntimeState;
use crate::type_system::Value;
use crate::CorvoError;
use directories::ProjectDirs;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// Environment variable that overrides the persistent build cache location.
/// Useful for CI runners and sandboxed environments where the default user
/// cache directory is unavailable or inappropriate.
const CACHE_DIR_ENV: &str = "CORVO_CACHE_DIR";

/// When set to the absolute filesystem path of the `corvo-lang` crate root,
/// `compile` appends a `[patch.crates-io]` table to the generated `Cargo.toml`
/// so `cargo build` links against that tree instead of crates.io.  Useful for
/// local development and benchmarks where the workspace version is not
/// published.
const CORVO_LANG_LOCAL_PATH_ENV: &str = "CORVO_LANG_LOCAL_PATH";

/// Binary name for the artifact produced by [`Compiler::compile`] (`cargo` package mode).
///
/// This must stay in sync everywhere compile-mode `[[bin]]` entries and `run_cargo_build`
/// locate the executable.
pub const COMPILE_MODE_BIN_NAME: &str = "corvo_compiled";

static CORVO_CACHE_ENV_TEST_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

/// Acquire a process-wide lock for tests that mutate `CORVO_CACHE_DIR` or related env vars.
///
/// Unit tests in this crate and the `benchmark_compile` integration test share the same
/// process; parallel `set_var` / `remove_var` would race without serialization.
pub fn corvo_cache_dir_test_lock() -> std::sync::MutexGuard<'static, ()> {
    CORVO_CACHE_ENV_TEST_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|p| p.into_inner())
}

/// True if `Cargo.toml` already defines a `[[bin]]` stanza with `name = "{name}"`.
///
/// We only scan text inside `[[bin]]` sections so we do not mistake `[package] name = "..."`
/// for a binary entry when the package name equals the transpile binary stem.
fn cargo_toml_has_bin_target_named(content: &str, name: &str) -> bool {
    let needle = format!("name = \"{name}\"");
    for section in content.split("[[bin]]").skip(1) {
        let head = section.split("[[").next().unwrap_or(section);
        if head.contains(&needle) {
            return true;
        }
    }
    false
}

/// Oxide `Cargo.toml` dependency line for `corvo-lang` with optional feature flags.
fn oxide_corvo_lang_dependency_line(features: &[String]) -> String {
    let features_str = if features.is_empty() {
        String::new()
    } else {
        format!(
            ", features = [{}]",
            features
                .iter()
                .map(|f| format!("\"{}\"", f))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    format!(
        "corvo-lang = {{ version = \"*\", default-features = false{} }}",
        features_str
    )
}

/// Parse `features = [ ... ]` from an oxide-style `corvo-lang = { ... }` dependency line.
fn parse_oxide_features_from_cargo_toml(content: &str) -> Vec<String> {
    const PREFIX: &str = "corvo-lang = { version = \"*\", default-features = false";
    let Some(idx) = content.find(PREFIX) else {
        return Vec::new();
    };
    let mut rest = content[idx + PREFIX.len()..].trim_start();
    if rest.starts_with('}') {
        return Vec::new();
    }
    const FEAT: &str = ", features = [";
    if !rest.starts_with(FEAT) {
        return Vec::new();
    }
    rest = &rest[FEAT.len()..];
    let Some(end) = rest.find(']') else {
        return Vec::new();
    };
    let inner = &rest[..end];
    inner
        .split(',')
        .filter_map(|s| {
            let t = s.trim();
            if t.len() >= 2 && t.starts_with('"') && t.ends_with('"') {
                Some(t[1..t.len() - 1].to_string())
            } else {
                None
            }
        })
        .collect()
}

fn merge_oxide_feature_lists(existing: &[String], added: &[String]) -> Vec<String> {
    use std::collections::BTreeSet;
    let mut set: BTreeSet<String> = BTreeSet::new();
    for x in existing {
        set.insert(x.clone());
    }
    for x in added {
        set.insert(x.clone());
    }
    set.into_iter().collect()
}

/// Replace the oxide `corvo-lang` dependency line, preserving the rest of `Cargo.toml`.
fn replace_oxide_corvo_lang_line(content: &str, merged_features: &[String]) -> Result<String, CorvoError> {
    const PREFIX: &str = "corvo-lang = { version = \"*\", default-features = false";
    let Some(idx) = content.find(PREFIX) else {
        return Err(CorvoError::runtime(
            "oxide merge: Cargo.toml missing oxide-style corvo-lang dependency line".to_string(),
        ));
    };
    let line_start = content[..idx].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line_end = content[line_start..]
        .find('\n')
        .map(|i| line_start + i + 1)
        .unwrap_or(content.len());
    let new_line = oxide_corvo_lang_dependency_line(merged_features);
    Ok(format!(
        "{}{}{}",
        &content[..line_start],
        new_line,
        &content[line_end..]
    ))
}

/// Cargo `[package] name` must satisfy identifier rules (Unicode XID). Tempdirs from
/// `tempfile` often look like `.tmpXXXXXX`, whose leading `.` breaks `cargo build`.
fn transpile_package_name_from_build_dir(build_dir: &Path) -> String {
    let raw = build_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("corvo_project");
    sanitize_cargo_package_name(raw)
}

fn sanitize_cargo_package_name(raw: &str) -> String {
    let mapped: String = raw
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = mapped.trim_matches(|c| c == '_' || c == '-');
    if trimmed.is_empty() {
        return "corvo_project".to_string();
    }
    let mut chars = trimmed.chars();
    let first = chars.next().unwrap();
    if first.is_numeric() {
        format!("pkg_{trimmed}")
    } else {
        trimmed.to_string()
    }
}

pub struct Compiler {
    source: String,
    /// Source with the prep block stripped out. Used when embedding source into
    /// the compiled binary: the prep block is already evaluated at compile time
    /// and its static values are baked in, so there is no need to re-run it at
    /// runtime (doing so would re-execute side effects like `fs.read` even
    /// after the file has been removed).
    source_without_prep: String,
    _source_path: PathBuf,
    build_mode: BuildMode,
    statics: HashMap<String, Value>,
    no_debug: bool,
}

pub enum BuildMode {
    Debug,
    Release,
}

impl Compiler {
    pub fn new(source: String, source_path: PathBuf) -> Self {
        // source_without_prep starts as a copy of the full source and is
        // replaced with the stripped version once pre_execute() runs.
        let source_without_prep = source.clone();
        Self {
            source,
            source_without_prep,
            _source_path: source_path,
            build_mode: BuildMode::Release,
            statics: HashMap::new(),
            no_debug: false,
        }
    }

    pub fn with_debug(mut self) -> Self {
        self.build_mode = BuildMode::Debug;
        self
    }

    /// Enable anti-debugging protection in the compiled binary.
    ///
    /// When set, the generated binary will refuse to run under debuggers
    /// (gdb, LLDB), tracers (strace), record-and-replay tools (rr), dynamic
    /// analysis tools (Valgrind), and Windows debuggers (WinDbg).
    pub fn with_no_debug(mut self) -> Self {
        self.no_debug = true;
        self
    }

    /// Pre-executes the script to capture static variable values.
    /// This is the key step: static.set("key", os.get_env("VAR")) runs at
    /// compile time, and the resulting value is baked into the binary.
    ///
    /// This method also strips the prep block from `source_without_prep` so
    /// that the compiled binary does not embed the prep block source code.
    /// Because the static values are already baked in, there is no need to
    /// re-run the prep block at runtime, and doing so could trigger side
    /// effects (e.g. `fs.read`) that fail if the referenced files no longer
    /// exist.
    pub fn pre_execute(&mut self) -> Result<(), CorvoError> {
        let mut lexer = crate::lexer::Lexer::new(&self.source);
        let tokens = lexer.tokenize()?;

        // Strip the prep block from the source that will be embedded in the
        // binary. The statics are baked in separately, so the prep block must
        // not run again at runtime.
        self.source_without_prep = strip_prep_block(&self.source, &tokens);

        let mut parser = crate::parser::Parser::new(tokens);
        let program = parser.parse()?;

        let mut state = RuntimeState::new();
        let mut evaluator = crate::compiler::Evaluator::new().with_pre_exec_mode(true);
        // Run the script. Runtime errors are OK - we just want static values.
        let _ = evaluator.run(&program, &mut state);

        self.statics = state.statics_snapshot();
        Ok(())
    }

    pub fn static_count(&self) -> usize {
        self.statics.len()
    }

    pub fn compile(&self, output: &Path) -> Result<PathBuf, CorvoError> {
        let build_dir = create_build_dir()?;

        self.generate_cargo_toml(&build_dir, COMPILE_MODE_BIN_NAME)?;
        if let Ok(root) = std::env::var(CORVO_LANG_LOCAL_PATH_ENV) {
            let canon = resolve_corvo_lang_local_path(&root)?;
            append_corvo_lang_patch_to_cargo_toml(&build_dir.join("Cargo.toml"), &canon)?;
        }
        self.generate_main_rs(&build_dir)?;

        let binary = self.run_cargo_build(&build_dir)?;
        let final_binary = copy_binary(&binary, output)?;

        Ok(final_binary)
    }

    pub fn transpile(&self, output_dir: &Path) -> Result<(), CorvoError> {
        let bin_name = self
            ._source_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("main");

        std::fs::create_dir_all(output_dir)
            .map_err(|e| CorvoError::io(format!("Failed to create output directory: {}", e)))?;

        self.generate_cargo_toml(output_dir, bin_name)?;

        // Use Transpiler to generate main.rs
        let mut lexer = crate::lexer::Lexer::new(&self.source_without_prep);
        let tokens = lexer.tokenize()?;
        let mut parser = crate::parser::Parser::new(tokens);
        let program = parser.parse()?;

        let transpiler =
            crate::compiler::transpiler::Transpiler::new().with_statics(self.statics.clone());
        let rust_code = transpiler.transpile(&program);

        let src_dir = output_dir.join("src");
        std::fs::create_dir_all(&src_dir)
            .map_err(|e| CorvoError::io(format!("Failed to create src dir: {}", e)))?;

        let output_file = src_dir.join(format!("{}.rs", bin_name));
        std::fs::write(&output_file, rust_code)
            .map_err(|e| CorvoError::io(format!("Failed to write {}.rs: {}", bin_name, e)))?;

        Ok(())
    }

    /// Oxide transpile: generate a lean Rust project with direct stdlib calls
    /// and only the Cargo features actually needed by the script.
    pub fn oxide(&self, output_dir: &Path) -> Result<(), CorvoError> {
        let bin_name = self
            ._source_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("main");

        std::fs::create_dir_all(output_dir)
            .map_err(|e| CorvoError::io(format!("Failed to create output directory: {}", e)))?;

        // Parse AST
        let mut lexer = crate::lexer::Lexer::new(&self.source_without_prep);
        let tokens = lexer.tokenize()?;
        let mut parser = crate::parser::Parser::new(tokens);
        let program = parser.parse()?;

        // Analyze usage
        let usage = crate::compiler::usage_analyzer::UsageAnalysis::from_program(&program);
        let features = usage.required_features();

        // Generate Cargo.toml with minimal features
        self.generate_oxide_cargo_toml(output_dir, bin_name, &features)?;

        if let Ok(root) = std::env::var(CORVO_LANG_LOCAL_PATH_ENV) {
            let canon = resolve_corvo_lang_local_path(&root)?;
            append_corvo_lang_patch_to_cargo_toml(&output_dir.join("Cargo.toml"), &canon)?;
        }

        // Generate Rust source via OxideTranspiler
        let oxide = crate::compiler::oxide_transpiler::OxideTranspiler::new(usage)
            .with_statics(self.statics.clone());
        let rust_code = oxide.transpile(&program);

        let src_dir = output_dir.join("src");
        std::fs::create_dir_all(&src_dir)
            .map_err(|e| CorvoError::io(format!("Failed to create src dir: {}", e)))?;

        let output_file = src_dir.join(format!("{}.rs", bin_name));
        std::fs::write(&output_file, rust_code)
            .map_err(|e| CorvoError::io(format!("Failed to write {}.rs: {}", bin_name, e)))?;

        Ok(())
    }

    fn generate_oxide_cargo_toml(
        &self,
        build_dir: &Path,
        bin_name: &str,
        features: &[String],
    ) -> Result<(), CorvoError> {
        let cargo_toml_path = build_dir.join("Cargo.toml");
        let package_name = transpile_package_name_from_build_dir(build_dir);

        if !cargo_toml_path.exists() {
            let features_str = if features.is_empty() {
                String::new()
            } else {
                format!(
                    ", features = [{}]",
                    features
                        .iter()
                        .map(|f| format!("\"{}\"", f))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };

            let content = format!(
                "[package]\n\
                 name = \"{}\"\n\
                 version = \"0.1.0\"\n\
                 edition = \"2021\"\n\
                 \n\
                 [dependencies]\n\
                 corvo-lang = {{ version = \"*\", default-features = false{} }}\n\
                 regex = \"1.10\"\n\
                 serde_json = \"1.0\"\n\
                 \n\
                 [profile.release]\n\
                 opt-level = \"z\"\n\
                 lto = true\n\
                 codegen-units = 1\n\
                 strip = true\n\
                 \n\
                 [[bin]]\n\
                 name = \"{}\"\n\
                 path = \"src/{}.rs\"\n",
                package_name, features_str, bin_name, bin_name,
            );

            std::fs::write(&cargo_toml_path, content)
                .map_err(|e| CorvoError::io(format!("Failed to write Cargo.toml: {}", e)))?;
            return Ok(());
        }

        // Subsequent oxide transpiles into the same directory: merge `corvo-lang` features and append [[bin]].
        let mut content = std::fs::read_to_string(&cargo_toml_path)
            .map_err(|e| CorvoError::io(format!("Failed to read Cargo.toml: {}", e)))?;

        let parsed = parse_oxide_features_from_cargo_toml(&content);
        let merged = merge_oxide_feature_lists(&parsed, features);
        content = replace_oxide_corvo_lang_line(&content, &merged)?;

        let bin_entry = format!(
            "\n[[bin]]\nname = \"{}\"\npath = \"src/{}.rs\"\n",
            bin_name, bin_name
        );
        if !cargo_toml_has_bin_target_named(&content, bin_name) {
            content.push_str(&bin_entry);
        }

        std::fs::write(&cargo_toml_path, content)
            .map_err(|e| CorvoError::io(format!("Failed to write Cargo.toml: {}", e)))?;

        Ok(())
    }
    fn generate_cargo_toml(&self, build_dir: &Path, bin_name: &str) -> Result<(), CorvoError> {
        let cargo_toml_path = build_dir.join("Cargo.toml");

        // Compile mode reuses a persistent, agent-managed build directory, so
        // the file is rewritten from scratch each time: this prevents stale
        // `[[bin]]` entries from accumulating across runs and keeps the
        // dependency set consistent with the running CLI version. Transpile
        // mode targets a user-owned project directory and preserves any
        // customisations the user made by reading + appending instead.
        let is_compile_mode = bin_name == COMPILE_MODE_BIN_NAME;
        let fresh = is_compile_mode || !cargo_toml_path.exists();

        let mut content = if fresh {
            // Compile mode uses a fixed package name so cache paths can contain dots or
            // other characters Cargo rejects. Transpile mode derives a name from the output
            // directory basename but sanitizes it (e.g. tempfile `.tmpXXXXXX`).
            let package_name = if is_compile_mode {
                "corvo_compile_workspace".to_string()
            } else {
                transpile_package_name_from_build_dir(build_dir)
            };

            // The `corvo-lang` requirement is left as a wildcard so the same
            // generated project works for any CLI version (including unreleased
            // dev builds whose version is not on crates.io). Reproducibility
            // across compiles still holds because the persistent build dir
            // preserves `Cargo.lock` between runs.
            format!(
                "[package]\n\
                 name = \"{}\"\n\
                 version = \"0.1.0\"\n\
                 edition = \"2021\"\n\
                 \n\
                 [dependencies]\n\
                 corvo-lang = \"*\"\n\
                 regex = \"1.10\"\n\
                 serde_json = \"1.0\"\n\
                 \n\
                 [profile.release]\n\
                 opt-level = 2\n\
                 lto = false\n",
                package_name,
            )
        } else {
            std::fs::read_to_string(&cargo_toml_path)
                .map_err(|e| CorvoError::io(format!("Failed to read Cargo.toml: {}", e)))?
        };

        // Add libc if needed and not already present
        if self.no_debug && !content.contains("libc =") {
            if let Some(pos) = content.find("[dependencies]") {
                let rest = &content[pos..];
                if let Some(newline_pos) = rest.find('\n') {
                    content.insert_str(pos + newline_pos + 1, "libc = \"0.2\"\n");
                }
            }
        }

        // Binary path: for compile mode we use "src/main.rs"; for transpile, `src/{bin}.rs`.
        let bin_path = if is_compile_mode {
            "src/main.rs".to_string()
        } else {
            format!("src/{}.rs", bin_name)
        };

        let bin_entry = format!(
            "\n[[bin]]\nname = \"{}\"\npath = \"{}\"\n",
            bin_name, bin_path
        );

        // Check if this binary target already exists (transpile mode only merges;
        // do not match `[package] name = "same_as_stem"`.)
        if !cargo_toml_has_bin_target_named(&content, bin_name) {
            content.push_str(&bin_entry);
        }

        std::fs::write(&cargo_toml_path, content)
            .map_err(|e| CorvoError::io(format!("Failed to write Cargo.toml: {}", e)))?;

        Ok(())
    }

    fn generate_main_rs(&self, build_dir: &Path) -> Result<(), CorvoError> {
        let src_dir = build_dir.join("src");
        std::fs::create_dir_all(&src_dir)
            .map_err(|e| CorvoError::io(format!("Failed to create src dir: {}", e)))?;

        // Obfuscate the Corvo source so it does not appear in `strings` output.
        // A random 32-byte key is generated per compilation and XOR-encrypted
        // with the source bytes.  The encrypted bytes and the key are both
        // stored as raw integer arrays (not string literals) in the binary, so
        // no readable source text survives in the compiled output.
        let key = generate_obfuscation_key();
        let encrypted = xor_encrypt(self.source_without_prep.as_bytes(), &key);

        let encrypted_literal = bytes_to_rust_array(&encrypted);
        let key_literal = bytes_to_rust_array(&key);

        let mut main_rs = String::new();

        // Inject the anti-debugging helper before main() so it is available
        // when called as the very first statement.
        if self.no_debug {
            main_rs.push_str(&generate_anti_debug_fn());
        }

        main_rs.push_str("fn main() {\n");

        // Abort immediately if a debugger / tracer is detected.
        if self.no_debug {
            main_rs.push_str("    abort_if_debugged();\n");
        }

        // Serialize and encrypt the statics so that static keys and string
        // values do not appear as readable strings in the compiled binary.
        // A fresh random key is generated for the statics independently of
        // the source-obfuscation key so the two byte arrays look unrelated.
        main_rs.push_str("    let mut state = corvo_lang::RuntimeState::new();\n");
        let statics_json = statics_to_json_bytes(&self.statics);
        let statics_key = generate_obfuscation_key();
        let encrypted_statics = xor_encrypt(&statics_json, &statics_key);
        let encrypted_statics_literal = bytes_to_rust_array(&encrypted_statics);
        let statics_key_literal = bytes_to_rust_array(&statics_key);
        main_rs.push_str(&format!(
            "    const ENCRYPTED_STATICS: &[u8] = &{};\n",
            encrypted_statics_literal
        ));
        main_rs.push_str(&format!(
            "    const STATICS_KEY: &[u8] = &{};\n",
            statics_key_literal
        ));
        main_rs.push_str(
            "    corvo_lang::load_statics_from_encrypted_bytes(&mut state, ENCRYPTED_STATICS, STATICS_KEY);\n",
        );
        main_rs.push_str("    state.set_script_argv(std::env::args().skip(1).collect());\n");

        // Embed the encrypted source and the key as raw byte arrays, then
        // decrypt at runtime before executing.  Neither the source nor the key
        // appears as a human-readable string in the compiled binary.
        main_rs.push_str(&format!(
            "    const ENCRYPTED_SOURCE: &[u8] = &{};\n",
            encrypted_literal
        ));
        main_rs.push_str(&format!(
            "    const OBFUSCATION_KEY: &[u8] = &{};\n",
            key_literal
        ));
        main_rs.push_str("    let decrypted: Vec<u8> = ENCRYPTED_SOURCE.iter().enumerate()\n");
        main_rs
            .push_str("        .map(|(i, &b)| b ^ OBFUSCATION_KEY[i % OBFUSCATION_KEY.len()])\n");
        main_rs.push_str("        .collect();\n");
        main_rs.push_str(
            "    let source = String::from_utf8(decrypted).expect(\"invalid UTF-8 in source\");\n",
        );
        main_rs.push_str("    match corvo_lang::run_source_with_state(&source, &mut state) {\n");
        main_rs.push_str("        Ok(_) => std::process::exit(0),\n");
        main_rs.push_str("        Err(e) => {\n");
        main_rs.push_str("            if let Some(code) = e.process_exit_code() {\n");
        main_rs.push_str("                std::process::exit(code);\n");
        main_rs.push_str("            }\n");
        main_rs.push_str("            eprintln!(\"{}\", e);\n");
        main_rs.push_str("            std::process::exit(e.exit_code());\n");
        main_rs.push_str("        }\n");
        main_rs.push_str("    }\n");
        main_rs.push_str("}\n");

        std::fs::write(src_dir.join("main.rs"), main_rs)
            .map_err(|e| CorvoError::io(format!("Failed to write main.rs: {}", e)))?;

        Ok(())
    }

    fn run_cargo_build(&self, build_dir: &Path) -> Result<PathBuf, CorvoError> {
        let profile = match self.build_mode {
            BuildMode::Debug => "debug",
            BuildMode::Release => "release",
        };

        let mut cmd = std::process::Command::new("cargo");
        cmd.arg("build");
        if matches!(self.build_mode, BuildMode::Release) {
            cmd.arg("--release");
        }
        // Nested `cargo` must use this project's `target/` directory. If the
        // parent process set `CARGO_TARGET_DIR` (e.g. IDE/sandbox wrappers),
        // inheriting it would send artifacts elsewhere and break binary lookup.
        cmd.env_remove("CARGO_TARGET_DIR");
        cmd.current_dir(build_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let output = cmd
            .output()
            .map_err(|e| CorvoError::io(format!("Failed to run cargo: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CorvoError::io(format!("Compilation failed:\n{}", stderr)));
        }

        let binary_name = if cfg!(target_os = "windows") {
            format!("{}.exe", COMPILE_MODE_BIN_NAME)
        } else {
            COMPILE_MODE_BIN_NAME.to_string()
        };

        let binary = build_dir.join("target").join(profile).join(binary_name);

        if !binary.exists() {
            return Err(CorvoError::io("Compiled binary not found".to_string()));
        }

        Ok(binary)
    }
}

/// Return the persistent compile cache directory.
///
/// Resolution order:
/// 1. `$CORVO_CACHE_DIR` if set to a **non-empty** value — must be an absolute path
///    and must not name only a drive/filesystem root (see safety checks in the
///    implementation). A blank or whitespace-only value is treated as unset.
/// 2. The user cache directory (e.g. `~/.cache/corvo/build` on Linux,
///    `~/Library/Caches/com.Corvo.corvo/build` on macOS) via the
///    `directories` crate.
/// 3. `$TMPDIR/corvo_build` as a last-resort fallback when no cache directory
///    can be determined.
///
/// This directory is reused across compilations so that `cargo`'s incremental
/// build cache survives between `corvo --compile` invocations. Use
/// [`clean_cache`] (or `corvo --clean`) to wipe it.
pub fn cache_dir() -> Result<PathBuf, CorvoError> {
    if let Some(custom) = cache_dir_env_override()? {
        return Ok(custom);
    }
    if let Some(proj) = ProjectDirs::from("com", "Corvo", "corvo") {
        return Ok(proj.cache_dir().join("build"));
    }
    Ok(std::env::temp_dir().join("corvo_build"))
}

fn cache_dir_env_override() -> Result<Option<PathBuf>, CorvoError> {
    let Some(raw_os) = std::env::var_os(CACHE_DIR_ENV) else {
        return Ok(None);
    };
    let raw = raw_os.to_string_lossy();
    if raw.trim().is_empty() {
        return Ok(None);
    }
    let path = PathBuf::from(raw.as_ref());
    if !path.is_absolute() {
        return Err(CorvoError::io(format!(
            "Environment variable {CACHE_DIR_ENV} must be an absolute path, got: {raw}",
        )));
    }
    if is_disallowed_cache_deletion_root(&path) {
        return Err(CorvoError::io(format!(
            "Environment variable {CACHE_DIR_ENV} must not point at a filesystem root ({})",
            path.display(),
        )));
    }
    Ok(Some(path))
}

/// Reject cache roots that would make `corvo --clean` or `remove_dir_all` catastrophic.
fn is_disallowed_cache_deletion_root(path: &Path) -> bool {
    use std::path::Component;
    let mut components = path.components();
    match components.next() {
        Some(Component::Prefix(_)) => {
            matches!(
                (components.next(), components.next()),
                (Some(Component::RootDir), None)
            )
        }
        Some(Component::RootDir) => components.next().is_none(),
        Some(Component::CurDir) | Some(Component::ParentDir) | Some(Component::Normal(_)) => false,
        None => true,
    }
}

/// Remove the persistent compile cache directory (returned by [`cache_dir`]).
///
/// Returns `Ok(true)` if the directory existed and was removed, `Ok(false)` if
/// nothing needed to be cleaned, and `Err(_)` if removal failed.
pub fn clean_cache() -> Result<bool, CorvoError> {
    let dir = cache_dir()?;
    if !dir.exists() {
        return Ok(false);
    }
    std::fs::remove_dir_all(&dir)
        .map_err(|e| CorvoError::io(format!("Failed to remove cache dir {:?}: {}", dir, e)))?;
    Ok(true)
}

/// Resolve and validate [`CORVO_LANG_LOCAL_PATH_ENV`]: must be absolute, exist, and be a directory.
fn resolve_corvo_lang_local_path(raw: &str) -> Result<PathBuf, CorvoError> {
    let path = PathBuf::from(raw);
    if !path.is_absolute() {
        return Err(CorvoError::io(format!(
            "Environment variable {CORVO_LANG_LOCAL_PATH_ENV} must be an absolute path, got: {raw}"
        )));
    }
    let canonical = path.canonicalize().map_err(|e| {
        CorvoError::io(format!(
            "Failed to canonicalize path from {CORVO_LANG_LOCAL_PATH_ENV}={raw}: {e}"
        ))
    })?;
    if !canonical.is_dir() {
        return Err(CorvoError::io(format!(
            "Environment variable {CORVO_LANG_LOCAL_PATH_ENV} must point to an existing directory: {}",
            canonical.display()
        )));
    }
    Ok(canonical)
}

/// Append a `[patch.crates-io]` table so `corvo-lang` resolves to `repo_root` (must exist).
///
/// Used by [`Compiler::compile`] when `CORVO_LANG_LOCAL_PATH` is set and by benchmark / test harnesses
/// patching generated projects. No-op if `[patch.crates-io]` is already present.
pub fn append_corvo_lang_patch_to_cargo_toml(
    manifest_path: &Path,
    repo_root: &Path,
) -> Result<(), CorvoError> {
    let mut content = std::fs::read_to_string(manifest_path).map_err(|e| {
        CorvoError::io(format!(
            "Failed to read {} for local patch: {}",
            manifest_path.display(),
            e
        ))
    })?;
    if content.contains("[patch.crates-io]") {
        return Ok(());
    }
    let root = repo_root.to_string_lossy().replace('\\', "/");
    content.push_str(&format!(
        "\n[patch.crates-io]\ncorvo-lang = {{ path = \"{}\" }}\n",
        root
    ));
    std::fs::write(manifest_path, content).map_err(|e| {
        CorvoError::io(format!(
            "Failed to write {} after local patch: {}",
            manifest_path.display(),
            e
        ))
    })?;
    Ok(())
}

/// Ensure the persistent build directory exists and return its path.
///
/// Unlike previous versions of this function, the directory is **never wiped**:
/// `cargo` relies on its `target/` cache (and the `Cargo.lock` it writes there)
/// surviving between compilations to avoid recompiling the runtime crate and
/// its transitive dependencies on every invocation.
fn create_build_dir() -> Result<PathBuf, CorvoError> {
    let base = cache_dir()?;
    std::fs::create_dir_all(&base)
        .map_err(|e| CorvoError::io(format!("Failed to create build dir: {}", e)))?;
    Ok(base)
}

fn copy_binary(source: &Path, target: &Path) -> Result<PathBuf, CorvoError> {
    let final_target = if target.is_dir() {
        let name = if cfg!(target_os = "windows") {
            "out.exe"
        } else {
            "out"
        };
        target.join(name)
    } else {
        target.to_path_buf()
    };

    std::fs::copy(source, &final_target)
        .map_err(|e| CorvoError::io(format!("Failed to copy binary: {}", e)))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&final_target)
            .map_err(|e| CorvoError::io(format!("Failed to read permissions: {}", e)))?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&final_target, perms)
            .map_err(|e| CorvoError::io(format!("Failed to set permissions: {}", e)))?;
    }

    Ok(final_target)
}

/// Remove the `prep { ... }` block from `source` so that the compiled binary
/// does not embed it.  The token list (already produced by the lexer) is used
/// to locate the exact character range of the block, which means the removal
/// is accurate regardless of whitespace, comments, or string contents.
///
/// If the source contains no prep block the original source is returned
/// unchanged.
fn strip_prep_block(source: &str, tokens: &[Token]) -> String {
    // Locate the `prep` keyword token.
    let prep_idx = match tokens
        .iter()
        .position(|t| matches!(t.token_type, TokenType::Prep))
    {
        Some(idx) => idx,
        None => return source.to_string(),
    };

    let prep_start = tokens[prep_idx].span.start.offset;

    // Walk forward from the `prep` token and track brace depth to find the
    // matching closing `}`.  StringInterpolation tokens already have their
    // inner braces consumed by the lexer, so only top-level LeftBrace /
    // RightBrace tokens affect the depth counter.
    let mut brace_depth: usize = 0;
    let mut prep_end: Option<usize> = None;

    for token in &tokens[prep_idx..] {
        match &token.token_type {
            TokenType::LeftBrace => brace_depth += 1,
            TokenType::RightBrace => {
                // Guard against malformed source where `}` appears before any `{`.
                if brace_depth == 0 {
                    return source.to_string();
                }
                brace_depth -= 1;
                if brace_depth == 0 {
                    // span.end.offset points to the character *after* the `}`.
                    prep_end = Some(token.span.end.offset);
                    break;
                }
            }
            _ => {}
        }
    }

    let prep_end = match prep_end {
        Some(end) => end,
        // Malformed source – return as-is so the compiler can report the error.
        None => return source.to_string(),
    };

    // Reconstruct the source without the prep block.  Offsets are character
    // (not byte) indices, matching how the lexer advances its position.
    let chars: Vec<char> = source.chars().collect();
    let before: String = chars[..prep_start].iter().collect();
    let after: String = chars[prep_end..].iter().collect();

    format!("{}{}", before, after).trim().to_string()
}

/// Generate a random 32-byte key used to XOR-obfuscate the embedded source.
/// A simple LCG seeded from the current nanosecond timestamp is sufficient –
/// the goal is a different key per compilation so that the encrypted bytes
/// look like noise to `strings`, not cryptographic security.
fn generate_obfuscation_key() -> Vec<u8> {
    // Arbitrary fallback constant used when the system clock is unavailable.
    // The value has no special meaning; it just ensures the key is never all-zero.
    const FALLBACK_SEED: u64 = 0x6c62_272e_07bb_0142;
    // Each LCG iteration produces one u64 (8 bytes).  Four iterations → 32 bytes.
    const ITERATIONS: usize = 4;

    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(FALLBACK_SEED);

    let mut state = seed;
    let mut key = Vec::with_capacity(ITERATIONS * 8);
    for _ in 0..ITERATIONS {
        // LCG constants from Knuth
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        key.extend_from_slice(&state.to_le_bytes());
    }
    key
}

/// XOR-encrypt `data` with a repeating `key`.
fn xor_encrypt(data: &[u8], key: &[u8]) -> Vec<u8> {
    data.iter()
        .enumerate()
        .map(|(i, &b)| b ^ key[i % key.len()])
        .collect()
}

/// Serialize the statics map to a JSON byte vector, encoding non-JSON-safe
/// f64 values (NaN, ±Infinity) as sentinel objects so they survive the
/// round-trip through the encrypted blob.
fn statics_to_json_bytes(statics: &HashMap<String, Value>) -> Vec<u8> {
    let json_map: serde_json::Map<String, serde_json::Value> = statics
        .iter()
        .map(|(k, v)| (k.clone(), value_to_json_value(v)))
        .collect();
    serde_json::to_vec(&serde_json::Value::Object(json_map))
        .expect("Failed to serialize statics to JSON")
}

/// Convert a corvo `Value` to a `serde_json::Value`.
///
/// `f64::NAN`, `f64::INFINITY`, and `f64::NEG_INFINITY` are not valid JSON
/// numbers, so they are stored as `{"__corvo_f64": "nan"}` /
/// `{"__corvo_f64": "inf"}` / `{"__corvo_f64": "-inf"}` sentinels.
fn value_to_json_value(v: &Value) -> serde_json::Value {
    match v {
        Value::Null => serde_json::Value::Null,
        Value::Boolean(b) => serde_json::Value::Bool(*b),
        Value::Number(n) if n.is_nan() => {
            serde_json::json!({"__corvo_f64": "nan"})
        }
        Value::Number(n) if *n == f64::INFINITY => {
            serde_json::json!({"__corvo_f64": "inf"})
        }
        Value::Number(n) if *n == f64::NEG_INFINITY => {
            serde_json::json!({"__corvo_f64": "-inf"})
        }
        Value::Number(n) => serde_json::Value::Number(
            serde_json::Number::from_f64(*n)
                .expect("non-finite f64 must be handled by the guards above"),
        ),
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::List(items) => {
            serde_json::Value::Array(items.iter().map(value_to_json_value).collect())
        }
        Value::Map(map) => {
            let obj: serde_json::Map<String, serde_json::Value> = map
                .iter()
                .map(|(k, v)| (k.clone(), value_to_json_value(v)))
                .collect();
            serde_json::Value::Object(obj)
        }
        Value::Regex(pattern, flags) => {
            serde_json::json!({"__corvo_regex": {"pattern": pattern, "flags": flags}})
        }
        Value::Procedure(_) | Value::NativeProcedure { .. } => {
            panic!("procedures cannot be serialized as statics")
        }
        Value::Shared(_) => {
            panic!("shared values cannot be serialized as statics")
        }
        Value::DatabasePool(_) => {
            panic!("database pools cannot be serialized as statics")
        }
        Value::AmqpConnection(_) => {
            panic!("AMQP connections cannot be serialized as statics")
        }
    }
}

/// Format a byte slice as a Rust array literal body, e.g. `[1u8, 2u8, 3u8]`.
fn bytes_to_rust_array(bytes: &[u8]) -> String {
    let parts: Vec<String> = bytes.iter().map(|b| format!("{}u8", b)).collect();
    format!("[{}]", parts.join(", "))
}

/// Generate the source code for the `abort_if_debugged` function that is
/// injected into compiled binaries when `--no-debug` is used.
///
/// The generated function terminates the process immediately if it detects any
/// of the following tools:
///
/// * **strace** — intercepted via `ptrace(PTRACE_TRACEME)` on Linux
/// * **gdb / LLDB** — intercepted via `ptrace(PTRACE_TRACEME)` on Linux and
///   `ptrace(PT_DENY_ATTACH)` + `sysctl(KERN_PROC_PID)` on macOS
/// * **rr** — intercepted via `ptrace(PTRACE_TRACEME)` on Linux plus the
///   `RUNNING_UNDER_RR` environment variable set by the rr runtime
/// * **Valgrind** — detected via `vgpreload` entries in `/proc/self/maps` on
///   Linux and the `VALGRIND_OPTS` environment variable
/// * **WinDbg** (and other Windows debuggers) — detected via
///   `IsDebuggerPresent` and `CheckRemoteDebuggerPresent` on Windows
fn generate_anti_debug_fn() -> String {
    let mut code = String::new();

    // ── Shared helper (all platforms) ────────────────────────────────────────
    // Environment-variable checks for tools that advertise themselves:
    //   • rr sets RUNNING_UNDER_RR during replay.
    //   • Valgrind propagates VALGRIND_OPTS to the child process.
    code.push_str("fn abort_if_debugged() {\n");
    code.push_str("    if std::env::var_os(\"RUNNING_UNDER_RR\").is_some() {\n");
    code.push_str("        std::process::exit(1);\n");
    code.push_str("    }\n");
    code.push_str("    if std::env::var_os(\"VALGRIND_OPTS\").is_some() {\n");
    code.push_str("        std::process::exit(1);\n");
    code.push_str("    }\n");

    // ── Linux ─────────────────────────────────────────────────────────────────
    // 1. ptrace(PTRACE_TRACEME): returns -1/EPERM when a tracer (gdb, strace,
    //    rr, LLDB) is already attached, because only one tracer may be active
    //    at a time and the tracer has already called ptrace on us.
    // 2. /proc/self/status TracerPid: non-zero when any ptrace-based tracer
    //    is attached, including after ptrace(PTRACE_TRACEME) above succeeds.
    // 3. /proc/self/maps: Valgrind preloads its own shared libraries whose
    //    paths contain "vgpreload"; scanning the memory map catches it even
    //    when VALGRIND_OPTS is unset.
    code.push_str("    #[cfg(target_os = \"linux\")]\n");
    code.push_str("    {\n");
    // Extract the ptrace call into a variable for readability.
    code.push_str("        let ptrace_result = unsafe {\n");
    code.push_str("            libc::ptrace(\n");
    code.push_str("                libc::PTRACE_TRACEME,\n");
    code.push_str("                0,\n");
    code.push_str("                std::ptr::null_mut::<libc::c_void>(),\n");
    code.push_str("                std::ptr::null_mut::<libc::c_void>(),\n");
    code.push_str("            )\n");
    code.push_str("        };\n");
    code.push_str("        if ptrace_result != 0 {\n");
    code.push_str("            std::process::exit(1);\n");
    code.push_str("        }\n");
    code.push_str("        if let Ok(status) = std::fs::read_to_string(\"/proc/self/status\") {\n");
    code.push_str("            for line in status.lines() {\n");
    code.push_str("                if let Some(rest) = line.strip_prefix(\"TracerPid:\") {\n");
    // Use map(...).unwrap_or(false) to avoid silently treating parse errors
    // as "no tracer": only a confirmed zero value passes the check.
    code.push_str("                    if rest.trim().parse::<i64>().map(|pid| pid != 0).unwrap_or(false) {\n");
    code.push_str("                        std::process::exit(1);\n");
    code.push_str("                    }\n");
    code.push_str("                    break;\n");
    code.push_str("                }\n");
    code.push_str("            }\n");
    code.push_str("        }\n");
    code.push_str("        if let Ok(maps) = std::fs::read_to_string(\"/proc/self/maps\") {\n");
    code.push_str("            if maps.contains(\"vgpreload\") || maps.contains(\"valgrind\") {\n");
    code.push_str("                std::process::exit(1);\n");
    code.push_str("            }\n");
    code.push_str("        }\n");
    code.push_str("    }\n");

    // ── macOS ─────────────────────────────────────────────────────────────────
    // 1. ptrace(PT_DENY_ATTACH = 31): sends SIGSEGV to any already-attached
    //    debugger and prevents future attachment by gdb, LLDB, Instruments, etc.
    // 2. sysctl(KERN_PROC_PID): reads the kinfo_proc struct for this PID; the
    //    P_TRACED flag (0x00000800) in kp_proc.p_flag is set whenever a
    //    debugger or Valgrind is tracing the process.
    code.push_str("    #[cfg(target_os = \"macos\")]\n");
    code.push_str("    unsafe {\n");
    code.push_str("        libc::ptrace(31 /* PT_DENY_ATTACH */, 0, std::ptr::null_mut(), 0);\n");
    code.push_str("        let pid = libc::getpid();\n");
    code.push_str("        let mut info = std::mem::MaybeUninit::<libc::kinfo_proc>::zeroed();\n");
    code.push_str("        let mut size = std::mem::size_of::<libc::kinfo_proc>();\n");
    // sysctl takes a *mut c_int for the mib parameter but does not modify the
    // array contents; cast from an immutable reference to avoid a misleading
    // `mut` binding on the array itself.
    code.push_str("        let mib: [libc::c_int; 4] = [libc::CTL_KERN, libc::KERN_PROC, libc::KERN_PROC_PID, pid];\n");
    code.push_str("        if libc::sysctl(\n");
    code.push_str("            mib.as_ptr() as *mut libc::c_int, 4,\n");
    code.push_str("            info.as_mut_ptr() as *mut libc::c_void, &mut size,\n");
    code.push_str("            std::ptr::null_mut(), 0,\n");
    code.push_str("        ) == 0 {\n");
    code.push_str("            const P_TRACED: i32 = 0x00000800;\n");
    code.push_str("            if (info.assume_init().kp_proc.p_flag & P_TRACED) != 0 {\n");
    code.push_str("                std::process::exit(1);\n");
    code.push_str("            }\n");
    code.push_str("        }\n");
    code.push_str("    }\n");

    // ── Windows ───────────────────────────────────────────────────────────────
    // IsDebuggerPresent detects WinDbg and any kernel-mode or user-mode
    // debugger that has set the process debug flag.
    // CheckRemoteDebuggerPresent catches remote debugging sessions.
    // Both functions are exported by kernel32 and do not require any extra
    // crate; they are declared inline via an extern block.
    code.push_str("    #[cfg(target_os = \"windows\")]\n");
    code.push_str("    unsafe {\n");
    code.push_str("        extern \"system\" {\n");
    code.push_str("            fn IsDebuggerPresent() -> u32;\n");
    code.push_str("            fn CheckRemoteDebuggerPresent(hProcess: *mut (), pbDebuggerPresent: *mut i32) -> i32;\n");
    code.push_str("            fn GetCurrentProcess() -> *mut ();\n");
    code.push_str("        }\n");
    code.push_str("        if IsDebuggerPresent() != 0 {\n");
    code.push_str("            std::process::exit(1);\n");
    code.push_str("        }\n");
    code.push_str("        let mut remote: i32 = 0;\n");
    code.push_str("        CheckRemoteDebuggerPresent(GetCurrentProcess(), &mut remote);\n");
    code.push_str("        if remote != 0 {\n");
    code.push_str("            std::process::exit(1);\n");
    code.push_str("        }\n");
    code.push_str("    }\n");

    code.push_str("}\n");
    code
}

/// Escape a string for use as a Rust string literal body.
///
/// This function is retained for test coverage of the escaping logic even
/// though production code no longer embeds static values as string literals.
#[allow(dead_code)]
fn escape_for_rust(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 2);
    for ch in s.chars() {
        match ch {
            '\\' => result.push_str("\\\\"),
            '"' => result.push_str("\\\""),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            _ => result.push(ch),
        }
    }
    result
}

/// Generate a Rust code expression that constructs a corvo `Value` at runtime.
///
/// This function is retained for test coverage even though production code no
/// longer embeds static values as Rust source literals — statics are now
/// obfuscated via the encrypted statics mechanism.
#[allow(dead_code)]
fn value_to_rust_code(value: &Value) -> String {
    match value {
        Value::String(s) => format!(
            "corvo_lang::type_system::Value::String(\"{}\".to_string())",
            escape_for_rust(s)
        ),
        Value::Number(n) => {
            if n.is_infinite() && n.is_sign_positive() {
                "corvo_lang::type_system::Value::Number(f64::INFINITY)".to_string()
            } else if n.is_infinite() && n.is_sign_negative() {
                "corvo_lang::type_system::Value::Number(f64::NEG_INFINITY)".to_string()
            } else if n.is_nan() {
                "corvo_lang::type_system::Value::Number(f64::NAN)".to_string()
            } else {
                format!("corvo_lang::type_system::Value::Number({:?})", n)
            }
        }
        Value::Boolean(b) => format!("corvo_lang::type_system::Value::Boolean({})", b),
        Value::Null => "corvo_lang::type_system::Value::Null".to_string(),
        Value::List(items) => {
            let items_code: Vec<String> = items.iter().map(value_to_rust_code).collect();
            format!(
                "corvo_lang::type_system::Value::List(vec![{}])",
                items_code.join(", ")
            )
        }
        Value::Map(map) => {
            let mut entries = Vec::new();
            for (k, v) in map {
                entries.push(format!(
                    "(\"{}\".to_string(), {})",
                    escape_for_rust(k),
                    value_to_rust_code(v)
                ));
            }
            format!(
                "corvo_lang::type_system::Value::Map(std::collections::HashMap::from([{}]))",
                entries.join(", ")
            )
        }
        Value::Regex(pattern, flags) => format!(
            "corvo_lang::type_system::Value::Regex(\"{}\".to_string(), \"{}\".to_string())",
            escape_for_rust(pattern),
            flags
        ),
        Value::Procedure(_) | Value::NativeProcedure { .. } => {
            panic!("procedures cannot be compiled to Rust source literals")
        }
        Value::Shared(_) => {
            panic!("shared values cannot be compiled to Rust source literals")
        }
        Value::DatabasePool(_) => {
            panic!("database pools cannot be compiled to Rust source literals")
        }
        Value::AmqpConnection(_) => {
            panic!("AMQP connections cannot be compiled to Rust source literals")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_for_rust() {
        assert_eq!(escape_for_rust("hello"), "hello");
        assert_eq!(escape_for_rust("say \"hi\""), "say \\\"hi\\\"");
        assert_eq!(escape_for_rust("line1\nline2"), "line1\\nline2");
    }

    #[test]
    fn test_value_to_rust_code_string() {
        assert_eq!(
            value_to_rust_code(&Value::String("hello".to_string())),
            "corvo_lang::type_system::Value::String(\"hello\".to_string())"
        );
    }

    #[test]
    fn test_value_to_rust_code_number() {
        let code = value_to_rust_code(&Value::Number(42.0));
        assert!(code.contains("42"));
        assert!(code.contains("Value::Number"));
    }

    #[test]
    fn test_value_to_rust_code_boolean() {
        assert_eq!(
            value_to_rust_code(&Value::Boolean(true)),
            "corvo_lang::type_system::Value::Boolean(true)"
        );
    }

    #[test]
    fn test_value_to_rust_code_null() {
        assert_eq!(
            value_to_rust_code(&Value::Null),
            "corvo_lang::type_system::Value::Null"
        );
    }

    #[test]
    fn test_value_to_rust_code_list() {
        let code = value_to_rust_code(&Value::List(vec![Value::Number(1.0), Value::Number(2.0)]));
        assert!(code.contains("Value::List"));
        assert!(code.contains("vec!["));
    }

    #[test]
    fn test_value_to_rust_code_map() {
        let mut map = std::collections::HashMap::new();
        map.insert("key".to_string(), Value::Number(42.0));
        let code = value_to_rust_code(&Value::Map(map));
        assert!(code.contains("Value::Map"));
        assert!(code.contains("key"));
    }

    #[test]
    fn test_pre_execute_captures_statics() {
        std::env::set_var("CORVO_TEST_VAR", "test_value_123");
        let source = r#"prep { static.set("config", os.get_env("CORVO_TEST_VAR")) }"#.to_string();
        let mut compiler = Compiler::new(source, PathBuf::from("test.corvo"));
        compiler.pre_execute().unwrap();

        assert!(compiler.statics.contains_key("config"));
        assert_eq!(
            compiler.statics.get("config").unwrap(),
            &Value::String("test_value_123".to_string())
        );
        std::env::remove_var("CORVO_TEST_VAR");
    }

    #[test]
    fn test_pre_execute_multiple_statics() {
        std::env::set_var("CORVO_A", "value_a");
        std::env::set_var("CORVO_B", "value_b");
        let source = r#"
            prep {
                static.set("a", os.get_env("CORVO_A"))
                static.set("b", os.get_env("CORVO_B"))
                static.set("c", 42)
            }
        "#
        .to_string();
        let mut compiler = Compiler::new(source, PathBuf::from("test.corvo"));
        compiler.pre_execute().unwrap();

        assert_eq!(compiler.statics.len(), 3);
        assert_eq!(
            compiler.statics.get("a").unwrap(),
            &Value::String("value_a".to_string())
        );
        assert_eq!(
            compiler.statics.get("b").unwrap(),
            &Value::String("value_b".to_string())
        );
        assert_eq!(compiler.statics.get("c").unwrap(), &Value::Number(42.0));
        std::env::remove_var("CORVO_A");
        std::env::remove_var("CORVO_B");
    }

    #[test]
    fn test_generate_main_rs_with_statics() {
        let mut compiler = Compiler::new(
            "sys.echo(static.get(\"key\"))".to_string(),
            PathBuf::from("test.corvo"),
        );
        compiler
            .statics
            .insert("key".to_string(), Value::String("baked_value".to_string()));

        let build_dir = std::env::temp_dir().join("corvo_test_statics_gen");
        let _ = std::fs::remove_dir_all(&build_dir);
        std::fs::create_dir_all(build_dir.join("src")).unwrap();

        compiler.generate_main_rs(&build_dir).unwrap();

        let main_rs = std::fs::read_to_string(build_dir.join("src/main.rs")).unwrap();
        // Static keys and values must NOT appear as plaintext — they are
        // encrypted and loaded via load_statics_from_encrypted_bytes.
        assert!(
            !main_rs.contains("baked_value"),
            "static values must not appear as plaintext in the generated binary"
        );
        assert!(
            !main_rs.contains("\"key\""),
            "static key names must not appear as plaintext in the generated binary"
        );
        assert!(
            main_rs.contains("ENCRYPTED_STATICS"),
            "encrypted statics array must be present"
        );
        assert!(
            main_rs.contains("STATICS_KEY"),
            "statics obfuscation key must be present"
        );
        assert!(
            main_rs.contains("load_statics_from_encrypted_bytes"),
            "encrypted statics loader must be called"
        );
        assert!(main_rs.contains("run_source_with_state"));
        assert!(
            main_rs.contains("set_script_argv"),
            "generated main must forward process argv to the runtime"
        );

        let _ = std::fs::remove_dir_all(&build_dir);
    }

    // --- strip_prep_block tests ---

    fn tokenize(source: &str) -> Vec<Token> {
        crate::lexer::Lexer::new(source).tokenize().unwrap()
    }

    #[test]
    fn test_strip_prep_block_removes_prep() {
        let source = "prep {\n    static.set(\"key\", 42)\n}\nsys.echo(\"hello\")";
        let tokens = tokenize(source);
        let stripped = strip_prep_block(source, &tokens);
        assert!(
            !stripped.contains("prep"),
            "stripped source must not contain 'prep'"
        );
        assert!(
            stripped.contains("sys.echo"),
            "non-prep statements must be preserved"
        );
    }

    #[test]
    fn test_strip_prep_block_no_prep() {
        let source = "sys.echo(\"hello\")";
        let tokens = tokenize(source);
        let stripped = strip_prep_block(source, &tokens);
        assert_eq!(stripped, source.trim());
    }

    #[test]
    fn test_strip_prep_block_only_prep() {
        let source = "prep {\n    static.set(\"x\", 1)\n}";
        let tokens = tokenize(source);
        let stripped = strip_prep_block(source, &tokens);
        assert!(
            stripped.is_empty(),
            "stripping a prep-only source yields an empty string"
        );
    }

    #[test]
    fn test_strip_prep_block_multiline() {
        let source = "prep {\n    static.set(\"a\", 1)\n    static.set(\"b\", 2)\n}\nvar.set(\"x\", static.get(\"a\"))";
        let tokens = tokenize(source);
        let stripped = strip_prep_block(source, &tokens);
        assert!(!stripped.contains("prep"), "prep block must be stripped");
        assert!(stripped.contains("var.set"), "non-prep code must survive");
    }

    #[test]
    fn test_pre_execute_strips_prep_block() {
        let source = "prep {\n    static.set(\"msg\", \"hello\")\n}\nsys.echo(static.get(\"msg\"))"
            .to_string();
        let mut compiler = Compiler::new(source, PathBuf::from("test.corvo"));
        compiler.pre_execute().unwrap();

        // The embedded source must not contain the prep block.
        assert!(
            !compiler.source_without_prep.contains("prep"),
            "source_without_prep must not contain 'prep'"
        );
        // The rest of the script must still be present.
        assert!(
            compiler.source_without_prep.contains("sys.echo"),
            "non-prep statements must be present in source_without_prep"
        );
        // The static value must still have been captured.
        assert_eq!(
            compiler.statics.get("msg").unwrap(),
            &Value::String("hello".to_string())
        );
    }

    #[test]
    fn test_generate_main_rs_does_not_embed_prep_block() {
        let source = "prep {\n    static.set(\"key\", \"baked\")\n}\nsys.echo(static.get(\"key\"))"
            .to_string();
        let mut compiler = Compiler::new(source, PathBuf::from("test.corvo"));
        compiler.pre_execute().unwrap();

        let build_dir = std::env::temp_dir().join("corvo_test_no_prep_in_binary");
        let _ = std::fs::remove_dir_all(&build_dir);
        std::fs::create_dir_all(build_dir.join("src")).unwrap();

        compiler.generate_main_rs(&build_dir).unwrap();

        let main_rs = std::fs::read_to_string(build_dir.join("src/main.rs")).unwrap();
        // The prep block must not appear in the embedded source string.
        assert!(
            !main_rs.contains("static.set"),
            "prep block (static.set) must not be embedded in the binary source"
        );
        // Statics are now loaded via the encrypted statics mechanism —
        // individual state.static_set calls must NOT appear.
        assert!(
            !main_rs.contains("state.static_set"),
            "statics must not be injected as plaintext state.static_set calls"
        );
        // The baked value must NOT appear as a readable string.
        assert!(
            !main_rs.contains("baked"),
            "baked value must not appear as plaintext in the generated binary"
        );
        // The encrypted statics mechanism must be present instead.
        assert!(
            main_rs.contains("ENCRYPTED_STATICS"),
            "encrypted statics array must be present"
        );
        assert!(
            main_rs.contains("load_statics_from_encrypted_bytes"),
            "encrypted statics loader must be called"
        );
        // The Corvo source is now embedded as encrypted bytes – plaintext
        // identifiers from the script must NOT appear in the generated code.
        assert!(
            !main_rs.contains("sys.echo"),
            "Corvo source must not appear as plaintext in the generated binary"
        );
        // The encrypted source mechanism must be present.
        assert!(
            main_rs.contains("ENCRYPTED_SOURCE"),
            "encrypted source array must be present"
        );
        assert!(
            main_rs.contains("OBFUSCATION_KEY"),
            "obfuscation key must be present"
        );

        let _ = std::fs::remove_dir_all(&build_dir);
    }

    #[test]
    fn test_generate_main_rs_obfuscates_source() {
        let source = "sys.echo(\"secret logic\")".to_string();
        let compiler = Compiler::new(source, PathBuf::from("test.corvo"));

        let build_dir = std::env::temp_dir().join("corvo_test_obfuscation");
        let _ = std::fs::remove_dir_all(&build_dir);
        std::fs::create_dir_all(build_dir.join("src")).unwrap();

        compiler.generate_main_rs(&build_dir).unwrap();

        let main_rs = std::fs::read_to_string(build_dir.join("src/main.rs")).unwrap();

        // The source string must NOT appear as plaintext.
        assert!(
            !main_rs.contains("sys.echo"),
            "source code must not appear as plaintext"
        );
        assert!(
            !main_rs.contains("secret logic"),
            "source strings must not appear as plaintext"
        );
        // The encrypted byte array and key must be embedded instead.
        assert!(
            main_rs.contains("ENCRYPTED_SOURCE"),
            "encrypted source array must be present"
        );
        assert!(
            main_rs.contains("OBFUSCATION_KEY"),
            "obfuscation key must be present"
        );
        // The runtime must decrypt and execute the source.
        assert!(
            main_rs.contains("run_source_with_state"),
            "runtime execution must be invoked"
        );

        let _ = std::fs::remove_dir_all(&build_dir);
    }

    #[test]
    fn test_xor_encrypt_roundtrip() {
        let data = b"Hello, Corvo! This is some Corvo source code.";
        let key = generate_obfuscation_key();
        let encrypted = xor_encrypt(data, &key);
        // XOR is its own inverse: encrypting twice yields the original.
        let decrypted = xor_encrypt(&encrypted, &key);
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_obfuscation_key_length() {
        let key = generate_obfuscation_key();
        assert_eq!(key.len(), 32, "key must be 32 bytes");
    }

    #[test]
    fn test_bytes_to_rust_array() {
        let arr = bytes_to_rust_array(&[0u8, 255u8, 42u8]);
        assert_eq!(arr, "[0u8, 255u8, 42u8]");
    }

    #[test]
    fn test_statics_to_json_bytes_roundtrip() {
        let mut statics = HashMap::new();
        statics.insert("KEY".to_string(), Value::String("secret".to_string()));
        statics.insert("NUM".to_string(), Value::Number(2.5));
        statics.insert("FLAG".to_string(), Value::Boolean(true));
        statics.insert("EMPTY".to_string(), Value::Null);

        let json_bytes = statics_to_json_bytes(&statics);
        // Must be valid JSON
        let parsed: serde_json::Value = serde_json::from_slice(&json_bytes).unwrap();
        let obj = parsed.as_object().unwrap();
        assert_eq!(obj["KEY"].as_str().unwrap(), "secret");
        assert!((obj["NUM"].as_f64().unwrap() - 2.5).abs() < 1e-10);
        assert!(obj["FLAG"].as_bool().unwrap());
        assert!(obj["EMPTY"].is_null());
    }

    #[test]
    fn test_value_to_json_value_nan_inf() {
        let nan = value_to_json_value(&Value::Number(f64::NAN));
        assert_eq!(nan["__corvo_f64"].as_str().unwrap(), "nan");

        let inf = value_to_json_value(&Value::Number(f64::INFINITY));
        assert_eq!(inf["__corvo_f64"].as_str().unwrap(), "inf");

        let neg_inf = value_to_json_value(&Value::Number(f64::NEG_INFINITY));
        assert_eq!(neg_inf["__corvo_f64"].as_str().unwrap(), "-inf");
    }

    #[test]
    fn test_statics_encryption_hides_plaintext() {
        let mut statics = HashMap::new();
        statics.insert(
            "SECRET_KEY".to_string(),
            Value::String("super_secret_value".to_string()),
        );

        let json_bytes = statics_to_json_bytes(&statics);
        let key = generate_obfuscation_key();
        let encrypted = xor_encrypt(&json_bytes, &key);

        // The encrypted bytes must not contain the plaintext key or value.
        let encrypted_str = String::from_utf8_lossy(&encrypted);
        assert!(
            !encrypted_str.contains("SECRET_KEY"),
            "encrypted statics must not contain plaintext key names"
        );
        assert!(
            !encrypted_str.contains("super_secret_value"),
            "encrypted statics must not contain plaintext values"
        );

        // Decrypting must restore the original JSON.
        let decrypted = xor_encrypt(&encrypted, &key);
        assert_eq!(decrypted, json_bytes);
    }

    // --- --no-debug / anti-debugging tests ---

    #[test]
    fn test_generate_anti_debug_fn_contains_env_checks() {
        let code = generate_anti_debug_fn();
        assert!(
            code.contains("RUNNING_UNDER_RR"),
            "must check RUNNING_UNDER_RR env var for rr detection"
        );
        assert!(
            code.contains("VALGRIND_OPTS"),
            "must check VALGRIND_OPTS env var for Valgrind detection"
        );
    }

    #[test]
    fn test_generate_anti_debug_fn_linux_checks() {
        let code = generate_anti_debug_fn();
        assert!(
            code.contains("PTRACE_TRACEME"),
            "Linux: must call ptrace(PTRACE_TRACEME) to detect attached tracers"
        );
        assert!(
            code.contains("TracerPid"),
            "Linux: must read TracerPid from /proc/self/status"
        );
        assert!(
            code.contains("vgpreload"),
            "Linux: must scan /proc/self/maps for Valgrind preload libraries"
        );
    }

    #[test]
    fn test_generate_anti_debug_fn_macos_checks() {
        let code = generate_anti_debug_fn();
        assert!(
            code.contains("PT_DENY_ATTACH"),
            "macOS: must call ptrace(PT_DENY_ATTACH) to block debugger attachment"
        );
        assert!(
            code.contains("P_TRACED"),
            "macOS: must check P_TRACED flag via sysctl(KERN_PROC_PID)"
        );
    }

    #[test]
    fn test_generate_anti_debug_fn_windows_checks() {
        let code = generate_anti_debug_fn();
        assert!(
            code.contains("IsDebuggerPresent"),
            "Windows: must call IsDebuggerPresent to detect WinDbg"
        );
        assert!(
            code.contains("CheckRemoteDebuggerPresent"),
            "Windows: must call CheckRemoteDebuggerPresent for remote debuggers"
        );
    }

    #[test]
    fn test_generate_main_rs_with_no_debug_injects_guard() {
        let compiler = Compiler {
            source: "sys.echo(\"hi\")".to_string(),
            source_without_prep: "sys.echo(\"hi\")".to_string(),
            _source_path: PathBuf::from("test.corvo"),
            build_mode: BuildMode::Release,
            statics: HashMap::new(),
            no_debug: true,
        };

        let build_dir = std::env::temp_dir().join("corvo_test_no_debug_gen");
        let _ = std::fs::remove_dir_all(&build_dir);
        std::fs::create_dir_all(build_dir.join("src")).unwrap();

        compiler.generate_main_rs(&build_dir).unwrap();

        let main_rs = std::fs::read_to_string(build_dir.join("src/main.rs")).unwrap();

        assert!(
            main_rs.contains("abort_if_debugged"),
            "generated main.rs must define and call abort_if_debugged"
        );
        assert!(
            main_rs.contains("RUNNING_UNDER_RR"),
            "anti-debug guard must include rr env-var check"
        );
        assert!(
            main_rs.contains("PTRACE_TRACEME"),
            "anti-debug guard must include Linux ptrace check"
        );
        assert!(
            main_rs.contains("IsDebuggerPresent"),
            "anti-debug guard must include Windows debugger check"
        );

        let _ = std::fs::remove_dir_all(&build_dir);
    }

    #[test]
    fn test_generate_main_rs_without_no_debug_has_no_guard() {
        let compiler = Compiler::new("sys.echo(\"hi\")".to_string(), PathBuf::from("test.corvo"));

        let build_dir = std::env::temp_dir().join("corvo_test_no_debug_absent");
        let _ = std::fs::remove_dir_all(&build_dir);
        std::fs::create_dir_all(build_dir.join("src")).unwrap();

        compiler.generate_main_rs(&build_dir).unwrap();

        let main_rs = std::fs::read_to_string(build_dir.join("src/main.rs")).unwrap();

        assert!(
            !main_rs.contains("abort_if_debugged"),
            "without --no-debug the guard function must not be generated"
        );

        let _ = std::fs::remove_dir_all(&build_dir);
    }

    #[test]
    fn test_generate_cargo_toml_with_no_debug_adds_libc() {
        let compiler = Compiler {
            source: "sys.echo(\"hi\")".to_string(),
            source_without_prep: "sys.echo(\"hi\")".to_string(),
            _source_path: PathBuf::from("test.corvo"),
            build_mode: BuildMode::Release,
            statics: HashMap::new(),
            no_debug: true,
        };

        let build_dir = std::env::temp_dir().join("corvo_test_cargo_toml_no_debug");
        let _ = std::fs::remove_dir_all(&build_dir);
        std::fs::create_dir_all(&build_dir).unwrap();

        compiler
            .generate_cargo_toml(&build_dir, "test_bin")
            .unwrap();

        let cargo_toml = std::fs::read_to_string(build_dir.join("Cargo.toml")).unwrap();
        assert!(
            cargo_toml.contains("libc"),
            "--no-debug Cargo.toml must include the libc dependency"
        );

        let _ = std::fs::remove_dir_all(&build_dir);
    }

    #[test]
    fn test_generate_cargo_toml_without_no_debug_omits_libc() {
        let compiler = Compiler::new("sys.echo(\"hi\")".to_string(), PathBuf::from("test.corvo"));

        let build_dir = std::env::temp_dir().join("corvo_test_cargo_toml_normal");
        let _ = std::fs::remove_dir_all(&build_dir);
        std::fs::create_dir_all(&build_dir).unwrap();

        compiler
            .generate_cargo_toml(&build_dir, "test_bin")
            .unwrap();

        let cargo_toml = std::fs::read_to_string(build_dir.join("Cargo.toml")).unwrap();
        assert!(
            !cargo_toml.contains("libc"),
            "without --no-debug the libc dependency must not appear in Cargo.toml"
        );

        let _ = std::fs::remove_dir_all(&build_dir);
    }

    // --- persistent cache dir tests ---

    #[test]
    fn test_generate_cargo_toml_uses_fixed_compile_workspace_name() {
        // The cache directory may live at any user-provided path (including
        // ones with characters cargo rejects in package names, e.g. dots from
        // `mktemp -t` on macOS). Compile mode must therefore use a fixed,
        // always-valid package identifier rather than the build dir's filename.
        let compiler = Compiler::new("sys.echo(\"hi\")".to_string(), PathBuf::from("test.corvo"));

        let build_dir = std::env::temp_dir().join("corvo.test.dotted.dir.name");
        let _ = std::fs::remove_dir_all(&build_dir);
        std::fs::create_dir_all(&build_dir).unwrap();

        compiler
            .generate_cargo_toml(&build_dir, COMPILE_MODE_BIN_NAME)
            .unwrap();

        let cargo_toml = std::fs::read_to_string(build_dir.join("Cargo.toml")).unwrap();
        assert!(
            cargo_toml.contains("name = \"corvo_compile_workspace\""),
            "compile mode must use the fixed `corvo_compile_workspace` package name; got:\n{}",
            cargo_toml
        );
        assert!(
            !cargo_toml.contains("corvo.test.dotted.dir.name"),
            "compile mode must not derive the package name from the build dir path; got:\n{}",
            cargo_toml
        );

        let _ = std::fs::remove_dir_all(&build_dir);
    }

    #[test]
    fn test_generate_cargo_toml_compile_mode_rewrites_fresh() {
        // When the build dir is reused across compilations, compile mode must
        // rewrite Cargo.toml from scratch so that stale `[[bin]]` entries from
        // previous transpilations or experiments don't accumulate.
        let compiler = Compiler::new("sys.echo(\"hi\")".to_string(), PathBuf::from("test.corvo"));

        let build_dir = std::env::temp_dir().join("corvo_test_cargo_toml_fresh");
        let _ = std::fs::remove_dir_all(&build_dir);
        std::fs::create_dir_all(&build_dir).unwrap();

        // Pre-seed Cargo.toml with a stale extra binary entry that should be
        // dropped on the next compile-mode invocation.
        std::fs::write(
            build_dir.join("Cargo.toml"),
            "[package]\nname = \"corvo_project\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[[bin]]\nname = \"stale_old_binary\"\npath = \"src/stale.rs\"\n",
        )
        .unwrap();

        compiler
            .generate_cargo_toml(&build_dir, COMPILE_MODE_BIN_NAME)
            .unwrap();

        let cargo_toml = std::fs::read_to_string(build_dir.join("Cargo.toml")).unwrap();
        assert!(
            !cargo_toml.contains("stale_old_binary"),
            "compile mode must rewrite Cargo.toml fresh and drop stale [[bin]] entries"
        );
        assert!(
            cargo_toml.contains(COMPILE_MODE_BIN_NAME),
            "compile mode must register the {} binary",
            COMPILE_MODE_BIN_NAME
        );

        let _ = std::fs::remove_dir_all(&build_dir);
    }

    /// Run `body` with `CORVO_CACHE_DIR` set to `value`, restoring the previous
    /// value (or unsetting it) afterwards regardless of panics.
    ///
    /// Uses [`super::corvo_cache_dir_test_lock`] with integration tests that
    /// touch the same environment variable.
    fn with_cache_env(value: &Path, body: impl FnOnce()) {
        let _guard = corvo_cache_dir_test_lock();
        let prev = std::env::var_os(CACHE_DIR_ENV);
        std::env::set_var(CACHE_DIR_ENV, value);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(body));
        match prev {
            Some(v) => std::env::set_var(CACHE_DIR_ENV, v),
            None => std::env::remove_var(CACHE_DIR_ENV),
        }
        if let Err(p) = result {
            std::panic::resume_unwind(p);
        }
    }

    #[test]
    fn test_cache_dir_honors_env_override() {
        let unique =
            std::env::temp_dir().join(format!("corvo_test_cache_override_{}", std::process::id()));

        with_cache_env(&unique, || {
            assert_eq!(cache_dir().unwrap(), unique);
        });
    }

    #[test]
    fn test_cache_dir_rejects_relative_env_override() {
        let _guard = corvo_cache_dir_test_lock();
        let prev = std::env::var_os(CACHE_DIR_ENV);
        std::env::set_var(CACHE_DIR_ENV, "relative/cache/path");
        let err = cache_dir().unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains(CACHE_DIR_ENV) && msg.contains("absolute"),
            "unexpected error: {msg}"
        );
        match prev {
            Some(v) => std::env::set_var(CACHE_DIR_ENV, v),
            None => std::env::remove_var(CACHE_DIR_ENV),
        }
    }

    #[test]
    fn test_cache_dir_whitespace_only_override_is_ignored() {
        let _guard = corvo_cache_dir_test_lock();
        let prev = std::env::var_os(CACHE_DIR_ENV);
        std::env::set_var(CACHE_DIR_ENV, "   ");
        let with_blank = cache_dir().unwrap();
        std::env::remove_var(CACHE_DIR_ENV);
        let unset = cache_dir().unwrap();
        assert!(
            with_blank == unset,
            "whitespace-only {} should behave like unset",
            CACHE_DIR_ENV
        );
        match prev {
            Some(v) => std::env::set_var(CACHE_DIR_ENV, v),
            None => std::env::remove_var(CACHE_DIR_ENV),
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_cache_dir_rejects_unix_filesystem_root() {
        let _guard = corvo_cache_dir_test_lock();
        let prev = std::env::var_os(CACHE_DIR_ENV);
        std::env::set_var(CACHE_DIR_ENV, "/");
        let err = cache_dir().unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("root") || msg.contains("filesystem"),
            "unexpected error: {msg}"
        );
        match prev {
            Some(v) => std::env::set_var(CACHE_DIR_ENV, v),
            None => std::env::remove_var(CACHE_DIR_ENV),
        }
    }

    #[test]
    fn test_transpile_cargo_toml_adds_bin_when_package_name_matches_stem() {
        let tmp =
            std::env::temp_dir().join(format!("corvo_test_pkg_stem_match_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let script_path = tmp.join("hello.corvo");
        std::fs::write(&script_path, "sys.echo(\"x\")").unwrap();
        let compiler = Compiler::new("sys.echo(\"x\")".to_string(), script_path);
        compiler.generate_cargo_toml(&tmp, "hello").unwrap();
        let cargo_toml = std::fs::read_to_string(tmp.join("Cargo.toml")).unwrap();
        assert!(
            cargo_toml.contains("[[bin]]") && cargo_toml.contains("path = \"src/hello.rs\""),
            "expected [[bin]] for hello when package name matches script stem; got:\n{cargo_toml}"
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_transpile_package_name_sanitizes_tempfile_style_dir() {
        let compiler = Compiler::new("sys.echo(\"hi\")".to_string(), PathBuf::from("test.corvo"));
        let build_dir = std::env::temp_dir().join(".tmp_corvo_pkg_sanitize_test");
        let _ = std::fs::remove_dir_all(&build_dir);
        std::fs::create_dir_all(&build_dir).unwrap();
        compiler.generate_cargo_toml(&build_dir, "hello").unwrap();
        let cargo_toml = std::fs::read_to_string(build_dir.join("Cargo.toml")).unwrap();
        assert!(
            cargo_toml.contains("name = \"tmp_corvo_pkg_sanitize_test\""),
            "leading-dot tempdirs must map to a valid package name; got:\n{cargo_toml}"
        );
        assert!(
            !cargo_toml.contains("name = \".tmp"),
            "must not emit a leading-dot package name; got:\n{cargo_toml}"
        );
        let _ = std::fs::remove_dir_all(&build_dir);
    }

    #[test]
    fn test_create_build_dir_does_not_wipe_existing_files() {
        let unique =
            std::env::temp_dir().join(format!("corvo_test_cache_no_wipe_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&unique);

        with_cache_env(&unique, || {
            // First call creates the directory.
            create_build_dir().expect("first create_build_dir call must succeed");
            // Drop a marker file that simulates cargo's `target/` cache.
            let marker = unique.join("MARKER");
            std::fs::write(&marker, b"do not delete me").unwrap();

            // Second call must NOT wipe the marker — that's the whole point.
            create_build_dir().expect("second create_build_dir call must succeed");

            assert!(
                marker.exists(),
                "create_build_dir must NOT wipe the cache between calls"
            );
            assert_eq!(
                std::fs::read(&marker).unwrap(),
                b"do not delete me",
                "marker file contents must be preserved across create_build_dir calls"
            );
        });

        let _ = std::fs::remove_dir_all(&unique);
    }

    #[test]
    fn test_clean_cache_removes_dir() {
        let unique =
            std::env::temp_dir().join(format!("corvo_test_clean_cache_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&unique);

        with_cache_env(&unique, || {
            create_build_dir().unwrap();
            std::fs::write(unique.join("MARKER"), b"x").unwrap();

            let removed_first = clean_cache();
            assert!(
                matches!(removed_first, Ok(true)),
                "clean_cache must remove an existing cache dir, got {:?}",
                removed_first
            );
            assert!(
                !unique.exists(),
                "cache dir must not exist after clean_cache"
            );

            // Cleaning an already-empty cache is a no-op that must succeed.
            let removed_second = clean_cache();
            assert!(
                matches!(removed_second, Ok(false)),
                "clean_cache on an absent dir must return Ok(false), got {:?}",
                removed_second
            );
        });

        let _ = std::fs::remove_dir_all(&unique);
    }

    #[test]
    fn test_corvo_lang_local_path_must_be_absolute_for_compile() {
        let _lock = corvo_cache_dir_test_lock();
        let out = std::env::temp_dir().join(format!("corvo_compile_out_{}", std::process::id()));
        let _ = std::fs::remove_file(&out);
        let prev = std::env::var_os("CORVO_LANG_LOCAL_PATH");
        std::env::set_var("CORVO_LANG_LOCAL_PATH", "relative/corvo");
        let compiler = Compiler::new("sys.echo(\"hi\")".to_string(), PathBuf::from("t.corvo"));
        let err = compiler.compile(&out).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("absolute path"), "unexpected error: {}", msg);
        match prev {
            Some(v) => std::env::set_var("CORVO_LANG_LOCAL_PATH", v),
            None => std::env::remove_var("CORVO_LANG_LOCAL_PATH"),
        }
    }
}
