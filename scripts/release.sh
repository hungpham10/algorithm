#!/bin/bash

######################################################################
# @author      : Hung Nguyen Xuan Pham (hung0913208@gmail.com)
# @file        : release
# @created     : Tuesday Aug 13, 2024 22:19:39 +07
#
# @description :
######################################################################


function prepare() {
  if [ -f /etc/grafana-agent/config.yaml.shenv ]; then
    envsubst < /etc/grafana-agent/config.yaml.j2 > /etc/grafana-agent/config.yaml
  fi

  if [[ ${DISABLE_AUTO_INIT_DATABASE} = "true" ]]; then
    return
  fi

  for I in {0..30}; do
    if ! pg_isready -d "$POSTGRES_DSN"; then
      sleep 1
    else
      break
    fi
  done

  if ! pg_isready -d "$POSTGRES_DSN" &> /dev/null; then
    pg_isready -d "$POSTGRES_DSN"
    exit $?
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

  shift
  exec "$cmd" "$@"
}

CMD=$1
SQL=$2
PORT=$3

shift
shift
shift

prepare "$SQL"
localtonet "$PORT"
boot "$CMD" "$@"
exit $?
