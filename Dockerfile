FROM rust:slim-trixie AS builder
RUN apt-get update && apt-get install -y pkg-config libdbus-1-dev libssl-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /playlist-randomizer
COPY Cargo.toml Cargo.lock .
COPY src/ src/
RUN cargo install --path .

FROM debian:trixie-slim
RUN apt-get update && apt-get install -y libdbus-1-3 ca-certificates && \
    update-ca-certificates && \
    rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/playlist-randomizer /usr/local/bin/playlist-randomizer
CMD ["playlist-randomizer"]
