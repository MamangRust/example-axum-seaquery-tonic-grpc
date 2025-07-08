run-client:
	cargo run --package seaquery_client

run-server:
	cargo run --package seaquery_server

build-client:
	cargo build --release --target x86_64-unknown-linux-musl --package seaquery_client

build-server:
	cargo build --release --target x86_64-unknown-linux-musl --package seaquery_server

up:
	docker compose up -d

down:
	docker compose down

clipy:
	cargo clippy --all-targets --all-features -- -D warnings

fmt:
	cargo fmt --all -- --check