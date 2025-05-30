#!/bin/bash

######################################################################
# @author      : Hung Nguyen Xuan Pham (hung0913208@gmail.com)
# @file        : release
# @created     : Tuesday Aug 13, 2024 22:19:39 +07
#
# @description :
######################################################################


function prepare() {
  if [[ ${DISABLE_AUTO_INIT_DATABASE} = "true" ]]; then
    return
  fi

  for I in {0..30}; do
    if ! pg_isready -d $POSTGRES_DSN; then
      sleep 1
    else
      break
    fi
  done

  if ! pg_isready -d $POSTGRES_DSN &> /dev/null; then
    pg_isready -d $POSTGRES_DSN
    exit $?
  fi

  for SCRIPT in $(ls -1 $1); do
    if ! psql -Atx $POSTGRES_DSN -f $1/$SCRIPT; then
      exit $?
    fi
  done
}

function localtonet() {
  if [ ${#DOTNET_SYSTEM_GLOBALIZATION_INVARIANT} -eq 0 ]; then
    export DOTNET_SYSTEM_GLOBALIZATION_INVARIANT=1
  fi

  if [ ${#LOCALTONET} -gt 0 ]; then
    set -x
    screen -S "localtonet.pid" -dm localtonet authtoken $LOCALTONET
    set +x
  fi
}

function boot() {
  CMD=$1

  shift
  $CMD $@
}

CMD=$1
SQL=$2
PORT=$3

shift
shift
shift

prepare $SQL
localtonet $PORT
boot $CMD $@
exit $?
