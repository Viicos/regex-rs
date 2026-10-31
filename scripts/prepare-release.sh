#!/usr/bin/env bash
# Prepare a release: bump the version, build the changelog, open the release PR and create
# a draft GitHub release holding the release notes.
#
# Usage: scripts/prepare-release.sh <version>  (e.g. `0.2.0`, without the `v` prefix)
#
# Release process:
# 1. Run this script from an up-to-date `main` branch.
# 2. Review (and optionally edit) the release PR and the draft GitHub release.
# 3. Merge the PR, then tag the merge commit and push the tag:
#    `git switch main && git pull && git tag v<version> && git push origin v<version>`
# 4. CI publishes to PyPI, attaches the sdist to the draft GitHub release, and publishes it.

set -euo pipefail
cd "$(dirname "$0")/.."

if [ $# -ne 1 ]; then
    echo "Usage: $0 <version>" >&2
    exit 1
fi
version=$1
tag="v$version"
branch="release/$tag"

if ! [[ $version =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
    echo "Invalid version '$version' (expected e.g. 0.2.0)" >&2
    exit 1
fi

if [ "$(git branch --show-current)" != main ]; then
    echo "Not on the main branch" >&2
    exit 1
fi
if [ -n "$(git status --porcelain --untracked-files=no)" ]; then
    echo "The working tree has uncommitted changes" >&2
    exit 1
fi
git fetch --quiet origin main --tags
if [ "$(git rev-parse HEAD)" != "$(git rev-parse origin/main)" ]; then
    echo "The local main branch is not in sync with origin/main" >&2
    exit 1
fi
if git rev-parse --quiet --verify "refs/tags/$tag" >/dev/null; then
    echo "Tag $tag already exists" >&2
    exit 1
fi
if gh release view "$tag" >/dev/null 2>&1; then
    echo "A GitHub release (possibly a draft) already exists for $tag" >&2
    exit 1
fi

uv run --quiet rtfc check

notes=$(mktemp)
trap 'rm -f "$notes"' EXIT
# Must run before `rtfc build`, which deletes the changelog entries:
uv run --quiet rtfc export github-markdown --version "$version" > "$notes"

# Bump the version (sed until we get https://github.com/rust-lang/cargo/issues/6583):
sed -i.bak -E "1,/^version = /s/^version = \".*\"/version = \"$version\"/" Cargo.toml
rm Cargo.toml.bak
cargo update --workspace --offline --quiet

uv run --quiet rtfc build --version "$version"

git switch --quiet --create "$branch"
git add --all Cargo.toml Cargo.lock docs/source/changelog.rst changelog/
git commit --quiet --message "Prepare release $tag"
git push --quiet --set-upstream origin "$branch"

gh pr create --base main --head "$branch" --title "Prepare release $tag" --body-file "$notes"
gh release create "$tag" --draft --target main --title "$tag" --notes-file "$notes"
