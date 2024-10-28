#!/bin/bash

######################################################################
# @author      : Hung Nguyen Xuan Pham (hung0913208@gmail.com)
# @file        : release
# @created     : Tuesday Aug 13, 2024 22:19:39 +07
#
# @description : 
######################################################################


function prepare() {
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

  if [[ ${DISABLE_AUTO_INIT_DATABASE} = "true" ]]; then
    return
  fi

  for SCRIPT in $(ls -1 $1); do
    if ! psql -Atx $POSTGRES_DSN -f $1/$SCRIPT; then
      exit $?
    fi
  done
}

function boot() {
  $1 $@
}

CMD=$1
SQL=$2

shift
shift

prepare $SQL
boot $CMD $@
exit $?
