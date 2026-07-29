# ULBOX360
An All-in-One GUI utility for modded Xbox 360 and Xbox Original game backup management on Linux.
Built entirely in **Rust** using the **Slint** GUI framework, linked with system **Qt** widgets for native Breeze / KDE styling and Wayland performance.

---
![AI-Generated](https://img.shields.io/badge/Code-AI--Assisted%20%3E50%25-blue?style=flat-square)

## Features

_Since I am not an experienced developer, code reviews, refactoring, and bug fixes from the community are highly encouraged and appreciated!_

### 1. ISO to GOD (Games on Demand) & 2-Disc Smart Install
* **ISO Conversion**: Converts raw Xbox 360 ISO backups into Games on Demand (`GOD`) containers directly recognized by RGH/JTAG consoles.
* **Padding Trimming**: Optional zero-byte sector trimming to dramatically reduce size on target HDDs.
* **Parallel Building**: Multi-threaded core allocation for parallel GOD container slicing.
* **2-Disc X360 Smart Modes**:
  * **GTA V Style (Install + Play)**: Unpacks Disc 1 mandatory install packages directly into `Content/`, while Disc 2 is converted into a playable GOD container.
  * **GOTY DLC Style (Game + DLC)**: Converts Disc 1 (Main Game) into GOD and extracts Disc 2's DLC packages (e.g. *Fallout: New Vegas*, *Skyrim GOTY*) into the appropriate `Content/0000000000000000/<TitleID>/00000002/` folder.

### 2. X Library (Title ID Overview)
* **Title ID Decoding**: Scans `Content/` or GOD target directories and decodes obscure 8-character hexadecimal Title IDs (e.g. `545408A7`, `584111F7`) into human-readable game names (e.g., *Grand Theft Auto V*, *Minecraft*, *Fallout: New Vegas*).
* **Content Classification**: Categorizes packages by type (GOD Games, DLC Add-ons, Xbox Live Arcade, Title Updates).
* **Scrollable Library Browser**: Smooth `ScrollView` interface to easily browse large game collections.

### 3. FATX Prepare Console Drive (Xbox 360 HDD Format)
* **Console-Ready Formatting**: Formats connected physical SATA/USB drives (`/dev/sdX`) or raw image files into a native Xbox 360 internal console HDD.
* **Automatic Partition Allocation**: Writes standard Xbox 360 partition table boundaries:
  * **Partition 3 (Data)** (`0x130EB0000`) - Games, Content, Profiles.
  * **Partition 2 (System)** (`0x120EB0000`) - Dashboard / System files.
  * **Partition 1 (Cache)** (`0x00110000`) - System cache.
  * **HDDX Partition (Original Xbox Backwards Compatibility)** (`0x110EB0000`, 512 MB) - Enables running Original Xbox backwards-compatible games.

### 4. FATX FUSE Mounter & System Permission Fix
* **Native Linux Mount**: Mount FATX raw dump images or physical block storage devices (`/dev/sdX`) to any mountpoint directory on Linux using FUSE.
* **Custom & Preset Layouts**: Supports Xbox 360 Partition 3 (Data), Partition 2 (System), Partition 1 (Cache), Original Xbox E/F/X/Y/Z partitions, or offset `0x0`.
* **Fix Drive Permissions**: 1-click Polkit integration (`pkexec chmod 666 /dev/sdX`) to grant drive access without crashing desktop sessions or restarting the GUI.
* **Deadlock-Free FUSE Engine**: Built-in cycle detection and loop safeguards to prevent file manager freezes in GNOME Nautilus or KDE Dolphin.

### 5. XISO Unpacker & Packer
* Pack local directories into Xbox 360 / Xbox Original compatible XISO images.
* Extract files from existing XISOs directly.
* Full support for Xbox Original **XDVDFS** filesystem structures (ideal for backwards compatibility backups in `/Hdd1/Compatibility/Xbox1/`).

### 6. Aurora FTP Upload
* Upload converted games or GOD content directly to the console over network via FTP.
* Recursive directory creation and progress tracking tailored for Aurora / FreestyleDash.

---

## Installation

### Fedora / RedHat
Install the required system dependencies for Slint compile hooks and FUSE 3 support:
```bash
sudo dnf install fontconfig-devel fuse3-devel
```

### Ubuntu / Debian
```bash
sudo apt-get install libfontconfig1-dev libfuse3-dev
```

---

## Usage

### Downloading the AppImage
Download the precompiled AppImage from the releases tab, make it executable, and run it:
```bash
chmod +x ULBOX360-x86_64.AppImage
./ULBOX360-x86_64.AppImage
```

### Building from Source
Ensure you have the latest Rust toolchain installed, then run:
```bash
cargo build --release
```
The compiled binary will be located in `target/release/ulbox360`.

### Packaging into AppImage
To compile and package the app yourself into a portable AppImage container, run the provided build script:
```bash
./build_appimage.sh
```

---

## Credits & Upstream Libraries

ULBOX360 is built using the following outstanding open-source Rust projects:
* **[Slint](https://github.com/slint-ui/slint)** - Next-generation native UI toolkit.
* **[iso2god-rs](https://github.com/iliazeus/iso2god-rs)** - Pure Rust port of the ISO2GOD Xbox 360 engine.
* **[xdvdfs](https://github.com/antangelo/xdvdfs)** - Pure Rust implementation of the Xbox DVD Filesystem.
* **[fatx](https://github.com/mborgerson/fatx)** - Rust implementation of the FATX filesystem parser.
* **[fuser](https://github.com/cberner/fuser)** - FUSE implementation for Rust.
* **[rfd](https://github.com/Polymeilex/rfd)** - Native OS file dialogs wrapper.

---

## License

This project is licensed under the [MIT License](LICENSE).
