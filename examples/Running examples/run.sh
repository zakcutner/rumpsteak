#!/bin/sh

DIR=$(pwd)

run() {
	for example in ./*; do 
		if [ -e $example/run.sh ]; then
			echo "Running example $example"
			cd $example
			./run.sh
			cd "$DIR"
			echo ""
		fi
	done
}

clean() {
	for example in ./*; do 
		if [ -e $example/run.sh ]; then
			echo "Cleaning example $example"
			cd $example
			./run.sh clean
			cd "$DIR"
			echo ""
		fi
	done
}

case "$1" in 
	"clean")
		clean
		break;;
	*) 
		run
		;;
esac
