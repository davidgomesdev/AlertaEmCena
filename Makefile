check:
	cargo fmt
	cargo clippy -- -D warnings

fmt:
	cargo fmt
	cargo clippy --fix --allow-dirty -- -D warnings
