
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
	cargo nextest run --release --no-fail-fast

test-debug:
	cargo nextest run --no-fail-fast

clean:
	cargo clean
