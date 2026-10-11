FROM rust:alpine AS builder

WORKDIR /app

RUN apk add --no-cache musl-dev sqlite-dev sqlite-static openssl-dev openssl-libs-static pkgconfig build-base curl unzip

# Copy workspace cargo files
COPY Cargo.toml Cargo.lock ./

# Copy crates
COPY crates ./crates

# Download and extract official WebUI release into suwayomi-api static directory
RUN curl -L https://github.com/Suwayomi/Suwayomi-WebUI/releases/download/v20260929.01/Suwayomi-WebUI-v20260929.01.zip -o webui.zip && \
    unzip -o webui.zip -d crates/suwayomi-api/static && \
    rm webui.zip

# Test the workspace
RUN cargo test --workspace

# Build the workspace release binary
RUN cargo build --release --bin suwayomi-server

# Runtime stage
FROM alpine:3.20

WORKDIR /app

RUN apk add --no-cache libgcc sqlite-libs openssl ca-certificates

# Copy server binary
COPY --from=builder /app/target/release/suwayomi-server /usr/local/bin/suwayomi-server

# Copy config and migrations
COPY config ./config
COPY migrations ./migrations

EXPOSE 4567
VOLUME /data

ENV SUWAYOMI_DATA_DIR=/data
ENV CONFIG_FILE=/app/config/default.toml

CMD ["suwayomi-server"]
