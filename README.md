# Aptos CTF Framework

## Usage

To get started, just add the following line to the `dependencies` section in your `Cargo.toml`:
```toml
[dependencies]
apt-ctf-framework = "0.1.0"
```

## What is this for?

This crate is meant to be used to create an environment for capture the flag players to solve challenges related to Aptos blockchain

## Support standalone binary usage
`FRAMEWORK_OUT_DIR="./path"` -> used by build.rs to output serialized aptos framework. Default to `OUT_DIR`.
`APTOS_FRAMEWORK_CACHE_PATH_OVERRIDE="./path"` -> used by lib.rs to override the Aptos Framework path. 