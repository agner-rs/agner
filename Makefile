
all:

fmt:
	cargo +nightly fmt

build-release:
	cargo build --release

build-debug:
	cargo build

run-test-scenarios-release:
	cargo test --release -p test-scenarios-run -- --nocapture

run-test-scenarios-debug:
	cargo test -p test-scenarios-run -- --nocapture

