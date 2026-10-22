# syntax=docker/dockerfile:1
FROM node:22-slim AS web
RUN corepack enable
WORKDIR /w
COPY web ./web
COPY crates/proto/fixtures ./crates/proto/fixtures
WORKDIR /w/web
ENV SHARU_BASE=/
RUN pnpm install --frozen-lockfile && pnpm build

FROM rust:1-slim AS server
WORKDIR /s
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo build --release -p sharu-server

FROM debian:stable-slim
RUN useradd -r -u 10001 sharu
COPY --from=server /s/target/release/sharu-server /usr/local/bin/sharu-server
COPY --from=web /w/web/dist /srv/www
USER sharu
ENV BIND=0.0.0.0:8080 STATIC_DIR=/srv/www
EXPOSE 8080
CMD ["sharu-server"]
