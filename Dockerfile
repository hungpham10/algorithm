FROM rust:latest AS build-backend
WORKDIR /app

COPY . .
RUN cd ./backend && cargo build --release
################################################################################

FROM node:23.0.0-bullseye AS build-frontend
WORKDIR /app

COPY . .
RUN cd ./frontend && npm install -g pnpm
RUN cd ./frontend && pnpm i
RUN cd ./frontend && pnpm run build

################################################################################
FROM ubuntu:latest

COPY --from=build-backend /app/backend/target/release/algorithm ./server
COPY --from=build-frontend /app/frontend/dist ./static
COPY assets assets
COPY sql/system ./sql
COPY scripts/release.sh /app/endpoint.sh

RUN cp -av ./assets/honeygain/$(uname -m)/honeygain ./honeygain || true
RUN cp -av ./assets/honeygain/$(uname -m)/libhg.so.* /usr/lib/ || true
RUN cp -av ./assets/honeygain/$(uname -m)/libmsquic.so.* /usr/lib/ || true
RUN rm -fr ./assets

RUN apt update                          && \
    apt install -y postgresql-client    && \
    apt install -y ca-certificates

ENTRYPOINT ["/app/endpoint.sh", "./server", "/sql", "3000", "sserver"]
EXPOSE 3000
