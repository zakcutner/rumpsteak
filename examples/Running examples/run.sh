#!/bin/sh

INITDIR="$(pwd)"
DIR="$(dirname "$0")"
cd "$DIR"
DIR="$(pwd)"

run() {
	for example in ./*; do 
		if [ -e "$example"/run.sh ]; then
			echo "Running example $example"
			cd "$example"
			./run.sh
			cd "$DIR"
			echo ""
		fi
	done
	cd "$INITDIR"
}

clean() {
	for example in ./*; do 
		if [ -e "$example"/run.sh ]; then
			echo "Cleaning example $example"
			cd "$example"
			./run.sh clean
			cd "$DIR"
			echo ""
		fi
	done
	cd "$INITDIR"
}

case "$1" in 
	"clean")
		clean
		break;;
	"quit")
		break;;
	*) 
		run
		;;
esac
