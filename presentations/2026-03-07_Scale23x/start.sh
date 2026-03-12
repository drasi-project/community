#!/usr/bin/env bash
set -e

cd "$(dirname "$0")/presentation"

if [ ! -d "node_modules" ]; then
  echo "Installing dependencies..."
  npm install
fi

echo "Starting presentation at http://localhost:3000"
echo "Press 'S' to open speaker notes. Ctrl+C to stop."
npx serve . -l 3000
