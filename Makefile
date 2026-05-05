.PHONY: transpile transpile-examples transpile-coreutils

transpile: transpile-examples transpile-coreutils

transpile-examples:
	@echo "Transpiling examples..."
	@rm -rf examples/transpiled || true 
	@mkdir -p examples/transpiled
	@for file in examples/*.corvo; do \
		COREUTILS_VERSION=0.0.1-alpha cargo run --quiet -- --transpile $$file -o examples/transpiled > /dev/null; \
	done
	@if ! grep -q "\[patch.crates-io\]" examples/transpiled/Cargo.toml; then \
		printf "\n[patch.crates-io]\ncorvo-lang = { path = \"../../\" }\n" >> examples/transpiled/Cargo.toml; \
	fi
	@echo "Examples transpiled to examples/transpiled"
	cd examples/transpiled && cargo fmt && cargo clippy



transpile-coreutils:
	@echo "Transpiling coreutils..."
	@rm -rf coreutils/transpiled || true
	@mkdir -p coreutils/transpiled
	@for file in coreutils/*.corvo; do \
		cargo run --quiet -- --transpile $$file -o coreutils/transpiled > /dev/null; \
	done
	@if ! grep -q "\[patch.crates-io\]" coreutils/transpiled/Cargo.toml; then \
		printf "\n[patch.crates-io]\ncorvo-lang = { path = \"../../\" }\n" >> coreutils/transpiled/Cargo.toml; \
	fi
	@echo "Coreutils transpiled to coreutils/transpiled"
	cd coreutils/transpiled && cargo fmt && cargo clippy

# Clean all generated files
.PHONY: clean
clean:
	@rm -rf examples/transpiled coreutils/transpiled
