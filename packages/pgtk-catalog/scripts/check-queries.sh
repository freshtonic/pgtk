#!/usr/bin/env bash
#
# Run every query in src/loader/queries.rs against PostgreSQL 14, 15, 16, 17,
# and 18 in Docker, and 19 beta when you name it. Fail on any SQL error. Print
# the row count of each query for each version.
#
# The SQL comes out of src/loader/queries.rs. This script holds no second
# copy. The extraction convention is documented at the top of that file:
#
#   //@sql <VariantName> [<first>-[<last>]]
#   const NAME: &str = r#"
#   ... SQL ...
#   "#;
#
# A text with a range runs only on the versions of its range.
#
# Usage:
#   scripts/check-queries.sh            # 14 to 18
#   scripts/check-queries.sh 18 19      # only the versions you name

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
QUERIES_RS="$REPO_ROOT/src/loader/queries.rs"

VERSIONS=("$@")
if [ ${#VERSIONS[@]} -eq 0 ]; then
    VERSIONS=(14 15 16 17 18)
fi

# The image of each version. 19 is pinned to one tag until 19.0.
image_of() {
    case "$1" in
        19) echo "postgres:19rc1" ;;
        *) echo "postgres:$1" ;;
    esac
}

CONTAINER_PREFIX="pgtk-catalog-check-$$"
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
# Writes one file per text into $WORK_DIR, named <n>.sql, and one index file
# that keeps declaration order. Each index line is `<n> <VariantName> <first>
# <last>`. A text with no range gets 0 and 999.

awk -v out="$WORK_DIR" '
    /^\/\/@sql [A-Za-z][A-Za-z0-9_]*( [0-9]+-[0-9]*)?$/ {
        name = $2
        first = 0
        last = 999
        if (NF == 3) {
            split($3, range, "-")
            first = range[1]
            if (range[2] != "") last = range[2]
        }
        n += 1
        file = sprintf("%s/%02d.sql", out, n)
        printf "%02d %s %s %s\n", n, name, first, last > (out "/index.txt")
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

    image="$(image_of "$version")"

    echo "=== PostgreSQL $version ($image, container $name, port $port) ==="

    if ! docker image inspect "$image" >/dev/null 2>&1; then
        echo "  pulling $image ..."
        docker pull "$image" >/dev/null
    fi

    docker rm -f "$name" >/dev/null 2>&1 || true
    STARTED_CONTAINERS+=("$name")
    docker run -d \
        --name "$name" \
        -e POSTGRES_HOST_AUTH_METHOD=trust \
        -e POSTGRES_USER=postgres \
        -e POSTGRES_DB=postgres \
        -p "$port:5432" \
        "$image" >/dev/null

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
        echo "  ERROR: $image did not become ready" >&2
        docker logs "$name" 2>&1 | tail -20 >&2
        FAILED=1
        docker rm -f "$name" >/dev/null 2>&1 || true
        continue
    fi

    server_version=$(docker exec "$name" psql -U postgres -d postgres -tAc \
        "SELECT current_setting('server_version')")
    echo "  server_version: $server_version"

    while read -r number query_name first last; do
        if [ "$version" -lt "$first" ] || [ "$version" -gt "$last" ]; then
            continue
        fi
        sql_file="$WORK_DIR/$number.sql"
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
