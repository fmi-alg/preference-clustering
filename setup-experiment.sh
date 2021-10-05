#!/usr/bin/env bash
set -euo pipefail

TARGET="${CARGO_TARGET_DIR=./pref-covers/target}"
if [[ "$1" = "-h" || "$1" = "--help" ]]; then
	"${TARGET}"/release/random_approx_instances --help
	exit
fi

RESULTS_DIR="results/$(date +%y-%m-%d_%H:%M:%S)"
mkdir -p "${RESULTS_DIR}"
cp experiment.makefile "${RESULTS_DIR}/Makefile"

"${TARGET}" --config-only
mv config.yml "${RESULTS_DIR}"

echo "created folder ${RESULTS_DIR} for experiment.\n
to run the experiment:
  cd ${RESULTS_DIR}
  make"
