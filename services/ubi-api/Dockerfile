# ubi-api — build from the REPOSITORY ROOT so the path dependency on crates/ubiqx-core resolves:
#
#   docker build -f services/ubi-api/Dockerfile -t ubi-api .
#   docker run --rm -p 8080:8080 -e ANTHROPIC_API_KEY=... -e UBI_ADMIN_TOKEN=... ubi-api
#
# Stage 1 compiles a static-ish binary against glibc; stage 2 is distroless (libc + CA certs only).

FROM rust:1.98-bookworm AS build
WORKDIR /src
# The root manifest first: crates/ubiqx-core inherits its version, edition and dependency
# versions from the root `[workspace]` (the other members it lists need not be present).
COPY Cargo.toml Cargo.lock ./
# Then only what the service needs: the shared domain crate and the service itself.
COPY crates/ubiqx-core crates/ubiqx-core
COPY services/ubi-api services/ubi-api
ENV CARGO_TARGET_DIR=/src/target
RUN cargo build --release --manifest-path services/ubi-api/Cargo.toml \
    && cp /src/target/release/ubi-api /src/target/release/ubi-license /usr/local/bin/

FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=build /usr/local/bin/ubi-api /usr/local/bin/ubi-license /usr/local/bin/
# The SQLite ledger lives on a volume mounted here (see fly.toml).
ENV UBI_API_LISTEN=0.0.0.0:8080 \
    UBI_DB=/data/ubi-api.sqlite \
    RUST_LOG=info
VOLUME ["/data"]
EXPOSE 8080
USER nonroot
ENTRYPOINT ["/usr/local/bin/ubi-api"]
