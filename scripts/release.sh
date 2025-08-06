#!/bin/sh

######################################################################
# @author      : Hung Nguyen Xuan Pham (hung0913208@gmail.com)
# @file        : release
# @created     : Tuesday Aug 13, 2024 22:19:39 +07
#
# @description :
######################################################################


function prepare() {
  if [ -f /etc/grafana-agent/config.yaml.shenv ]; then
    if ! envsubst < /etc/grafana-agent/config.yaml.shenv > /etc/grafana-agent/config.yaml; then
      echo "Error: failed to generate /etc/grafana-agent/config.yaml" >&2
      exit 1
    fi
  fi

  if [[ ${DISABLE_AUTO_INIT_DATABASE} = "true" ]]; then
    return
  fi

  for i in {0..30}; do
    if mysqladmin ping -h "$MYSQL_HOST" -u "$MYSQL_USER" -P "${MYSQL_PORT:-3306}" --password="$MYSQL_PASSWORD" --silent; then
      break
    else
      echo "Waiting for MySQL to be ready..."
      sleep 1
    fi
  done

  if ! mysqladmin ping -h "$MYSQL_HOST" -u "$MYSQL_USER" -P "${MYSQL_PORT:-3306}" --password="$MYSQL_PASSWORD" --silent; then
    echo "Error: MySQL is not ready" >&2
    mysqladmin ping -h "$MYSQL_HOST" -u "$MYSQL_USER" -P "${MYSQL_PORT:-3306}" --password="$MYSQL_PASSWORD"
    exit 1
  fi

  for script_path in "$1"/*; do
    if ! psql -Atx "$POSTGRES_DSN" -f "$script_path"; then
      exit $?
    fi
  done
}

function localtonet() {
  if [ -z "${DOTNET_SYSTEM_GLOBALIZATION_INVARIANT:-}" ]; then
    export DOTNET_SYSTEM_GLOBALIZATION_INVARIANT=1
  fi

  if [ -n "${LOCALTONET:-}" ]; then
    set -x
    screen -S "localtonet.pid" -dm localtonet authtoken $LOCALTONET
    set +x
  fi
}

function boot() {
  local cmd=$1

  set -x
  if [ "${VERBOSE}" = "true" ]; then
    sleep 3650d
  fi
  if [ "${HTTP_PROTOCOL}" = "https" ]; then
    sed -i "s/%%FORCE_SSL%%/on/g" /etc/nginx/http.d/default.conf
  else
    sed -i '/HTTPS/d' /etc/nginx/http.d/default.conf
    sed -i '/HTTP_X_FORWARDED_PROTO/d' /etc/nginx/http.d/default.conf
    sed -i '/HTTP_X_FORWARDED_PORT/d' /etc/nginx/http.d/default.conf
    HTTP_PROTOCOL="http"
  fi
  sed -i "s/%%HTTP_SERVER%%/$HTTP_SERVER/g" /etc/nginx/http.d/default.conf
  sed -i "s#%%WOOCOMMERCE_SERVER%%#$WOOCOMMERCE_SERVER#g" /etc/nginx/http.d/default.conf

  shift
  exec "$cmd" "$@"
  set +x
}

CMD=$1
SQL=$2

shift
shift

prepare "$SQL"
boot "$CMD" "$@"
exit $?
