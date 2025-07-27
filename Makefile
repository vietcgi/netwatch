# Makefile for netwatch development

.PHONY: help setup build test fmt clippy clean install hooks dev release check all

# Default target
help: ## Show this help message
	@echo "🦀 Netwatch Development Commands"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

setup: ## Initial setup for development
	@echo "🔧 Setting up development environment..."
	@cargo build
	@./scripts/setup-hooks.sh
	@echo "✅ Setup complete!"

build: ## Build the project
	@echo "🔨 Building netwatch..."
	@cargo build

release: ## Build release version
	@echo "🚀 Building release version..."
	@cargo build --release

test: ## Run all tests
	@echo "🧪 Running tests..."
	@cargo test --all

fmt: ## Format code
	@echo "📝 Formatting code..."
	@cargo fmt --all

clippy: ## Run clippy linting
	@echo "🔍 Running clippy..."
	@cargo clippy --all-targets --all-features -- -D warnings

clean: ## Clean build artifacts
	@echo "🧹 Cleaning build artifacts..."
	@cargo clean

install: ## Install netwatch locally
	@echo "📦 Installing netwatch..."
	@cargo install --path .

hooks: ## Install git hooks
	@echo "🪝 Installing git hooks..."
	@./scripts/setup-hooks.sh

dev: fmt clippy test ## Run development checks (fmt, clippy, test)

check: ## Run all quality checks
	@echo "✅ Running all quality checks..."
	@make fmt
	@make clippy
	@make test
	@echo "🎉 All checks passed!"

all: clean build test ## Clean, build, and test

# Development workflow
watch: ## Watch for changes and run tests
	@echo "👀 Watching for changes..."
	@cargo watch -x test

run: ## Run netwatch in development mode
	@echo "🏃 Running netwatch..."
	@cargo run

# Release workflow
prepare-release: check ## Prepare for release (run all checks)
	@echo "📋 Preparing for release..."
	@make check
	@echo "🚀 Ready for release!"