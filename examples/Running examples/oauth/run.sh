#!/bin/sh

GENERATE="$(which rumpsteak-generate || if [ -e "../../../target/debug/rumpsteak-generate" ]; then echo "../../../target/debug/rumpsteak-generate"; fi || echo "../../../target/release/rumpsteak-generate")"

failwith() {
	echo [FAIL] $1 1>&2
	exit 127
}

nuscr2dot() {
	for endpoint in A C S; do
		nuscr --fsm $endpoint@Proto o_auth.nuscr | sed s/G/$endpoint/ > $endpoint.dot || failwith "Can not generate .dot files (nuscr error)."
	done
}

checkdots() {
	for endpoint in A C S; do
		cmp $endpoint.dot ${endpoint}_expected.dot || failwith "$endpoint.dot is not identical to what is expected."
	done
}

dot2rs() {
	$GENERATE --name Proto C.dot S.dot A.dot > oauth.rs || failwith "Can not generate .rs file (rumpsteak-generate error)."
}

checkrs() {
	cmp oauth.rs oauth_expected.rs || failwith "oauth.rs is not what is expected."
}

clean() {
	for endpoint in A C S; do
		rm $endpoint.dot
	done
	rm oauth.rs
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
