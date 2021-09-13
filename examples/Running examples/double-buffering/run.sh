#!/bin/sh

GENERATE="$(which rumpsteak-generate || echo "../../../target/debug/rumpsteak-generate")"
SUBTYPE="$(which subtype || echo "../../../target/debug/subtype")"

failwith() {
	echo [FAIL] $1 1>&2
	exit 127
}

warn() {
	echo [WARN] $1 1>&2
}

nuscr2dot() {
	for endpoint in s k t; do
		nuscr --fsm $endpoint@DB db.nuscr | sed s/G/$endpoint/ > $endpoint.dot || failwith "Can not generate .dot files (nuscr error)."
	done
}

checkdots() {
	for endpoint in s k t; do
		cmp $endpoint.dot ${endpoint}_expected.dot || failwith "$endpoint.dot is not identical to what is expected."
	done
}

dot2rs() {
	$GENERATE --name DB k_optimised.dot s.dot t.dot > db_opt.rs || failwith "Can not generate .rs file (rumpsteak-generate error)."
}

checksubtype() {
	$SUBTYPE --visits 100 k_optimised.dot k.dot
}

clean() {
	for endpoint in k s t; do
		rm $endpoint.dot
	done
	rm db_opt.rs
}

case "$1" in
	"clean")
		clean
		break ;;
	"config")
		echo "$GENERATE"
		break ;;
	*)
		nuscr2dot
		checkdots
		checksubtype
		dot2rs
		warn "The double-duffering example was not implemented with generated .rs files".
		echo "Test successful" 1>&2
		;;
esac
