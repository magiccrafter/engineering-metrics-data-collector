FROM rust:1.73 as build

# Create appuser
ENV USER=appuser
ENV UID=10001

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    "${USER}"

WORKDIR /app
COPY . .
# RUN rustup target add x86_64-unknown-linux-musl
# RUN apt-get update && apt-get install -y libssl-dev
# RUN rustup target add aarch64-apple-darwin
# RUN cargo install --path . --target aarch64-apple-darwin
RUN cargo install --path .
# RUN cargo install --path .
# RUN cargo build --release

# FROM alpine:3.18 AS runtime
# FROM debian:bullseye-slim
FROM gcr.io/distroless/cc
# RUN apt-get update && apt-get install -y extra-runtime-dependencies && rm -rf /var/lib/apt/lists/*
# RUN apt-get update && apt-get install -y libssl-dev

# Import from build.
COPY --from=build /etc/passwd /etc/passwd
COPY --from=build /etc/group /etc/group

WORKDIR /app
COPY --from=build /usr/local/cargo/bin/engineering-metrics-data-collector /usr/local/bin/engineering-metrics-data-collector
# COPY --from=build /app/target/release/engineering-metrics-data-collector ./
USER appuser:appuser
CMD ["engineering-metrics-data-collector"]
