
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
RUN apt update 																	&& \
    apt install -y postgresql-client ca-certificates curl unzip screen supervisor  gettext-base							&& \
    curl -kO https://localtonet.com/download/localtonet-linux-x64.zip 										&& \
    unzip localtonet-linux-x64.zip 														&& \
    chmod +x ./localtonet 															&& \
    mv ./localtonet /usr/bin/localtonet 													&& \
    rm localtonet-linux-x64.zip 														&& \
    curl -fsSL https://apt.grafana.com/gpg.key | gpg --dearmor -o /usr/share/keyrings/grafana.gpg 						&& \
    echo "deb [signed-by=/usr/share/keyrings/grafana.gpg] https://apt.grafana.com stable main" | tee /etc/apt/sources.list.d/grafana.list 	&& \
    apt update 																	&& \
    apt install -y grafana-agent 														&& \
    apt autoremove -y && apt clean

# Create supervisor configuration directory
RUN mkdir -p /etc/supervisor/conf.d

# Copy supervisor configuration files
COPY supervisord.conf /etc/supervisor/supervisord.conf

# Create directory for Grafana Agent configuration
RUN mkdir -p /etc/grafana-agent

# Copy Grafana Agent configuration
COPY grafana-agent.yaml /etc/grafana-agent/config.yaml.shenv

ENTRYPOINT ["/app/endpoint.sh", "/usr/bin/supervisord", "/sql"]
EXPOSE 8000
