# Build stage
FROM rustlang/rust:nightly AS build

WORKDIR /app
COPY . .
RUN apt update && \
    apt install -y protobuf-compiler && \
    cd ./backend && \
    cargo +nightly build --release

# Release stage
FROM ubuntu:latest

WORKDIR /app
COPY --from=build /app/target/release/algorithm ./server
COPY sql ./sql
COPY scripts/release.sh /app/endpoint.sh

RUN rm -fr ./assets

RUN apt update && \
    apt install -y postgresql-client ca-certificates curl unzip screen && \
    curl -kO https://localtonet.com/download/localtonet-linux-x64.zip && \
    unzip localtonet-linux-x64.zip && \
    chmod 777 ./localtonet && \
    cp ./localtonet /usr/bin/localtonet

ENTRYPOINT ["/app/endpoint.sh", "./server", "/sql"]
EXPOSE 8000
