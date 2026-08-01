#!/bin/bash
set -e

echo
echo "======================================"
echo " ULBOX360 Release Builder"
echo "======================================"

echo
echo "[1/2] Building AppImage..."

./build_appimage.sh

echo
echo "[2/2] Building Debian package..."

./build_deb.sh

echo
echo "======================================"
echo "Release completed!"
echo "======================================"

echo
ls -lh dist
