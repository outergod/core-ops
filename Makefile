SHELL := /bin/sh

.PHONY: fmt lint test

fmt:
	cargo fmt

lint:
	cargo clippy --all-targets --all-features -- -D warnings

test:
	cargo test
