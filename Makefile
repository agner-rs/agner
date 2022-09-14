
all:

fmt:
	cargo +nightly fmt

doc:
	cargo doc

build-release:
	cargo build --release

build-debug:
	cargo build

test-release:
	cargo nextest run --release

test-debug:
	cargo nextest run

