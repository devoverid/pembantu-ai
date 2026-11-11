telegram:
	RUST_LOG=trace cargo watch -c -w crates --exec "run -p siti_telegram"
build:
	cargo build --all
prod:
	./target/release/siti_telegram
.PHONY:
	telegram build