#!/usr/bin/env bash
# Regenerate `python/rustregex/_rustregex.pyi` from the signatures and doc comments of the Rust code.

set -euo pipefail
cd "$(dirname "$0")/.."

# Keep in sync with `requires-python` in `pyproject.toml`.
python_version=3.11

stubs=python/rustregex/_rustregex.pyi
uv python install --quiet "$python_version"
uv run maturin generate-stubs --features stubs --out python --quiet \
    --interpreter "$(uv python find "$python_version")"
uv run ruff check --quiet --fix --select W605,UP037,I002 \
    --config 'lint.isort.required-imports = ["import collections.abc"]' "$stubs"
uv run ruff format --quiet "$stubs"
