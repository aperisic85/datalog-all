# ============================================================
# Stage 1: Build
# ============================================================
FROM rust:1.88-alpine AS builder

RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig

WORKDIR /app

# Cache dependencies first
COPY Cargo.toml Cargo.lock ./
# `crates/` su workspace članovi (aton_decode) — moraju postojati već ovdje,
# inače cargo ne može učitati manifest workspacea ni u ovom cache koraku.
COPY crates ./crates
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release 2>/dev/null || true

# Build full app
COPY src ./src
COPY migrations ./migrations
# .sqlx contains pre-generated query metadata from `cargo sqlx prepare`
# allowing offline builds without a live DATABASE_URL
COPY .sqlx ./.sqlx
ENV SQLX_OFFLINE=true
RUN touch src/main.rs && cargo build --release

# ============================================================
# Stage 2: Runtime (minimal image)
# ============================================================
FROM alpine:3.19 AS runtime

RUN apk add --no-cache ca-certificates curl libgcc

WORKDIR /app

# Copy binary
COPY --from=builder /app/target/release/datalogger-backend /app/datalogger-backend

# Copy migrations (sqlx::migrate! embeds them at compile time, but keep for reference)
COPY migrations ./migrations

EXPOSE 8095

CMD ["/app/datalogger-backend"]
