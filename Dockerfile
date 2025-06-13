FROM rust:1.80-bookworm as builder

WORKDIR /app

COPY . .

RUN cargo build --release

FROM rust:1.80-bookworm as runtime

COPY --from=builder /app/target/release/pembantu_telegram .

CMD ["./pembantu_telegram"]
