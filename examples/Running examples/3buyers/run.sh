#!/bin/sh

GENERATE="$(which rumpsteak-generate || echo "../../../target/debug/rumpsteak-generate")"

failwith() {
	echo [FAIL] $1 1>&2
	exit 127
}

nuscr2dot() {
	for endpoint in A C S; do
		nuscr --fsm $endpoint@ThreeBuyers three_buyers.nuscr | sed s/G/$endpoint/ > $endpoint.dot || failwith "Can not generate .dot files (nuscr error)."
	done
}

checkdots() {
	for endpoint in A C S; do
		cmp $endpoint.dot ${endpoint}_expected.dot || failwith "$endpoint.dot is not identical to what is expected."
	done
}

dot2rs() {
	$GENERATE --name ThreeBuyers C.dot S.dot A.dot > 3buyers.rs || failwith "Can not generate .rs file (rumpsteak-generate error)."
}

checkrs() {
	cmp 3buyers.rs 3buyers_expected.rs || failwith "oauth.rs is not what is expected."
}

clean() {
	for endpoint in A C S; do
		rm $endpoint.dot
	done
	rm 3buyers.rs
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
		dot2rs
		checkrs
		echo "Test successful" 1>&2
		;;
esac
