.PHONY: run game build test fmt lint web web-build clean

run: game

game:
	cargo run -p game

build:
	cargo build --workspace

test:
	cargo test --workspace

fmt:
	cargo fmt --all

lint:
	cargo clippy --workspace --all-targets -- -D warnings

web:
	cd game && trunk serve

web-build:
	cd game && trunk build --release

clean:
	cargo clean
	rm -rf game/dist
