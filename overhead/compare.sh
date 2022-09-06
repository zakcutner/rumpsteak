# cd rumpsteak/benches

# cd
# rm -rf rumpsteak_benches

mkdir results

# git clone https://github.com/qixi5703/rumpsteak.git
# cd rumpsteak
cd ..
git checkout master

cargo build
cd examples

for file in 3buyers travel_agency auth; do
    hyperfine --warmup 10 --runs 100 --export-json ../overhead/results/$file.1.json "cargo run --example ${file}" 
done

# git checkout refined_mpst
git checkout main
cd ..
cargo build
cd examples

for file in 3buyers travel_agency auth; do
    hyperfine --warmup 10 --runs 100 --export-json ../overhead/results/$file.2.json "cargo run --example ${file}" 
done

cd ..
cd overhead
# git clone https://github.com/qixi5703/hyperfine.git

for file in 3buyers travel_agency auth; do
    echo "*********$file**********"
    python3 cal_diff.py results/$file.1.json results/$file.2.json
done





