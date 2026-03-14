.PHONY: all help build release clean run dev \
        test test-unit test-coverage \
        fmt format-check lint check \
        security-scan docs \
        ci-build ci-test ci-docs \
        docker-build docker-dev \
        version-sync version-bump

VERSION := $(shell cat VERSION)

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

# ── Build ───────────────────────────────────────────────

build: ## Debug build
	cargo build --workspace

release: ## Release build (optimized)
	cargo build --workspace --release

clean: ## Clean build artifacts
	cargo clean

# ── Run ─────────────────────────────────────────────────

run: ## Run main binary
	cargo run

dev: ## Run with auto-reload
	cargo watch -x run

# ── Test ────────────────────────────────────────────────

test: ## Run all tests
	cargo test --workspace

test-unit: ## Run unit tests only
	cargo test --workspace --lib

test-coverage: ## Run tests with coverage (65% threshold)
	cargo tarpaulin --workspace --fail-under 65 --out html

# ── Quality ─────────────────────────────────────────────

fmt: ## Format code
	cargo fmt --all

format-check: ## Check formatting
	cargo fmt --all -- --check

lint: ## Run clippy
	cargo clippy --workspace -- -D warnings

check: format-check lint test ## Full quality check

# ── Security ────────────────────────────────────────────

security-scan: ## Run cargo audit
	cargo audit

# ── Docs ────────────────────────────────────────────────

docs: ## Build documentation
	cargo doc --workspace --no-deps

# ── CI ──────────────────────────────────────────────────

ci-build: build ## CI build step

ci-test: test ## CI test step

ci-docs: docs ## CI docs step

# ── Docker ──────────────────────────────────────────────

docker-build: ## Build production container
	docker build -f docker/Dockerfile -t rasa:$(VERSION) .

docker-dev: ## Build dev container
	docker build -f docker/Dockerfile.dev -t rasa-dev:$(VERSION) .

# ── Version ─────────────────────────────────────────────

version-sync: ## Display current version
	@echo "Rasa v$(VERSION)"

version-bump: ## Bump version (usage: make version-bump V=2026.3.15)
	@test -n "$(V)" || (echo "Usage: make version-bump V=2026.3.15" && exit 1)
	@echo "$(V)" > VERSION
	@echo "Version bumped to $(V)"
