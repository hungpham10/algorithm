ARG NGX_VERSION=1.27.1

# -----------------------------------------------------------------------
# --- Build proxy stage
# -----------------------------------------------------------------------
FROM rust:bookworm AS proxy
ARG NGX_VERSION
ARG NGX_CONFIGURE_ARGS=
WORKDIR /app

COPY . .
RUN apt-get -qq update
RUN DEBIAN_FRONTEND=noninteractive apt-get -qq install --yes --no-install-recommends --no-install-suggests 	\
	libclang-dev 												\
	libpcre2-dev 												\
	libssl-dev 												\
	zlib1g-dev 												\
	pkg-config 												\
	ca-certificates												\
	grep 													\
	gawk 													\
	gnupg2 													\
	sed 													\
	make													\
	wget
RUN make proxy

# -----------------------------------------------------------------------
# --- Build backend stage
# -----------------------------------------------------------------------
FROM rust:bookworm AS backend
WORKDIR /app

# Copy source code and build
COPY . .
RUN apt-get -qq update
RUN DEBIAN_FRONTEND=noninteractive apt-get -qq install --yes --no-install-recommends --no-install-suggests 	\
	make 													\
	pkgconf
RUN make server

# -----------------------------------------------------------------------
# --- Release stage
# -----------------------------------------------------------------------
FROM openresty/openresty:1.27.1.2-4-bookworm-fat
ARG TARGETARCH
ENV NGINX_DIR=/usr/local/openresty/nginx/conf
ENV SUPERVISOR_DIR=/etc/supervisor/conf.d

WORKDIR /app
RUN mkdir -p /app/secrets

COPY --from=backend /app/target/release/services ./aio
COPY --from=proxy /app/target/release/libproxy.so /usr/local/openresty/nginx/modules/libproxy.so
COPY secrets* /app/secrets/
COPY sql /sql
COPY scripts/nginx.sh /app/nginx.sh
COPY scripts/release.sh /app/endpoint.sh

# Install runtime dependencies
RUN apt update && apt install --no-install-recommends -y supervisor curl git gettext-base default-mysql-client tor gnupg2 && 			\
    mkdir -p /etc/apt/keyrings/ && 														\
    curl -fsSL https://apt.grafana.com/gpg.key | gpg --dearmor -o /etc/apt/keyrings/grafana.gpg && 						\
    echo "deb [signed-by=/etc/apt/keyrings/grafana.gpg] https://apt.grafana.com stable main" | tee /etc/apt/sources.list.d/grafana.list && 	\
    apt-get update && apt-get install -y grafana-agent && 											\
    if [ "$TARGETARCH" = "amd64" ]; then SOPS_ARCH="amd64"; 											\
    elif [ "$TARGETARCH" = "arm64" ]; then SOPS_ARCH="arm64"; 											\
    fi && 																	\
    curl -LO "https://github.com/getsops/sops/releases/download/v3.12.2/sops_3.12.2_${SOPS_ARCH}.deb" && 					\
    dpkg -i "sops_3.12.2_${SOPS_ARCH}.deb" && 													\
    rm "sops_3.12.2_${SOPS_ARCH}.deb" && 													\
    apt-get clean && rm -rf /var/lib/apt/lists/*

# Create supervisor configuration directory
RUN mkdir -p /etc/supervisor/conf.d

# Copy supervisor configuration files
COPY conf/torrc /etc/tor/torrc

# Copy supervisor configuration files
COPY conf/supervisor/*.conf /etc/supervisor/conf.d/

# Copy grafana-agent.yaml
COPY conf/grafana-agent.yaml /etc/grafana-agent.yaml

# Copy Nginx configuration
COPY conf/nginx/http.conf /usr/local/openresty/nginx/conf/nginx.conf
COPY conf/nginx/www.conf /usr/local/openresty/nginx/conf/http.d/default.conf
COPY conf/nginx/map /usr/local/openresty/nginx/conf/map.d
COPY conf/nginx/vhost /usr/local/openresty/nginx/conf/http.d/vhost

# Setup openresty modules
RUN if git clone https://github.com/zmartzone/lua-resty-openidc.git /tmp/openidc; then 				\
    cp -av /tmp/openidc/lib/resty/* /usr/local/openresty/lualib/resty/; 					\
    rm -fr /tmp/openidc;											\
  fi
RUN if git clone https://github.com/fffonion/lua-resty-openssl.git /tmp/openssl; then				\
    cp -av /tmp/openssl/lib/resty/* /usr/local/openresty/lualib/resty/;						\
    rm -fr /tmp/openssl;											\
  fi
RUN if git clone https://github.com/anvouk/lua-resty-jwt-verification.git /tmp/jwt; then			\
    cp -av /tmp/jwt/lib/resty/* /usr/local/openresty/lualib/resty/;						\
    rm -fr /tmp/jwt;												\
  fi
RUN if git clone https://github.com/jkeys089/lua-resty-hmac.git /tmp/hmac; then					\
    cp -av /tmp/hmac/lib/resty/* /usr/local/openresty/lualib/resty/;						\
    rm -fr /tmp/hmac;												\
  fi
RUN if git clone https://github.com/cdbattags/lua-resty-jwt.git /tmp/jwt; then					\
    cp -av /tmp/jwt/lib/resty/* /usr/local/openresty/lualib/resty/;						\
    rm -fr /tmp/jwt;												\
  fi
RUN if git clone https://github.com/bungle/lua-resty-session.git /tmp/session; then				\
    cp -av /tmp/session/lib/resty/* /usr/local/openresty/lualib/resty/;						\
    rm -fr /tmp/session;											\
  fi
RUN if git clone https://github.com/ledgetech/lua-resty-http.git /tmp/http; then				\
    cp -av /tmp/http/lib/resty/* /usr/local/openresty/lualib/resty/;						\
    rm -fr /tmp/http;												\
  fi
RUN if git clone https://github.com/hamishforbes/lua-ffi-zlib.git /tmp/ffi-zlib; then				\
    cp -av /tmp/ffi-zlib/lib/* /usr/local/openresty/lualib/resty/;						\
    rm -fr /tmp/ffi-zlib;											\
  fi
RUN if git clone https://github.com/openresty/lua-resty-redis.git /tmp/redis; then				\
    cp -av /tmp/redis/lib/resty/* /usr/local/openresty/lualib/resty/;						\
    rm -fr /tmp/redis;												\
  fi

RUN useradd nginx &&												\
	mkdir /var/log/nginx && 										\
	chown -R nginx:nginx /var/log/nginx && 									\
	chmod -R 755 /var/log/nginx

RUN rm -rf /var/lib/apt/lists/*

ENTRYPOINT ["/app/endpoint.sh", "/usr/bin/supervisord", "/sql", "-n"]
EXPOSE 8080
# -----------------------------------------------------------------------
# ---
# -----------------------------------------------------------------------
