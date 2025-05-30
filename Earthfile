VERSION 0.7
FROM rustlang/rust:nightly
WORKDIR /app

build:
	FROM rustlang/rust:nightly

	COPY . .
	RUN apt update                       && \
	    apt install -y protobuf-compiler && \
	    cd ./backend                     && \
	    cargo +nightly build --release

	SAVE ARTIFACT ./target/release/algorithm AS LOCAL algorithm

release:
	FROM ubuntu:latest

	COPY +build/algorithm ./server
	COPY sql ./sql
	COPY scripts/release.sh /app/endpoint.sh

	RUN rm -fr ./assets

	RUN apt update                                                          && \
	    apt install -y postgresql-client                                    && \
	    apt install -y ca-certificates curl unzip screen

	RUN curl -kO https://localtonet.com/download/localtonet-linux-x64.zip   && \
		unzip localtonet-linux-x64.zip	                                && \
		chmod 755 ./localtonet	                                        && \
		cp ./localtonet /usr/bin/localtonet

	ENTRYPOINT ["/app/endpoint.sh", "./server", "/sql"]
	EXPOSE 8000
	SAVE IMAGE algorithm:latest

release:
	BUILD +build
	BUILD +release
