# Versioning

Rasa follows date-based versioning: `YYYY.M.DD`

- **YYYY** — year
- **M** — month (no leading zero)
- **DD** — day

Example: `2026.3.13` = March 13, 2026

The version is the source of truth in the `VERSION` file at the repo root. `Cargo.toml` workspace version must match.

To bump: `make version-bump V=2026.3.15`
