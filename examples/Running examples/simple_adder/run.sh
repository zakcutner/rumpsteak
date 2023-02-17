#!/bin/sh

set -x

GENERATE="$(which rumpsteak-generate || if [ -e "../../../target/debug/rumpsteak-generate" ]; then echo "../../../target/debug/rumpsteak-generate"; fi || echo "../../../target/release/rumpsteak-generate")"

SCR2DOT="../../../../scr2dot/_build/default/scr2dot.exe"
MPST_UNROLL="../../../../mpst_unroll/target/debug/mpst_unroll"
DYNAMIC_VERIFY="../../../../dynamic_verify/target/debug/parser"

PROTO="Adder"
ENDPOINTS="C S"
FILE="simple_adder"

failwith() {
	echo [FAIL] $1 1>&2
	exit 127
}

nuscr2dot() {
	for endpoint in $ENDPOINTS; do
		nuscr --fsm $endpoint@$PROTO ${FILE}.nuscr |
			sed "s/digraph G/digraph $endpoint/" |
			sed s/int/i32/ |
			sed 's/<.*>//' > ${endpoint}.dot || failwith "Can not generate .dot files (nuscr error)."
	done
}

checkdots() {
	for endpoint in $ENDPOINTS; do
		cmp $endpoint.dot ${endpoint}_expected.dot || failwith "$endpoint.dot is not identical to what is expected."
	done
}

dot2rs() {
	DOT="$(echo ${ENDPOINTS}.dot | sed s/\ /.dot\ /g)"

	$GENERATE --name $PROTO $DOT > ${FILE}.rs || failwith "Can not generate .rs file (rumpsteak-generate error)."
}

checkrs() {
	cmp ${FILE}.rs ${FILE}_expected.rs || failwith "${FILE}.rs is not what is expected."
}

dynamic_verify() {
	$SCR2DOT ${FILE}.nuscr | $MPST_UNROLL | $DYNAMIC_VERIFY
}

clean() {
	for endpoint in $ENDPOINTS; do
		rm $endpoint.dot
	done
	rm ${FILE}.rs
}

case "$1" in
	"clean")
		clean
		break ;;
	"config")
		echo "$GENERATE"
		break ;;
	"dyn_verif")
		dynamic_verify
		break ;;
	"nuscr2dot")
		nuscr2dot
		break;;
	"checkdots")
		checkdots
		break;;
	"dot2rs")
		dot2rs
		break;;
	"checkrs")
		checkrs
		break;;
	*)
		nuscr2dot
		checkdots
		dot2rs
		checkrs
		echo "Test successful" 1>&2
		;;
esac
