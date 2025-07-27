#!/bin/bash
# Setup script for netwatch git hooks
# This script installs pre-commit hooks to ensure code quality

set -e

echo "🔧 Setting up git hooks for netwatch..."

# Get the repository root
REPO_ROOT=$(git rev-parse --show-toplevel)
HOOKS_DIR="$REPO_ROOT/.githooks"
GIT_HOOKS_DIR="$REPO_ROOT/.git/hooks"

# Check if we're in a git repository
if [ ! -d "$REPO_ROOT/.git" ]; then
    echo "❌ Not in a git repository"
    exit 1
fi

# Create .git/hooks directory if it doesn't exist
mkdir -p "$GIT_HOOKS_DIR"

# Install pre-commit hook
if [ -f "$HOOKS_DIR/pre-commit" ]; then
    echo "📋 Installing pre-commit hook..."
    cp "$HOOKS_DIR/pre-commit" "$GIT_HOOKS_DIR/pre-commit"
    chmod +x "$GIT_HOOKS_DIR/pre-commit"
    echo "✅ Pre-commit hook installed"
else
    echo "❌ Pre-commit hook not found at $HOOKS_DIR/pre-commit"
    exit 1
fi

# Install commit-msg hook
if [ -f "$HOOKS_DIR/commit-msg" ]; then
    echo "📋 Installing commit-msg hook..."
    cp "$HOOKS_DIR/commit-msg" "$GIT_HOOKS_DIR/commit-msg"
    chmod +x "$GIT_HOOKS_DIR/commit-msg"
    echo "✅ Commit-msg hook installed"
else
    echo "❌ Commit-msg hook not found at $HOOKS_DIR/commit-msg"
    exit 1
fi

# Test the hooks
echo "🧪 Testing hooks..."
if [ -x "$GIT_HOOKS_DIR/pre-commit" ]; then
    echo "✅ Pre-commit hook is executable"
else
    echo "❌ Pre-commit hook is not executable"
    exit 1
fi

if [ -x "$GIT_HOOKS_DIR/commit-msg" ]; then
    echo "✅ Commit-msg hook is executable"
else
    echo "❌ Commit-msg hook is not executable"
    exit 1
fi

echo "🎉 Git hooks setup complete!"
echo ""
echo "The following hooks are now active:"
echo "  - pre-commit: Runs cargo fmt, clippy, and tests"
echo "  - commit-msg: Enforces conventional commit message format"
echo ""
echo "Conventional commit format:"
echo "  feat: add new feature"
echo "  fix: bug fix"
echo "  docs: documentation changes"
echo "  style: code formatting"
echo "  refactor: code refactoring"
echo "  perf: performance improvements"
echo "  test: adding tests"
echo "  build: build system changes"
echo "  ci: CI/CD changes"
echo "  chore: maintenance tasks"
echo ""
echo "To bypass hooks temporarily, use: git commit --no-verify"