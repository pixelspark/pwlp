FROM rust:latest as builder
WORKDIR /usr/src/app
COPY . .
RUN cargo build --release

FROM debian:buster
WORKDIR /pwlp
COPY --from=builder /usr/src/app/target/release/pwlp /pwlp/pwlp
CMD ["/pwlp/pwlp"]

