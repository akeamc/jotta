FROM clux/muslrust:1.59.0 as build-env
WORKDIR /app
COPY . .
RUN cargo build --release --bin jotta-rest

FROM gcr.io/distroless/static:nonroot
COPY --from=build-env --chown=nonroot:nonroot /app/target/x86_64-unknown-linux-musl/release/jotta-rest /
EXPOSE 8000
ENTRYPOINT ["/jotta-rest"]
