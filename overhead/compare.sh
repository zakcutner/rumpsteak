
workdir=$(dirname $(cd $(dirname $0); pwd))
echo $workdir

FOLDER=$(mktemp -d)

echo $FOLDER
cd $FOLDER
RESULTS=$(mktemp -d $FOLDER/results)

echo $RESULTS

cd $workdir
git checkout master

cargo build
cd examples

for file in 3buyers travel_agency auth diabetes; do
    hyperfine --warmup 10 --runs 1000 --export-json $RESULTS/$file.1.json "cargo run --example $file" 
done

git checkout main
cd ..
cargo build
cd examples

for file in 3buyers travel_agency auth diabetes; do
    hyperfine --warmup 10 --runs 1000 --export-json $RESULTS/$file.2.json "cargo run --example $file" 
done



cd $workdir/overhead
for file in 3buyers travel_agency auth diabetes; do
    echo "*********$file**********"
    python3 eval.py $RESULTS/$file.1.json $RESULTS/$file.2.json
done





