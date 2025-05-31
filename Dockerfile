
# Build stage
FROM deepnote/python:3.11 AS build

WORKDIR /app

# Install Rust and dependencies
RUN apt update && \
    apt install -y protobuf-compiler curl && \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly && \
    apt autoremove -y && apt clean

# Set Rust environment
ENV PATH="/root/.cargo/bin:$PATH"

# Verify Python library
RUN ldconfig -p | grep libpython3.11

# Cache dependencies
COPY ./backend/Cargo.toml ./backend/Cargo.lock ./backend/
RUN cd ./backend && cargo +nightly fetch

# Copy source code and build
COPY . .
RUN cd ./backend && \
    PYO3_PYTHON=/usr/local/bin/python3.11 \
    LD_LIBRARY_PATH=/usr/local/lib \
    cargo +nightly build --release --verbose

# Release stage
FROM deepnote/python:3.11

WORKDIR /app
COPY --from=build /app/target/release/algorithm ./server
COPY sql ./sql
COPY scripts/release.sh /app/endpoint.sh

# Remove unnecessary assets
RUN rm -rf ./assets

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
