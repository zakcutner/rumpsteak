#!/bin/sh

INIT_PWD="$(pwd)"

cd examples/Running\ examples/ && \
	./run.sh
cd "$INIT_PWD"

cd comparison && ./commands.sh
cd "$INIT_PWD"

cd comparison && cargo bench
cd "$INIT_PWD"
