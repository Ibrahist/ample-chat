.PHONY: all fmt lint test check coverage build release docker clean audit deny geiger

all: fmt lint test

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

lint:
	cargo clippy --all-targets --all-features -- -D warnings

test:
	cargo test --workspace

coverage:
	cargo llvm-cov --workspace --all-features --lcov --output-path lcov.info

build:
	cargo build --workspace

release:
	cargo build --workspace --release

audit:
	cargo audit

deny:
	cargo deny check

geiger:
	cargo geiger

docker:
	docker build -t ample-chat:latest .

clean:
	cargo clean
