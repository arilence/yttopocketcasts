FROM docker.io/rust:1.65.0-bullseye as builder
# Create dummy rust project so we can cache crate dependencies in Docker
RUN USER=root cargo new app
WORKDIR /usr/src/app
COPY Cargo.toml ./Cargo.toml
COPY Cargo.lock ./Cargo.lock
# Needs at least a main.rs file with a main function
RUN mkdir src && echo "fn main(){}" > src/main.rs
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/src/app/target \
    cargo build --release
# Build our app
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/src/app/target \
    cargo install --offline --path .

FROM docker.io/debian:bullseye-slim
RUN apt-get install -y ca-certificates
RUN useradd -ms /bin/bash app
USER app
WORKDIR /app
COPY --from=builder /usr/local/cargo/bin/yttopocketcasts /app/botapp

# No CMD or ENTRYPOINT, see fly.toml with `cmd` override.
CMD ["/app/botapp"]
