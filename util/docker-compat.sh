#!/bin/bash
# Run compatibility tests inside Docker.
#
# Usage:
#   util/docker-compat.sh gnu          # Run GNU findutils tests
#   util/docker-compat.sh bfs          # Run BFS tests
#   util/docker-compat.sh gnu sv-bug   # Run a single GNU test
#   util/docker-compat.sh bfs --verbose=tests --gnu  # Custom BFS flags

set -eo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FINDUTILS_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

IMAGE_NAME="findutils-compat"

# Build the Docker image if it doesn't exist (or if --build is passed)
if [[ "$1" == "--build" ]]; then
    shift
    docker build -t "$IMAGE_NAME" -f "$FINDUTILS_DIR/Dockerfile.compat" "$FINDUTILS_DIR"
elif ! docker image inspect "$IMAGE_NAME" &>/dev/null; then
    echo "Image '$IMAGE_NAME' not found, building (this takes a while the first time)..."
    docker build -t "$IMAGE_NAME" -f "$FINDUTILS_DIR/Dockerfile.compat" "$FINDUTILS_DIR"
fi

suite="${1:?Usage: $0 [--build] <gnu|bfs> [args...]}"
shift

case "$suite" in
    gnu)
        docker run --rm \
            -v "$FINDUTILS_DIR:/findutils" \
            -e GNU_DIR=/findutils.gnu \
            "$IMAGE_NAME" \
            bash util/build-gnu.sh "$@"
        ;;
    bfs)
        docker run --rm \
            -v "$FINDUTILS_DIR:/findutils" \
            -e BFS_DIR=/bfs \
            "$IMAGE_NAME" \
            bash util/build-bfs.sh "$@"
        ;;
    *)
        echo "Unknown suite: $suite"
        echo "Usage: $0 [--build] <gnu|bfs> [args...]"
        exit 1
        ;;
esac
