#!/bin/sh

INIT_PWD="$(pwd)"

cd examples/Running\ examples/ && \
	./run.sh
cd "$INIT_PWD"

cd comparison && ./commands.sh

