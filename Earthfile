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

pack:
	FROM ubuntu:latest

	COPY +build/algorithm ./server
	COPY sql ./sql
	COPY scripts/release.sh /app/endpoint.sh

	RUN rm -fr ./assets

	RUN apt update                                                          && \
	    apt install -y postgresql-client                                    && \
	    apt install -y ca-certificates curl unzip screen

	RUN curl -O https://localtonet.com/download/localtonet-linux-x64.zip   	&& \
		unzip localtonet-linux-x64.zip	                                && \
		chmod 755 ./localtonet	                                        && \
		cp ./localtonet /usr/bin/localtonet				&& \
		rm localtonet-linux-x64.zip

	ENTRYPOINT ["/app/endpoint.sh", "/app/server", "/sql"]
 	EXPOSE 8000
	LABEL org.opencontainers.image.source="https://github.com/YOUR_ORG/YOUR_REPO"
	LABEL org.opencontainers.image.description="Algorithm HTTP Server"
	SAVE IMAGE algorithm:latest

release:
    FROM docker
    RUN docker build -t $DOCKERHUB_USERNAME/algorithm:$TAG .
    RUN docker push $DOCKERHUB_USERNAME/algorithm:$TAG
