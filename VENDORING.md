# Dependency Vendoring

Corvo vendors every crates.io dependency under `vendor/` so builds can run
**fully offline** with no registry fetches.

## Why

- Reproducible builds without network access
- Auditable source for every transitive crate
- CI can prove integrity against `Cargo.lock`

## Layout

| Path | Purpose |
|------|---------|
| `vendor/<crate>-<version>/` | Vendored crate sources (`cargo vendor --versioned-dirs`) |
| `.cargo/config.toml` | Redirects `crates-io` → local `vendor/` |
| `Cargo.lock` | Exact resolved versions (committed) |
| `deny.toml` | License / advisory / ban policy |
| `patches/` | Local patches applied on top of upstream after re-vendor (if any) |

## Offline build

```bash
cargo build --offline --all-features
cargo test --offline --all-features
```

`.cargo/config.toml` replaces crates.io with `vendor/`, so `--offline` must
succeed without contacting the network.

## Refresh / update dependencies

1. Edit version requirements in `Cargo.toml` as needed.
2. Update the lockfile (network required once):

   ```bash
   cargo update
   cargo check --all-features
   cargo test --all-features
   ```

3. Re-vendor from the lockfile:

   ```bash
   cargo vendor --versioned-dirs vendor
   ```

4. Confirm offline still works:

   ```bash
   cargo clean
   cargo build --offline --all-features
   ```

5. Run license / advisory gates:

   ```bash
   cargo deny check advisories licenses bans
   ```

## Adding a new dependency

1. Add it to `Cargo.toml` with the latest **stable** version that the codebase can adopt.
2. `cargo update -p <crate>` (or full `cargo update`).
3. Fix any API breakage in Corvo.
4. Re-run `cargo vendor --versioned-dirs vendor`.
5. Ensure CI offline build and `cargo deny` still pass.

## Binary policy

We vendor **source crates only**. Native code that is **compiled from vendored
source** at build time (for example `aws-lc-sys` C/ASM) is allowed.

Windows target crates (`windows_*`, `winapi-*`) ship **platform import
libraries** (`.a` / `.lib`). These are upstream-provided linker stubs required
by the Rust Windows crates ecosystem, not opaque application binaries. They are
documented and accepted for cross-platform lockfile completeness. No
`.so` / `.dylib` / `.dll` / `.node` runtime blobs are introduced for the
default Unix build path.

## License policy

Project license: **MIT**.

Allowed dependency SPDX identifiers are listed in `deny.toml` under
`[licenses] allow`. Notable additions beyond the usual MIT/Apache set:

| SPDX | Why allowed |
|------|-------------|
| `0BSD` | Permissive (quoted_printable) |
| `BSL-1.0` | Boost Software License (clipboard-win / error-code via rustyline) |
| `CDLA-Permissive-2.0` | Certificate-bundle license (webpki-roots) |
| `MPL-2.0` | Weak file-level copyleft (`option-ext` via directories). Compatible with MIT while we do not modify MPL-covered files |

Copyleft-strong (GPL/AGPL) or unknown licenses must not be added without an
explicit documented exception.

## Makefile helpers

```bash
make vendor          # re-vendor from current Cargo.lock
make vendor-check    # offline build gate
make vendor-licenses # cargo deny licenses
make vendor-audit    # cargo deny advisories + licenses + bans
```

## CI

- `.github/workflows/vendor-integrity.yml` — offline build + vendor checksum
  consistency + `cargo deny`
- `.github/workflows/audit.yml` — scheduled / on-lockfile advisories
- `.github/dependabot.yml` — weekly Cargo / Actions updates (after merge,
  re-vendor on the follow-up PR)

## Deferred major upgrades

These stayed on the current major/minor line because the jump is API-breaking
and needs a dedicated follow-up:

| Crate | Locked | Latest stable | Notes |
|-------|--------|---------------|-------|
| miette | 5.10 | 7.x | Diagnostic API changes |
| reqwest | 0.12 | 0.13 | TLS / feature surface |
| sqlx | 0.8 | 0.9 | Query / pool APIs |
| lapin | 3.x | 4.x | Connection APIs |
| aes-gcm / sha2 / md-5 / sha1 / hmac | 0.10 / 0.12 | 0.11 / 0.13 | RustCrypto coordinated bump |
| dns-lookup | 2.x | 3.x | Lookup API |
| fs4 | 0.13 | 1.x | FileExt path |
| base64 | 0.22 | 0.23 | 0.x minor = breaking |
| serde_yaml | 0.9 | deprecated | Prefer `serde_yml` / `serde_norway` later |

## Change log

- **2026-07-23** — Initial vendor tree; redirect crates.io via `.cargo/config.toml`;
  bump `thiserror` 2, `directories` 6, `rand` 0.10, `quick-xml` 0.41,
  `rustyline` 18, `uzers` 0.12, and related floors; offline build verified.
