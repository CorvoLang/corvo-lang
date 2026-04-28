# GitHub Copilot Instructions for Corvo

When generating or modifying code for the Corvo Language repository, your **single source of truth** is the [`AGENTS.md`](AGENTS.md) file. Please refer to it for comprehensive guidelines on project architecture, testing pipelines, and pre-commit standards.

In addition to the rules in `AGENTS.md`, please follow these unique idiomatic patterns when writing Corvo scripts:

1. **No Hallucinations**: Do not hallucinate Python, Bash, or JavaScript patterns; use Corvo's explicit syntax. Corvo has zero dependencies and no `import` mechanism; rely entirely on the built-in standard library.
2. **Idiomatic Corvo**:
   - Prefer built-in shorthands (`@var++`, `@var += 5`, `@var or= (...)`).
   - Use `try` / `fallback` blocks for robust error handling.
   - When iterating, use `browse` or `async_browse` instead of manual index management.
