
FROM rust as builder
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM debian:buster-slim
COPY --from=builder /app/target/release/api-rinha /usr/local/bin/
EXPOSE 8000

CMD ["api-rinha"]
