#!/bin/bash
set -e

APP=ulbox360
VERSION=0.1.5

ROOT=packaging/deb_build
DIST=dist

echo "== Limpando =="

rm -rf "$ROOT"
mkdir -p "$ROOT"

mkdir -p "$ROOT/DEBIAN"
mkdir -p "$ROOT/usr/bin"
mkdir -p "$ROOT/usr/share/applications"
mkdir -p "$ROOT/usr/share/icons/hicolor/scalable/apps"

echo "== Copiando arquivos =="

cp target/release/ulbox360 \
"$ROOT/usr/bin/"

cp ulbox360.desktop \
"$ROOT/usr/share/applications/"

cp ulbox360.svg \
"$ROOT/usr/share/icons/hicolor/scalable/apps/"

cat > "$ROOT/DEBIAN/control" <<EOF
Package: ulbox360
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: amd64
Maintainer: Alexandre Chichosz <SEU_EMAIL>
Homepage: https://github.com/vladkioladki/ULBOX360
Depends: libc6 (>=2.31), libstdc++6, libqt6core6, libqt6gui6, libqt6widgets6, libfuse3-4
Description: All-in-One Xbox 360 Utility
 ULBOX360 is a graphical toolkit for Xbox 360.
 .
 Features:
  * FATX Explorer
  * FATX Mount
  * ISO Extractor
  * ISO2GOD
  * FTP Client
  * XDVDFS
EOF

cat > "$ROOT/DEBIAN/postinst" <<'EOF'
#!/bin/sh
set -e

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database -q
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -q /usr/share/icons/hicolor || true
fi

exit 0
EOF

cat > "$ROOT/DEBIAN/prerm" <<'EOF'
#!/bin/sh
set -e

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database -q
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -q /usr/share/icons/hicolor || true
fi

exit 0
EOF

chmod 755 "$ROOT/DEBIAN/postinst"
chmod 755 "$ROOT/DEBIAN/prerm"
chmod 755 "$ROOT/usr/bin/ulbox360"

mkdir -p "$DIST"

dpkg-deb --build --root-owner-group "$ROOT" \
"$DIST/${APP}_${VERSION}_amd64.deb"

echo
echo "Pacote criado em:"
echo "$DIST/${APP}_${VERSION}_amd64.deb"

ls -lh "$DIST/${APP}_${VERSION}_amd64.deb"
