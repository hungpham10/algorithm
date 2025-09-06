
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
COPY --from=build /app/target/release/algorithm ./aio
COPY sql ./sql
COPY scripts/release.sh /app/endpoint.sh

# Install runtime dependencies
RUN apk add --no-cache ca-certificates supervisor mysql-client curl git

# Create supervisor configuration directory
RUN mkdir -p /etc/supervisor.d

# Copy supervisor configuration files
COPY conf/supervisor/*.ini /etc/supervisor.d/

# Copy Nginx configuration
COPY conf/nginx/http.conf /etc/nginx/nginx.conf
COPY conf/nginx/www.conf /etc/nginx/http.d/default.conf

# Setup and configure openresty
RUN curl -s https://openresty.org/package/alpine/openresty.rsa.pub -o /etc/apk/keys/openresty.rsa.pub 	\
 	&& echo "https://openresty.org/package/alpine/v3.18/main" >> /etc/apk/repositories 		\
	&& apk add --no-cache openresty

# Setup openresty modules
RUN if git clone https://github.com/zmartzone/lua-resty-openidc.git /tmp/openidc &> /dev/null; then 	\
    cp -av /tmp/openidc/lib/resty/* /usr/lib/nginx/lualib/resty/; 					\
    rm -fr /tmp/openidc;										\
  fi
RUN if git clone https://github.com/fffonion/lua-resty-openssl.git /tmp/openssl &> /dev/null; then	\
    cp -av /tmp/openssl/lib/resty/* /usr/lib/nginx/lualib/resty/;					\
    rm -fr /tmp/openssl;										\
  fi
RUN if git clone https://github.com/anvouk/lua-resty-jwt-verification.git /tmp/jwt &> /dev/null; then	\
    cp -av /tmp/jwt/lib/resty/* /usr/lib/nginx/lualib/resty/;						\
    rm -fr /tmp/jwt;											\
  fi
RUN if git clone https://github.com/jkeys089/lua-resty-hmac.git /tmp/hmac &> /dev/null; then		\
    cp -av /tmp/hmac/lib/resty/* /usr/lib/nginx/lualib/resty/;						\
    rm -fr /tmp/hmac;											\
  fi
RUN if git clone https://github.com/cdbattags/lua-resty-jwt.git /tmp/jwt &> /dev/null; then		\
    cp -av /tmp/jwt/lib/resty/* /usr/lib/nginx/lualib/resty/;						\
    rm -fr /tmp/jwt;											\
  fi
RUN if git clone https://github.com/bungle/lua-resty-session.git /tmp/session &> /dev/null; then	\
    cp -av /tmp/session/lib/resty/* /usr/lib/nginx/lualib/resty/;					\
    rm -fr /tmp/session;										\
  fi
RUN if git clone https://github.com/ledgetech/lua-resty-http.git /tmp/http &> /dev/null; then		\
    cp -av /tmp/http/lib/resty/* /usr/lib/nginx/lualib/resty/;						\
    rm -fr /tmp/http;											\
  fi
RUN if git clone https://github.com/hamishforbes/lua-ffi-zlib.git /tmp/ffi-zlib &> /dev/null; then	\
    cp -av /tmp/ffi-zlib/lib/resty/* /usr/lib/nginx/lualib/resty/;					\
    rm -fr /tmp/ffi-zlib;										\
  fi

ENTRYPOINT ["/app/endpoint.sh", "/usr/bin/supervisord", "/sql", "-n"]
EXPOSE 8080
