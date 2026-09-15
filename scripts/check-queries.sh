#!/usr/bin/env bash
#
# Run every query in src/loader/queries.rs against PostgreSQL 14, 15, 16, and
# 17 in Docker. Fail on any SQL error. Print the row count of each query for
# each version.
#
# The SQL comes out of src/loader/queries.rs. This script holds no second
# copy. The extraction convention is documented at the top of that file:
#
#   //@sql <VariantName>
#   const NAME: &str = r#"
#   ... SQL ...
#   "#;
#
# Usage:
#   scripts/check-queries.sh            # every version
#   scripts/check-queries.sh 16 17      # only the versions you name

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
QUERIES_RS="$REPO_ROOT/src/loader/queries.rs"

VERSIONS=("$@")
if [ ${#VERSIONS[@]} -eq 0 ]; then
    VERSIONS=(14 15 16 17)
fi

CONTAINER_PREFIX="pg-catalog-check-$$"
WORK_DIR="$(mktemp -d)"
STARTED_CONTAINERS=()

cleanup() {
    local name
    for name in "${STARTED_CONTAINERS[@]:-}"; do
        [ -n "$name" ] || continue
        docker rm -f "$name" >/dev/null 2>&1 || true
    done
    rm -rf "$WORK_DIR"
}
trap cleanup EXIT INT TERM

if [ ! -f "$QUERIES_RS" ]; then
    echo "error: $QUERIES_RS not found" >&2
    exit 1
fi

# --- Extract the SQL -------------------------------------------------------
#
# Writes one file per query into $WORK_DIR, named <n>-<VariantName>.sql, and
# one index file that keeps declaration order.

awk -v out="$WORK_DIR" '
    /^\/\/@sql [A-Za-z][A-Za-z0-9_]*$/ {
        name = $2
        n += 1
        file = sprintf("%s/%02d-%s.sql", out, n, name)
        print name > (out "/index.txt")
        want_open = 1
        next
    }
    want_open && /r#"$/ {
        want_open = 0
        in_sql = 1
        next
    }
    in_sql && /^"#;$/ {
        in_sql = 0
        close(file)
        next
    }
    in_sql { print $0 > file }
    END {
        if (in_sql || want_open) {
            print "error: unterminated SQL constant" > "/dev/stderr"
            exit 1
        }
        if (n == 0) {
            print "error: no SQL constants found" > "/dev/stderr"
            exit 1
        }
    }
' "$QUERIES_RS"

QUERY_COUNT=$(wc -l < "$WORK_DIR/index.txt" | tr -d ' ')
echo "Extracted $QUERY_COUNT queries from src/loader/queries.rs"
echo

# --- Run them against each version -----------------------------------------

FAILED=0
PORT_BASE=54320

for version in "${VERSIONS[@]}"; do
    name="$CONTAINER_PREFIX-$version"
    port=$((PORT_BASE + version))

    echo "=== PostgreSQL $version (container $name, port $port) ==="

    if ! docker image inspect "postgres:$version" >/dev/null 2>&1; then
        echo "  pulling postgres:$version ..."
        docker pull "postgres:$version" >/dev/null
    fi

    docker rm -f "$name" >/dev/null 2>&1 || true
    STARTED_CONTAINERS+=("$name")
    docker run -d \
        --name "$name" \
        -e POSTGRES_HOST_AUTH_METHOD=trust \
        -e POSTGRES_USER=postgres \
        -e POSTGRES_DB=postgres \
        -p "$port:5432" \
        "postgres:$version" >/dev/null

    # Wait for readiness. pg_isready reports true once during init, so also
    # require a real query to succeed.
    ready=0
    for _ in $(seq 1 90); do
        if docker exec "$name" pg_isready -U postgres -q >/dev/null 2>&1 &&
           docker exec "$name" psql -U postgres -d postgres -tAc 'SELECT 1' >/dev/null 2>&1; then
            ready=1
            break
        fi
        sleep 1
    done
    if [ "$ready" -ne 1 ]; then
        echo "  ERROR: postgres:$version did not become ready" >&2
        docker logs "$name" 2>&1 | tail -20 >&2
        FAILED=1
        docker rm -f "$name" >/dev/null 2>&1 || true
        continue
    fi

    server_version=$(docker exec "$name" psql -U postgres -d postgres -tAc \
        "SELECT current_setting('server_version')")
    echo "  server_version: $server_version"

    while IFS= read -r query_name; do
        sql_file=$(find "$WORK_DIR" -name "*-$query_name.sql" | head -1)
        docker cp "$sql_file" "$name:/tmp/q.sql" >/dev/null

        # -v ON_ERROR_STOP=1 makes psql exit non-zero on any SQL error. The
        # query runs exactly as written. psql prints the count as a footer
        # line, for example `(423 rows)`.
        if ! out=$(docker exec "$name" psql -U postgres -d postgres \
                -v ON_ERROR_STOP=1 -A -f /tmp/q.sql 2>&1); then
            printf '  %-20s FAILED\n' "$query_name"
            printf '%s\n' "$out" | sed 's/^/      /' >&2
            FAILED=1
            continue
        fi
        rows=$(printf '%s\n' "$out" | tail -1 |
            sed -n 's/^(\([0-9][0-9]*\) rows*)$/\1/p')
        if [ -z "$rows" ]; then
            printf '  %-20s FAILED (no row count in output)\n' "$query_name"
            printf '%s\n' "$out" | tail -5 | sed 's/^/      /' >&2
            FAILED=1
            continue
        fi
        printf '  %-20s %s rows\n' "$query_name" "$rows"
    done < "$WORK_DIR/index.txt"

    docker rm -f "$name" >/dev/null 2>&1 || true
    echo
done

if [ "$FAILED" -ne 0 ]; then
    echo "FAIL: at least one query failed"
    exit 1
fi

echo "OK: every query ran on every version"
