# Versioning

Rasa follows date-based versioning: `YYYY.M.DD` or `YYYY.M.DD-N`

- **YYYY** — year
- **M** — month (no leading zero)
- **DD** — day
- **-N** — optional same-day patch number (e.g. `-1`, `-2`)

Examples:
- `2026.3.13` = March 13, 2026
- `2026.3.13-1` = second release on March 13, 2026

The version is the source of truth in the `VERSION` file at the repo root. `Cargo.toml` workspace version is updated automatically by the bump script.

To bump: `make version-bump V=2026.3.15` or `make version-bump V=2026.3.15-1`
