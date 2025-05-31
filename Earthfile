VERSION 0.8

# Base image for all targets
FROM rustlang/rust:nightly
WORKDIR /app

build:
    # Use the same base image as specified
    FROM rustlang/rust:nightly

    # Cache dependencies
    COPY ./backend/Cargo.toml ./backend/Cargo.lock ./backend/
    RUN cd ./backend && cargo +nightly fetch

    # Copy source code and build
    COPY . .
    RUN make server

    # Save the compiled binary as an artifact
    SAVE ARTIFACT ./target/release/algorithm AS LOCAL ./algorithm

pack:
    FROM ubuntu:22.04  # Use ubuntu:22.04 instead of latest for stability
    WORKDIR /app

    # Copy the compiled binary from the build stage
    COPY +build/algorithm ./server
    COPY ./sql ./sql
    COPY ./scripts/release.sh ./endpoint.sh

    # Remove unnecessary assets
    RUN rm -rf ./assets

    # Install runtime dependencies and clean up
    RUN apt update && \
        apt install -y postgresql-client ca-certificates curl unzip screen python3.11 libpython3.11 && \
        update-alternatives --install /usr/bin/python3 python3 /usr/bin/python3.11 1 && \
        apt autoremove -y && apt clean

    # Install localtonet
    RUN curl -O https://localtonet.com/download/localtonet-linux-x64.zip && \
        unzip localtonet-linux-x64.zip && \
        chmod +x ./localtonet && \
        mv ./localtonet /usr/bin/localtonet && \
        rm localtonet-linux-x64.zip

    # Set the entrypoint and expose port
    ENTRYPOINT ["/app/endpoint.sh", "./server", "/sql"]
    EXPOSE 8000

    # Add image labels
    LABEL org.opencontainers.image.source="https://github.com/YOUR_ORG/YOUR_REPO"
    LABEL org.opencontainers.image.description="Algorithm HTTP Server"

    # Save the final image
    SAVE IMAGE algorithm:latest

release:
    FROM docker
    WORKDIR /app

    # Copy the built image from the pack target
    COPY +pack/algorithm:latest .

    # Build and push to Docker Hub
    ARG DOCKERHUB_USERNAME
    ARG TAG=latest
    RUN --push docker tag algorithm:latest $DOCKERHUB_USERNAME/algorithm:$TAG
    RUN --push docker push $DOCKERHUB_USERNAME/algorithm:$TAG
