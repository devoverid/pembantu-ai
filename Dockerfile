FROM --platform=linux/amd64 rust:1.80-bookworm as builder

WORKDIR /app

COPY . .

RUN cargo build --release

FROM --platform=linux/amd64 rust:1.80-bookworm as runtime

COPY --from=builder /app/target/release/pembantu_telegram .

CMD ["./pembantu_telegram"]
