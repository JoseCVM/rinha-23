
FROM rust as builder
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM debian:buster-slim
RUN apt-get update && apt-get install -y libssl1.1
COPY --from=builder /app/target/release/api-rinha /usr/local/bin/
EXPOSE 8000

COPY db.sql /

CMD ["api-rinha"]
