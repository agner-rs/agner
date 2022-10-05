
all:

fmt:
	cargo +nightly fmt

doc:
	cargo doc --all-features

clippy:
	cargo clippy --all-features

build-release:
	cargo build --release

build-debug:
	cargo build

test-release:
	cargo nextest run --release

test-debug:
	cargo nextest run

