#!/bin/bash
# The reference results made again with another Umbra build, every query with the optimizer, and
# compared with the committed references.
#
#     UMBRA=<umbra build directory> tools/references/validation.sh <out dir> <sf>...
set -euo pipefail
cd "$(dirname "$0")/../.."
OUT=$1; shift
for sf in "$@"; do
    echo "=== sf$sf $(date -Is)"
    tools/references/make-references.sh "$sf" "$OUT/validation-sf$sf"
    ./target/release/dbo-bench check "workload/results/sf$sf/through-2025-Q4" "$OUT/validation-sf$sf/results" | cut -c1-400 | tail -12
done
echo "=== all done $(date -Is)"
