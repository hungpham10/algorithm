VERSION 0.7
FROM rust:latest
WORKDIR /app

build-backend:
	FROM rust:latest

	COPY . .
	RUN cd ./backend && cargo build --release

	SAVE ARTIFACT ./target/release/algorithm AS LOCAL algorithm

build-frontend:
	FROM node:23.0.0-bullseye

	COPY . .
	RUN cd ./frontend && npm install -g pnpm
	RUN cd ./frontend && pnpm i
	RUN cd ./frontend && pnpm run build

	SAVE ARTIFACT ./frontend/dist AS LOCAL dist

server-release:
	FROM ubuntu:latest

	COPY +build-backend/algorithm ./server
	COPY +build-frontend/dist ./static
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

	ENTRYPOINT ["/app/endpoint.sh", "./server", "/sql", "server"]
	EXPOSE 3000
	SAVE IMAGE algorithm:latest

release:
	BUILD +build-backend
	BUILD +build-frontend
	BUILD +server-release
