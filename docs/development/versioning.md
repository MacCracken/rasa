# Versioning

Rasa follows date-based versioning: `YYYY.M.DD` or `YYYY.M.DD-N`

- **YYYY** — year
- **M** — month (no leading zero)
- **DD** — day
- **-N** — optional same-day patch number (e.g. `-1`, `-2`)

Examples:
- `2026.3.13` = March 13, 2026
- `2026.3.13-1` = second release on March 13, 2026

The version is the source of truth in the `VERSION` file at the repo root. The bump script updates `VERSION`, `Cargo.toml`, `Cargo.lock`, `.agnos-agent.json`, and `docs/development/roadmap.md` automatically.

To bump: `make version-bump V=2026.3.18` or `./bump-version.sh 2026.3.18`
