#!/bin/bash
set -euxo pipefail

export COMMIT_SHA=$(git rev-parse HEAD)

# Initialize and update the ethgild submodule
echo "Initializing ethgild submodule..."
git submodule update --init --recursive

echo "Running orderbook prep-base..."
(cd lib/rain.orderbook && ./prep-base.sh)

echo "Setup complete!"
