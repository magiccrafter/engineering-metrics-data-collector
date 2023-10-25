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

RUN cargo install --path .

FROM gcr.io/distroless/cc

# Import from build.
COPY --from=build /etc/passwd /etc/passwd
COPY --from=build /etc/group /etc/group

WORKDIR /app
COPY --from=build /usr/local/cargo/bin/engineering-metrics-data-collector /usr/local/bin/engineering-metrics-data-collector
USER appuser:appuser
CMD ["engineering-metrics-data-collector"]
