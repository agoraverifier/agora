# Agora Verifier

## Build

```sh
# prepare Rust environment
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# build the project
cargo build
```

## Smart Contract Deployment

Please refer to [this document](./contracts/README.md).

## Verification of SFI and LVI Mitigation

Note: If you want to build your own binaries for verification, please refer to [this instruction](./resources/README.md).

## VeriWASM SFI PoC

1. Generate Proof

```sh
cd poc/veriwasm_proofgen
./proof_gen.sh path_to_X86_ELF_WASM_Module
# for example
./proof_gen.sh ../../resources/brloop/brloop
# back to the root directory
cd ../..
```

Now, the proof is generated at the input binary file's location.

2. Proof Validation and Constraint File Generation

Note: this step will output a series of SMT2 files in current directory.

```sh
cargo run --bin checker -- path_to_binary path_to_proof
# for example
cargo run --bin checker -- resources/brloop/brloop resources/brloop/brloop.prf
# or with 8 threads
cargo run --bin checker -- resources/brloop/brloop resources/brloop/brloop.prf -t 8
```

## LVI Mitigation PoC

1. Generate Proof

```sh
cargo run --bin lfence -- resources/brloop/brloop_lvi 
# the output proof file is located at .output.prf
```

2. Proof Validation and Constraint File Generation

```sh
cargo run --bin checker -- -p lfence resources/load/load_lvi ./output.prf      
```
