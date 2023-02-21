
workdir=$(dirname $(cd $(dirname $0); pwd))
echo $workdir

FOLDER=$(mktemp -d)

echo $FOLDER
cd $FOLDER
RESULTS=$(mktemp -d $FOLDER/results)

echo $RESULTS

cd $workdir
git checkout main

cargo build
cd dynamic_verify

for file in 3buyers travel_agency auth diabetes; do
    hyperfine --warmup 10 --runs 1000 --export-json $RESULTS/$file.json "cargo run $file" 
done



cd $workdir/overhead
for file in 3buyers travel_agency auth diabetes; do
    echo "*********$file**********"
    python3 eval2.py $RESULTS/$file.json
done





