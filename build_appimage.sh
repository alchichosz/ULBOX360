#!/bin/bash
set -e

APP="ULBOX360"
BIN="ulbox360"

VERSION=$(sed -n 's/^version *= *"\(.*\)"/\1/p' Cargo.toml | head -1)

APPDIR="packaging/AppDir"
DIST="dist"
TOOLS="packaging/tools"

OUT="${APP}-${VERSION}-x86_64.AppImage"

echo
echo "========================================="
echo " ULBOX360 AppImage Builder"
echo " Version: ${VERSION}"
echo "========================================="
echo

echo "[1/5] Cleaning..."

rm -rf "${APPDIR}"
mkdir -p "${APPDIR}/usr/bin"
mkdir -p "${DIST}"

echo
echo "[2/5] Building release..."

cargo build --release

cp target/release/${BIN} \
   "${APPDIR}/usr/bin/"

cp ulbox360.desktop "${APPDIR}/"
cp ulbox360.svg "${APPDIR}/"

cat > "${APPDIR}/AppRun" <<'EOF'
#!/bin/sh

HERE="$(dirname "$(readlink -f "$0")")"

export PATH="${HERE}/usr/bin:$PATH"

exec ulbox360 "$@"
EOF

chmod +x "${APPDIR}/AppRun"

cat > "${APPDIR}/usr/bin/qt.conf" <<EOF
[Paths]
Plugins = ../plugins
EOF

echo
echo "[3/5] Checking linuxdeploy..."
if [ ! -x "${TOOLS}/linuxdeploy" ]; then
    echo "Downloading linuxdeploy..."

    mkdir -p "${TOOLS}"

    curl -L \
        https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage \
        -o "${TOOLS}/linuxdeploy"

    chmod +x "${TOOLS}/linuxdeploy"
fi

if [ ! -x "${TOOLS}/linuxdeploy-plugin-qt" ]; then
    echo "Downloading linuxdeploy Qt plugin..."

    curl -L \
        https://github.com/linuxdeploy/linuxdeploy-plugin-qt/releases/download/continuous/linuxdeploy-plugin-qt-x86_64.AppImage \
        -o "${TOOLS}/linuxdeploy-plugin-qt"

    chmod +x "${TOOLS}/linuxdeploy-plugin-qt"
fi

export QMAKE=qmake6

echo
echo "[4/5] Building AppImage..."

ARCH=x86_64 \
OUTPUT="${OUT}" \
ARCH=x86_64 \
"${TOOLS}/linuxdeploy" \
    --appdir "${APPDIR}" \
    --desktop-file ulbox360.desktop \
    --icon-file ulbox360.svg \
    --plugin qt \
    --output appimage

echo
echo "Locating generated AppImage..."

APPIMAGE=$(find . -type f -name "*.AppImage" \
    ! -path "./${DIST}/*" \
    | head -n1)

if [ -z "${APPIMAGE}" ]; then
    echo
    echo "ERROR: AppImage was not generated."
    exit 1
fi

mkdir -p "${DIST}"

mv -f "${APPIMAGE}" "${DIST}/${OUT}"

echo
echo "[5/5] Cleaning..."

rm -rf "${APPDIR}"

echo
echo "========================================="
echo "Done."
echo "========================================="
echo

ls -lh "${DIST}/${OUT}"
