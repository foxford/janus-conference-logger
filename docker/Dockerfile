FROM rust:1.46.0-slim-buster
WORKDIR /build

# Install dependencies
RUN apt update && apt install -y --no-install-recommends \
  libjansson-dev

# Install and build crates
COPY Cargo.* /build/
RUN mkdir /build/src && touch /build/src/lib.rs
RUN cargo build --release

# Build lib
COPY src/ /build/src/
RUN touch src/lib.rs && cargo build --release
