telegram:
	RUST_LOG=trace cargo watch -c -w crates --exec "run -p pembantu_telegram"
.PHONY:
	telegram