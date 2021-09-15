#!/bin/sh

cargo build --bins

TARGET="$(cargo metadata --no-deps --format-version=1 | jq -r '.target_directory')/debug"

STREAM="$TARGET/stream"
NESTED_CHOICE="$TARGET/nested_choice"
RING="$TARGET/ring"
DOUBLE_BUFFERING="$TARGET/double_buffering"

for bench in stream nested_choice ring double_buffering; do
	for bin in rumpsteak kmc concur19; do
		mkdir -p data/"$bench"/"$bin"
	done
done

for typ in subtype supertype; do
	for bin in rumpsteak concur19; do
		mkdir data/nested_choice/"$bin"/"$typ"
	done

	mkdir data/ring/rumpsteak/"$typ"
done

for i in $(seq 0 10 100); do
    "$STREAM" --unrolls $i rumpsteak > "data/stream/rumpsteak/$i.txt";
    "$STREAM" --unrolls $i kmc > "data/stream/kmc/$i.txt";
    "$STREAM" --unrolls $i concur19 > "data/stream/concur19/$i.txt";
done

for i in $(seq 5); do
    "$NESTED_CHOICE" --levels $i rumpsteak > "data/nested_choice/rumpsteak/subtype/$i.txt";
    "$NESTED_CHOICE" --levels $i --reverse rumpsteak > "data/nested_choice/rumpsteak/supertype/$i.txt";
    "$NESTED_CHOICE" --levels $i kmc > "data/nested_choice/kmc/$i.txt";
    "$NESTED_CHOICE" --levels $i concur19 > "data/nested_choice/concur19/subtype/$i.txt";
    "$NESTED_CHOICE" --levels $i --reverse concur19 > "data/nested_choice/concur19/supertype/$i.txt";
done

mkdir data/ring/rumpsteak/{subtype,supertype}
for i in $(seq 2 2 30); do
    "$RING" --roles $i -O rumpsteak > "data/ring/rumpsteak/subtype/$i.txt";
    "$RING" --roles $i rumpsteak > "data/ring/rumpsteak/supertype/$i.txt";
    "$RING" --roles $i -O kmc > "data/ring/kmc/$i.txt";
done

for i in $(seq 0 5 100); do
    "$DOUBLE_BUFFERING" --unrolls $i rumpsteak > "data/double_buffering/rumpsteak/$i.txt";
    "$DOUBLE_BUFFERING" --unrolls $i kmc > "data/double_buffering/kmc/$i.txt";
done

hyperfine -w 5 --parameter-scan n 0 100 -D 10 --export-csv data/stream/results.csv 'subtype --visits 2 data/stream/rumpsteak/{n}.txt data/stream/rumpsteak/0.txt' 'kmc --fsm data/stream/kmc/{n}.txt 1 50' 'concur19 -T data/stream/concur19/{n}.txt data/stream/concur19/0.txt'

hyperfine -w 5 --parameter-scan n 1 5 --export-csv data/nested_choice/results.csv 'subtype --visits 1 data/nested_choice/rumpsteak/subtype/{n}.txt data/nested_choice/rumpsteak/supertype/{n}.txt' 'kmc --fsm data/nested_choice/kmc/{n}.txt 1 1' 'concur19 -T data/nested_choice/concur19/subtype/{n}.txt data/nested_choice/concur19/supertype/{n}.txt'

hyperfine -w 5 --parameter-scan n 2 30 -D 2 --export-csv data/ring/results.csv 'subtype --visits 2 data/ring/rumpsteak/subtype/{n}.txt data/ring/rumpsteak/supertype/{n}.txt' 'kmc --fsm data/ring/kmc/{n}.txt 1 1'

hyperfine -w 5 --parameter-scan n 0 100 -D 5 --export-csv data/double_buffering/results.csv 'subtype --visits 76 data/double_buffering/rumpsteak/{n}.txt data/double_buffering/rumpsteak/0.txt' 'kmc --fsm data/double_buffering/kmc/{n}.txt 1 50'
