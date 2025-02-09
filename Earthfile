VERSION 0.7
FROM rustlang/rust:nightly
WORKDIR /app

build-backend:
	FROM rustlang/rust:nightly

	COPY . .
	RUN apt update                       && \
	    apt install -y protobuf-compiler && \
	    cd ./backend                     && \
	    cargo +nightly build --release

	SAVE ARTIFACT ./target/release/bff AS LOCAL bff

build-frontend:
	FROM node:23.0.0-bullseye

	COPY . .
	RUN cd ./frontend && npm install -g pnpm
	RUN cd ./frontend && pnpm i
	RUN cd ./frontend && pnpm run build

	SAVE ARTIFACT ./frontend/dist AS LOCAL dist

graphql-server-release:
	FROM ubuntu:latest

	COPY +build-backend/bff ./server
	COPY +build-frontend/dist ./static
	COPY assets assets
	COPY sql/system ./sql
	COPY scripts/release.sh /app/endpoint.sh

	RUN cp -av ./assets/honeygain/$(uname -m)/honeygain ./honeygain || true
	RUN cp -av ./assets/honeygain/$(uname -m)/libhg.so.* /usr/lib/ || true
	RUN cp -av ./assets/honeygain/$(uname -m)/libmsquic.so.* /usr/lib/ || true
	RUN rm -fr ./assets

	RUN apt update                                                          && \
	    apt install -y postgresql-client                                    && \
	    apt install -y ca-certificates curl unzip screen

	ENTRYPOINT ["/app/endpoint.sh", "./server", "/sql", "3000"]
	EXPOSE 3000
	SAVE IMAGE bff-algorithm:latest


release:
	BUILD +build-backend
	BUILD +graphql-server-release
