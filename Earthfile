VERSION 0.7
FROM rust:latest
WORKDIR /app

deps:
	COPY . .

build:
	FROM +deps

	RUN cargo build --release

	SAVE ARTIFACT ./target/release/algorithm AS LOCAL algorithm

server-release:
	FROM ubuntu:latest

	COPY +build/algorithm .
	COPY sql/system ./sql
	COPY scripts/release.sh /app/endpoint.sh
	

	RUN apt update                          && \
	    apt install -y postgresql-client    && \
	    apt install -y ca-certificates

	ENTRYPOINT ["/app/endpoint.sh", "/algorithm", "/sql"]
	EXPOSE 3000
	SAVE IMAGE algorithm:latest

release:
	BUILD +server-release
