telegram:
	RUST_LOG=trace cargo watch -c -w crates --exec "run -p pembantu_telegram"
build:
	cargo build --all
prod:
	./target/release/pembantu_telegram
.PHONY:
	telegram build