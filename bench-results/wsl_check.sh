#!/usr/bin/env bash
set -euo pipefail
echo "WHOAMI: $(whoami)"
echo "HOME: $HOME"
DEST="$HOME/phantom-bench"
echo "DEST: $DEST"
echo "REALPATH: $(realpath -m "$DEST")"
if [ -d "$DEST" ]; then
    echo "EXISTS"
else
    echo "NOT_EXISTS"
fi

