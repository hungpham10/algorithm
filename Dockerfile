
# Build stage
FROM rustlang/rust:nightly AS build

WORKDIR /app

# Copy source code and build
COPY . .
RUN make server

# Release stage
FROM rustlang/rust:nightly

WORKDIR /app
COPY --from=build /app/target/release/algorithm ./server
COPY sql ./sql
COPY scripts/release.sh /app/endpoint.sh

# Install runtime dependencies
RUN apt update && \
    apt install -y postgresql-client ca-certificates curl unzip screen && \
    curl -kO https://localtonet.com/download/localtonet-linux-x64.zip && \
    unzip localtonet-linux-x64.zip && \
    chmod +x ./localtonet && \
    mv ./localtonet /usr/bin/localtonet && \
    rm localtonet-linux-x64.zip && \
    apt autoremove -y && apt clean

ENTRYPOINT ["/app/endpoint.sh", "./server", "/sql"]
EXPOSE 8000
