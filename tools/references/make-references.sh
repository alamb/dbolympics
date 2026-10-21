#!/bin/bash
# Reference results of a scale factor: Umbra with exact arithmetic (debug.semana.exactarithmetic,
# from a native build with rationals, e.g. the branch birler-dbolympics-validation), every period
# of every table loaded into one database, then the 409 queries run once, without a timeout. The
# result is a run directory for `dbo-bench references`.
#
#     UMBRA=<umbra build directory> tools/references/make-references.sh <sf> <out dir>
set -euo pipefail
SF=$1
OUT=$(realpath -m "$2")
REPO=$(cd "$(dirname "$0")/../.." && pwd)
UMBRA=${UMBRA:?the Umbra build directory (bin/sql, bin/server)}
PORT=55432
export PGPASSWORD=postgres
PSQL=(psql -X -q -v ON_ERROR_STOP=1 -h 127.0.0.1 -p $PORT -U postgres)

mkdir -p "$OUT/results" "$OUT/rendered"
cd "$OUT"
rm -rf db && mkdir db
"$REPO/target/release/dbo-bench" render --repo "$REPO" --system umbra --out rendered >/dev/null

# the database, and the server on localhost
"$UMBRA/bin/sql" -createdb db/umbra.db <<<"ALTER ROLE postgres WITH LOGIN SUPERUSER PASSWORD 'postgres';" >/dev/null
"$UMBRA/bin/server" -createSSLFiles -certFile db/umbra.cert -keyFile db/umbra.pem -address 127.0.0.1 -port $PORT db/umbra.db >server.log 2>&1 &
SERVER=$!
trap 'kill $SERVER 2>/dev/null || true' EXIT
for i in $(seq 1 60); do "${PSQL[@]}" -c "SELECT 1" >/dev/null 2>&1 && break; sleep 1; done

echo "=== schema $(date -Is)"
"${PSQL[@]}" -f rendered/settings.sql -f rendered/schema.sql >/dev/null

# every period of every table: the tables with arrays as text from the copy (systems/umbra: arrays = "text")
echo "=== load $(date -Is)"
T0=$(date +%s.%N)
for t in $(sed -n 's/^CREATE TABLE \([a-z_0-9]*\).*/\1/p' rendered/schema.sql); do
    dir=$REPO/data/sf$SF/$t
    [ -d "$REPO/data/sf$SF-arrays/$t" ] && dir=$REPO/data/sf$SF-arrays/$t
    for p in 2024-and-earlier 2025-Q1 2025-Q2 2025-Q3 2025-Q4; do
        "${PSQL[@]}" -c "COPY $t FROM '$dir/$p.parquet' WITH (FORMAT parquet)" >/dev/null
    done
done
echo "loaded in $(echo "$(date +%s.%N) - $T0" | bc) s"

# the queries, once each, read-only, with the system's settings and exact arithmetic
echo "=== queries $(date -Is)"
T0=$(date +%s.%N)
fail=0
for q in rendered/q*.sql; do
    n=$(basename "$q" .sql)
    if ! { cat rendered/settings.sql; echo "SET debug.semana.exactarithmetic = true;"; cat "$q"; echo ";"; } \
         | "${PSQL[@]}" --csv -t -f - >"results/$n.csv" 2>"results/$n.err"; then
        echo "$n: failed: $(head -c 300 "results/$n.err")"
        rm -f "results/$n.csv"
        fail=$((fail + 1))
    else
        rm -f "results/$n.err"
    fi
done
echo "queries in $(echo "$(date +%s.%N) - $T0" | bc) s, $fail failed"

VERSION=$("$UMBRA/bin/sql" --version 2>&1 | head -1)
COMMIT=$(git -C "$UMBRA" describe --always --long 2>/dev/null || git -C "$UMBRA" rev-parse --short HEAD)
cat >run.json <<EOF
{"system": "Umbra $COMMIT (bin/server, $VERSION), debug.semana.exactarithmetic", "config": "sf$SF-reference",
 "date": "$(date +%F)", "sf": $SF, "complete": $([ $fail = 0 ] && echo true || echo false)}
EOF
echo "=== done $(date -Is)"
