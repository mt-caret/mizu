# Build instructions

The smart contract acting as the backend is written in
[SCaml](https://gitlab.com/dailambda/scaml/-/tree/master), a subset of OCaml.
Compilation of `contract.ml` can be done with Docker with the `scamlc`
script [here](https://gitlab.com/dailambda/docker-tezos-hands-on/-/tree/tezos-hands-on-2020-03-21).

The Mizu client is entirely written in Rust, and can be compiled with `cargo`.
