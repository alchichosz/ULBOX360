# ULBOX360

Portable all-in-one GUI utility for Xbox 360 and Original Xbox game backup management on Linux.

Built entirely in **Rust** using the **Slint** GUI framework with native **Qt 6** integration.

Portable **AppImage** and **Debian package** generation are included out of the box.

The AppImage bundles the required Qt runtime and plugins, making it portable across modern Linux distributions without requiring users to install additional Qt packages.

---

![Linux](https://img.shields.io/badge/Linux-AppImage%20%2B%20Debian-success?style=flat-square)
![Rust](https://img.shields.io/badge/Rust-Stable-orange?style=flat-square)
![License](https://img.shields.io/badge/License-MIT-green?style=flat-square)
![AI-Generated](https://img.shields.io/badge/Code-AI--Assisted%20%3E50%25-blue?style=flat-square)

---

# Features

_Since I am not an experienced developer, code reviews, refactoring, bug fixes, and feature suggestions are highly appreciated._

---

## 1. ISO to GOD (Games on Demand)

Convert Xbox 360 ISO backups into native Games on Demand packages.

Features:

- ISO → GOD conversion
- Multi-threaded processing
- Optional zero-sector trimming
- Automatic Content folder generation
- Xbox 360 compatible output

Supports dual-disc games:

### GTA V Mode

Disc 1 is extracted directly into Content/.

Disc 2 becomes a playable GOD container.

### GOTY Mode

Main game becomes GOD.

Second disc DLC packages are automatically extracted into:

```

Content/0000000000000000/<TitleID>/00000002/

```

Examples:

- Fallout New Vegas Ultimate
- Skyrim GOTY

---

## 2. X Library

Scan existing Xbox 360 Content folders and identify installed games.

Features:

- Decode hexadecimal Title IDs
- Detect GOD packages
- Detect DLC
- Detect XBLA titles
- Detect Title Updates
- Fast scrolling game library

Example:

```

545408A7 → Grand Theft Auto V

584111F7 → Minecraft

```

---

## 3. FATX Drive Preparation

Prepare a SATA, USB or image file as a native Xbox 360 HDD.

Supported:

- Physical disks (/dev/sdX)
- Image files
- Internal HDD layout

Automatic partition creation:

| Partition | Purpose |
|------------|-------------------------|
| Partition 1 | Cache |
| Partition 2 | System |
| Partition 3 | Games / Profiles |
| HDDX | Original Xbox Compatibility |

---

## 4. FATX FUSE Mount

Mount Xbox FATX partitions directly under Linux.

Supports:

- Partition 3
- Partition 2
- Partition 1
- HDDX
- Original Xbox partitions
- Custom offsets

Features:

- Native FUSE
- Read-only safety
- Physical drives
- Image files
- Automatic permission helper

---

## 5. XISO Tools

Complete Xbox filesystem utilities.

Functions:

- Pack directories into XISO
- Extract XISO images
- Xbox Original support
- Xbox 360 support
- XDVDFS compatible

---

## 6. FTP Client

Upload directly to Aurora dashboards.

Features:

- Recursive upload
- Progress bars
- Automatic directory creation
- Compatible with Aurora and FreestyleDash

---

# Requirements

## Debian / Ubuntu

```bash
sudo apt install \
    build-essential \
    curl \
    git \
    pkg-config \
    libfontconfig1-dev \
    libfuse3-dev \
    qt6-base-dev \
    qt6-wayland
```

Install Rust:

```bash
curl https://sh.rustup.rs -sSf | sh
```

---

## Fedora

```bash
sudo dnf install \
    gcc \
    gcc-c++ \
    curl \
    git \
    pkg-config \
    fontconfig-devel \
    fuse3-devel \
    qt6-qtbase-devel \
    qt6-qtwayland
```

Install Rust:

```bash
curl https://sh.rustup.rs -sSf | sh
```

---

# Building

Clone the repository:

```bash
git clone https://github.com/vladkioladki/ULBOX360.git
cd ULBOX360
```

Compile:

```bash
cargo build --release
```

Binary:

```
target/release/ulbox360
```

---

# Build AppImage

Generate a fully portable AppImage.

```bash
./build_appimage.sh
```

Output:

```
dist/ULBOX360-x86_64.AppImage
```

The script automatically:

- compiles the project
- downloads linuxdeploy when needed
- bundles Qt runtime
- bundles Qt plugins
- creates the AppImage
- cleans temporary files

No Qt installation is required on the destination computer.

---

# Build Debian Package

Generate a Debian package.

```bash
./build_deb.sh
```

Output:

```
dist/ulbox360_0.1.5_amd64.deb
```

The package includes:

- executable
- desktop entry
- icon
- metadata
- menu integration

---

# Running

Run directly from Cargo:

```bash
cargo run --release
```

Run AppImage:

```bash
chmod +x dist/ULBOX360-x86_64.AppImage
./dist/ULBOX360-x86_64.AppImage
```

Install Debian package:

```bash
sudo dpkg -i dist/ulbox360_0.1.5_amd64.deb
```

---

# Repository Layout

```
ULBOX360
├── src/
├── ui/
├── packaging/
│   └── tools/
├── build_appimage.sh
├── build_deb.sh
├── Cargo.toml
└── dist/
```

The `dist/` directory is generated automatically and contains the release artifacts.

---
# Credits

ULBOX360 is built on top of several excellent open-source Rust projects.

Special thanks to their authors and contributors.

| Project | Description |
|---------|-------------|
| **Slint** | Modern native GUI toolkit for Rust |
| **iso2god-rs** | Rust implementation of the Xbox 360 ISO2GOD converter |
| **xdvdfs** | Xbox DVD filesystem implementation |
| **fatx** | FATX filesystem parser and utilities |
| **fuser** | FUSE userspace filesystem library |
| **rfd** | Native cross-platform file dialogs |

Repositories:

- https://github.com/slint-ui/slint
- https://github.com/iliazeus/iso2god-rs
- https://github.com/antangelo/xdvdfs
- https://github.com/mborgerson/fatx
- https://github.com/cberner/fuser
- https://github.com/PolyMeilex/rfd

---

# Packaging

The project includes official packaging scripts.

## AppImage

```bash
./build_appimage.sh
```

Produces:

```
dist/ULBOX360-x86_64.AppImage
```

The generated AppImage:

- bundles Qt libraries
- bundles required Qt plugins
- works without installing Qt
- is portable across modern Linux distributions

---

## Debian Package

```bash
./build_deb.sh
```

Produces:

```
dist/ulbox360_0.1.5_amd64.deb
```

The package installs:

```
/usr/bin/ulbox360

/usr/share/applications/ulbox360.desktop

/usr/share/icons/hicolor/scalable/apps/ulbox360.svg
```

---

# Development

Compile only:

```bash
cargo build --release
```

Run directly:

```bash
cargo run --release
```

Clean:

```bash
cargo clean
```

Update Rust dependencies:

```bash
cargo update
```

---

# Pull Requests

Contributions are welcome.

Examples include:

- bug fixes
- code cleanup
- UI improvements
- performance optimizations
- Linux packaging improvements
- Xbox filesystem improvements
- documentation

Please keep pull requests focused on a single feature or fix whenever possible.

---

# Roadmap

Planned improvements include:

- Better progress reporting
- Additional FATX tools
- More XISO features
- Original Xbox utilities
- Improved FTP management
- Performance optimizations
- Additional desktop integration
- Improved error reporting

---

# License

This project is distributed under the MIT License.

See:

```
LICENSE
```

for complete license information.

---

# Author

Originally created by:

**Vladkioladki**

Packaging improvements, AppImage support, Debian packaging, Linux compatibility improvements, testing and additional contributions by the community.

---

If ULBOX360 helps you preserve and manage your Xbox collection, consider giving the repository a ⭐ on GitHub.
