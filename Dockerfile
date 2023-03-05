ARG RUST_VERSION=1.67.1-bullseye
ARG DEBIAN_VERSION=bullseye-slim

FROM docker.io/rust:$RUST_VERSION as builder
RUN DEBIAN_FRONTEND=noninteractive apt-get update && \
    apt-get install -y \
        ca-certificates \
        ffmpeg \
        gcc \
        git \
        libc6-dev \
        libssl-dev \
        pkg-config \
        python3 \
        python3-pip \
        && \
    rm -rf /var/lib/apt/lists/* && \
    apt-get clean
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

FROM builder as yt-dlp
RUN DEBIAN_FRONTEND=noninteractive apt-get update && \
    apt-get install -y curl python3 python3-pip && \
    rm -rf /var/lib/apt/lists/* && \
    apt-get clean
# Latest yt-dlp release is broken, temporarily using unofficial daily build of master branch to fix it.
# RUN curl -L https://github.com/yt-dlp/yt-dlp/releases/download/2023.02.17/yt-dlp -o /usr/local/bin/yt-dlp
RUN curl -L https://github.com/ytdl-patched/yt-dlp/releases/download/2023.03.01.19419/yt-dlp -o /usr/local/bin/yt-dlp
RUN chmod +x /usr/local/bin/yt-dlp

FROM builder as development
COPY --from=yt-dlp /usr/local/bin/yt-dlp /usr/local/bin/yt-dlp
WORKDIR /app
RUN rustup component add rustfmt
RUN mkdir /tmp/.cache
#
# The Rust docker image defaults to using root as the image user.
#
# This causes permission issues when mounting files from the host OS.
# devcontainer's work around `updateRemoteUserUID` sets the PID and GID of the
# container user to match the host user.
#
# However, `updateRemoteUserUID` requires a non-root container user.
#
# So here we create a new user and make sure they have permissions to cargo files.
#
RUN useradd --system --create-home --no-log-init --shell /bin/bash devuser
RUN chmod -R a+w $RUSTUP_HOME $CARGO_HOME /tmp/.cache
USER devuser

FROM docker.io/debian:$DEBIAN_VERSION
RUN DEBIAN_FRONTEND=noninteractive apt-get update && \
    apt-get install -y \
        ca-certificates \
        python3 \
        ffmpeg \
        && \
    rm -rf /var/lib/apt/lists/* && \
    apt-get clean
COPY --from=yt-dlp /usr/local/bin/yt-dlp /usr/local/bin/yt-dlp
WORKDIR /app
RUN mkdir /tmp/.cache
RUN useradd -ms /bin/bash app
USER app
COPY --from=builder /usr/local/cargo/bin/yttopocketcasts /app/botapp

# No CMD or ENTRYPOINT, see fly.toml with `cmd` override.
CMD ["/app/botapp"]
