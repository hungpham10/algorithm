
# Build stage
FROM rustlang/rust:nightly-alpine AS build

WORKDIR /app

# Copy source code and build
COPY . .
RUN apk add make pkgconf musl-dev openssl-dev openssl-libs-static
RUN make server

# Release stage
FROM tailscale/tailscale:latest

WORKDIR /app
COPY --from=build /app/target/release/algorithm ./server
COPY sql ./sql
COPY scripts/release.sh /app/endpoint.sh

# Install runtime dependencies
RUN apk add ca-certificates supervisor nginx mysql-client

# Create supervisor configuration directory
RUN mkdir -p /etc/supervisor.d

# Copy supervisor configuration files
COPY conf/supervisor/*.ini /etc/supervisor.d/

# Copy Nginx configuration
COPY conf/nginx/www.conf /etc/nginx/http.d/default.conf

ENTRYPOINT ["/app/endpoint.sh", "/usr/bin/supervisord", "/sql", "-n"]
EXPOSE 8080
