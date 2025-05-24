# Makefile for gmux-rs

.PHONY: all build test fmt check clean lint watch install docs

all: build

build:
	cargo build

install:
	cargo watch -x "install --path ."

test:
	cargo test

fmt:
	cargo fmt --all
	cargo clippy --fix --allow-dirty

check:
	cargo check

lint:
	cargo clippy -- -D warnings

watch:
	cargo watch -x check -x test -x fmt

docs:
	cargo doc --no-deps --document-private-items

clean:
	cargo clean
	rm -rf target/
	rm -rf **/*.rs.bk 