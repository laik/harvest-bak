linux:
	CROSS_COMPILE=x86_64-linux-musl- cargo build --release --target x86_64-unknown-linux-musl

run:
	RUST_BACKTRACE=full cargo run -- --namespace finance-dev --docker-dir /Users/dxp/workspace/go/src/github.com/laik/harvest/tmp --api-server http://localhost:9999/ --host node1
