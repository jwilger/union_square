#!/bin/bash
# Setup TDD-specific git hooks

echo "Setting up TDD git hooks..."

# Set the git hooks path
git config core.hooksPath .githooks

echo "TDD git hooks enabled!"
echo ""
echo "The prepare-commit-msg hook will now:"
echo "- Suggest 'test:' prefix when only test files are changed"
echo "- Suggest 'feat:', 'fix:', or 'refactor:' when both tests and source are changed"
echo "- Warn when source files are changed without tests"
echo ""
echo "Remember the TDD cycle:"
echo "1. RED: Write failing test first (commit with 'test:' prefix)"
echo "2. GREEN: Make test pass (commit with 'feat:' or 'fix:' prefix)"
echo "3. REFACTOR: Improve code (commit with 'refactor:' prefix)"
