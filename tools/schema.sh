#!/usr/bin/env bash
set -e

exec cargo pgrx schema "$@"
