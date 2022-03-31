#!/bin/sh

GENERATE="$(which rumpsteak-generate || if [ -e "../../../target/debug/rumpsteak-generate" ]; then echo "../../../target/debug/rumpsteak-generate"; fi || echo "../../../target/release/rumpsteak-generate")"

failwith() {
	echo [FAIL] $1 1>&2
	exit 127
}

nuscr2dot() {
	for endpoint in C S; do
		nuscr --fsm $endpoint@Proto simple_auth.nuscr | sed s/G/$endpoint/ > $endpoint.dot || failwith "Can not generate .dot files (nuscr error)."
	done
}

checkdots() {
	for endpoint in C S; do
		cmp $endpoint.dot ${endpoint}_expected.dot || failwith "$endpoint.dot is not identical to what is expected."
	done
}

dot2rs() {
	$GENERATE --name Proto C.dot S.dot > simple_auth.rs || failwith "Can not generate .rs file (rumpsteak-generate error)."
}

checkrs() {
	cmp simple_auth.rs simple_auth_expected.rs || failwith "simple.rs is not what is expected."
}

clean() {
	for endpoint in C S; do
		rm $endpoint.dot
	done
	rm simple_auth.rs
}

case "$1" in
	"clean")
		clean
		break ;;
	"config")
		echo "$GENERATE"
		break ;;
	"generate")
		dot2rs
		break;;
	*)
		nuscr2dot
		#checkdots
		dot2rs
		#checkrs
		echo "Files generated" 1>&2
		;;
esac
