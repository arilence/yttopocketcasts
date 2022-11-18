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

FROM docker.io/debian:bullseye-slim as yt-dlp
RUN DEBIAN_FRONTEND=noninteractive apt-get update && \
    apt-get install -y curl && \
    rm -rf /var/lib/apt/lists/* && \
    apt-get clean
RUN curl -L https://github.com/yt-dlp/yt-dlp/releases/download/2022.11.11/yt-dlp -o /usr/local/bin/yt-dlp
RUN chmod +x /usr/local/bin/yt-dlp

FROM docker.io/debian:bullseye-slim
RUN DEBIAN_FRONTEND=noninteractive apt-get update && \
    apt-get install -y ca-certificates python3 ffmpeg && \
    rm -rf /var/lib/apt/lists/* && \
    apt-get clean
COPY --from=yt-dlp /usr/local/bin/yt-dlp /usr/local/bin/yt-dlp
RUN useradd -ms /bin/bash app
USER app
WORKDIR /app
RUN mkdir .cache
COPY --from=builder /usr/local/cargo/bin/yttopocketcasts /app/botapp

# No CMD or ENTRYPOINT, see fly.toml with `cmd` override.
CMD ["/app/botapp"]
