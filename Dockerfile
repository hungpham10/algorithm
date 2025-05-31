# Build stage
FROM rustlang/rust:nightly AS build

WORKDIR /app
COPY . .

# Install dependencies, including Python development libraries
RUN apt update && \
    apt install -y protobuf-compiler python3-dev && \
    cd ./backend && \
    cargo +nightly build --release

# Release stage
FROM ubuntu:latest

WORKDIR /app
COPY --from=build /app/target/release/algorithm ./server
COPY sql ./sql
COPY scripts/release.sh /app/endpoint.sh

RUN rm -fr ./assets

# Install runtime dependencies
RUN apt update && \
    apt install -y postgresql-client ca-certificates curl unzip screen python3 && \
    curl -kO https://localtonet.com/download/localtonet-linux-x64.zip && \
    unzip localtonet-linux-x64.zip && \
    chmod 777 ./localtonet && \
    cp ./localtonet /usr/bin/localtonet

ENTRYPOINT ["/app/endpoint.sh", "./server", "/sql"]
EXPOSE 8000
