# This image builds its own application binary from source code.

FROM rust:1.95-slim-bookworm AS builder
# From here, build the binary

WORKDIR /app

RUN apt-get update \
	&& apt-get install -y --no-install-recommends \
	ca-certificates \
	cmake \
	git \
	libssl-dev \
	pkg-config \
	&& rm -rf /var/lib/apt/lists/*

# Ensures sqlx will not complain about no accessible database
ENV SQLX_OFFLINE=true

COPY Cargo.toml Cargo.lock build.rs ./
COPY .sqlx ./.sqlx
COPY migrations ./migrations
COPY src ./src

RUN cargo build --release --locked

FROM debian:bookworm-slim AS runtime
# From here, run the binary

RUN apt-get update \
	&& apt-get install -y --no-install-recommends \
	ca-certificates \
	libssl3 \
	&& rm -rf /var/lib/apt/lists/* \
	&& groupadd --system --gid 10001 app \
	&& useradd --system --uid 10001 --gid app --home-dir /app --shell /usr/sbin/nologin app \
	&& mkdir -p /app \
	&& chown app:app /app

WORKDIR /app

COPY --from=builder /app/target/release/fixfiction /usr/local/bin/fixfiction

USER app

EXPOSE 7669

CMD ["/usr/local/bin/fixfiction"]
