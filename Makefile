.PHONY: transpile transpile-examples vendor vendor-check vendor-licenses vendor-audit vendor-clean help

## Show available targets
help:
	@echo "Targets:"
	@echo "  transpile / transpile-examples  Transpile example Corvo scripts"
	@echo "  vendor                          Re-vendor crates from Cargo.lock into vendor/"
	@echo "  vendor-check                    Offline build with vendored sources"
	@echo "  vendor-licenses                 cargo deny check licenses"
	@echo "  vendor-audit                    cargo deny check advisories licenses bans"
	@echo "  vendor-clean                    Remove vendor/ (requires re-vendor before build)"
	@echo "  clean                           Remove examples/transpiled"

## Re-populate vendor/ from the current Cargo.lock (requires network once).
## Temporarily disables the crates.io → vendor redirect so cargo can fetch.
vendor:
	@echo "Vendoring crates into vendor/ ..."
	@if [ -f .cargo/config.toml ]; then mv .cargo/config.toml .cargo/config.toml.bak; fi
	cargo vendor --versioned-dirs vendor
	@if [ -f .cargo/config.toml.bak ]; then mv .cargo/config.toml.bak .cargo/config.toml; fi
	@echo "Done. Verify with: make vendor-check"

## Prove an offline build succeeds using only vendor/.
vendor-check:
	CARGO_NET_OFFLINE=true cargo build --offline --all-features
	CARGO_NET_OFFLINE=true cargo test --offline --all-features --no-run

## License policy gate (deny.toml).
vendor-licenses:
	cargo deny check licenses

## Advisories + licenses + bans.
vendor-audit:
	cargo deny check advisories licenses bans

## Remove vendored sources (build will need network or a re-vendor).
vendor-clean:
	rm -rf vendor

transpile: transpile-examples

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

# Clean all generated files
.PHONY: clean
clean:
	@rm -rf examples/transpiled
