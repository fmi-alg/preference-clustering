#!/usr/bin/env bash
set -euo pipefail

cargo build --manifest-path=pref-polys/Cargo.toml --release

cmake -B./hs_gen/build -H./hs_gen -DCMAKE_BUILD_TYPE=Release -DCMAKE_EXPORT_COMPILE_COMMANDS=True
cmake --build ./hs_gen/build -- -j

make -C./solve_hs
