set -eux

cd framework-solve/solve && aptos move compile
cd ..
cargo r --release
