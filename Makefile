SHELL := /bin/sh

.PHONY: fmt lint test

fmt:
	direnv exec . cargo fmt

lint:
	direnv exec . cargo clippy --all-targets --all-features -- -D warnings

test:
	direnv exec . cargo test
