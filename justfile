
# List available recipes.
help:
	@just -l

# Run all tests
test:
	@cargo test

# Lint and stuff.
ci:
	@cargo fmt
	@cargo clippy
	@cargo test

# Build docs and open them in your browser.
docs:
	@cargo doc --no-deps --open

# Install the tool into .cargo/bin
install:
	@cargo install --path .

# Set the crate version and tag the repo to match.
tag VERSION:
	#!/usr/bin/env bash
	version="{{VERSION}}"
	version=${version/v/}
	status=$(git status --porcelain)
	if [ "$status" != ""  ]; then
		echo "There are uncommitted changes! Cowardly refusing to act."
		exit 1
	fi
	sed -i "s/^version = \".*\"/version = \"$version\"/" Cargo.toml
	# update the lock file
	cargo check
	git commit Cargo.toml Cargo.lock -m "${version}"
	git tag "${version}"
	echo "Release tagged for version ${version}"
