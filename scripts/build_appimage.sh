#!/bin/bash
set -e

IMAGE_NAME="x-addon-oxide-builder"

echo "Building Docker image..."
docker build -t $IMAGE_NAME -f docker/appimage.Dockerfile .

echo "Running build inside Docker..."
docker run --rm -v $(pwd):/app -w /app $IMAGE_NAME /bin/bash scripts/build_appimage_internal.sh

echo "Done! Check for the AppImage in the current directory."
