set -eux

cd framework/challenge && aptos move compile 
cd .. 
RUSTFLAGS="--cfg tokio_unstable" cargo run
