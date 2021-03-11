linux:
	CROSS_COMPILE=x86_64-linux-musl- cargo build --release --target x86_64-unknown-linux-musl

run:
	RUST_BACKTRACE=1 cargo run
