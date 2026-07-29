use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::sync::Arc;
use anyhow::{anyhow, Context, Result};

const FATX_SIGNATURE: u32 = 0x58544146; // "XTAF" / "FATX"
const FATX_FAT_OFFSET_BYTES: u64 = 0x1000; // 4KB after partition start

pub struct DrivePrepConfig {
    pub device_path: String,
    pub include_hddx: bool,
    pub partition3_size_gb: Option<u64>,
}

pub fn prepare_xbox360_drive(
    config: DrivePrepConfig,
    progress_cb: Arc<dyn Fn(f32, &str) + Send + Sync>,
) -> Result<()> {
    progress_cb(0.05, &format!("Opening drive device {}...", config.device_path));

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&config.device_path)
        .with_context(|| format!("Failed to open device {}. Ensure permissions or run with root access.", config.device_path))?;

    // Determine device size
    let dev_size = if config.device_path.starts_with("/dev/") {
        unsafe {
            let mut size: u64 = 0;
            let ret = libc::ioctl(std::os::unix::io::AsRawFd::as_raw_fd(&file), 0x80081272, &mut size);
            if ret != 0 || size == 0 {
                return Err(anyhow!("Failed to query device size via ioctl for {}", config.device_path));
            }
            size
        }
    } else {
        file.metadata()?.len()
    };

    let dev_size_gb = dev_size as f64 / (1024.0 * 1024.0 * 1024.0);
    progress_cb(0.10, &format!("Device size: {:.2} GB. Validating partition layout boundaries...", dev_size_gb));

    if dev_size < 0x130EB0000 + 0x10000000 {
        return Err(anyhow!("Device size ({:.2} GB) is too small to fit Xbox 360 partition structures (minimum 6 GB required).", dev_size_gb));
    }

    // Partition boundaries
    let p1_offset = 0x00110000u64; // P1 Cache (2 GB)
    let p1_size = 0x80000000u64;

    let hddx_offset = 0x110EB0000u64; // HDDX Original Xbox Backwards Compatibility (512 MB)
    let hddx_size = 0x20000000u64;

    let p2_offset = 0x120EB0000u64; // P2 System (256 MB)
    let p2_size = 0x10000000u64;

    let p3_offset = 0x130EB0000u64; // P3 Data (Remaining)
    let p3_size = dev_size.saturating_sub(p3_offset);

    // Step 1: Format Partition 1 (Cache)
    progress_cb(0.25, "Formatting Partition 1 (Cache)...");
    format_fatx_partition(&mut file, p1_offset, p1_size, "360_Cache")?;

    // Step 2: Format HDDX (Original Xbox Backwards Compatibility) if enabled
    if config.include_hddx {
        progress_cb(0.45, "Formatting HDDX (Original Xbox Backwards Compatibility Partition)...");
        format_fatx_partition(&mut file, hddx_offset, hddx_size, "HDDX_Emu")?;
    }

    // Step 3: Format Partition 2 (System)
    progress_cb(0.65, "Formatting Partition 2 (System)...");
    format_fatx_partition(&mut file, p2_offset, p2_size, "360_System")?;

    // Step 4: Format Partition 3 (Data / Content)
    progress_cb(0.85, &format!("Formatting Partition 3 (Data - {:.2} GB)...", p3_size as f64 / (1024.0 * 1024.0 * 1024.0)));
    format_fatx_partition(&mut file, p3_offset, p3_size, "360_Data")?;

    progress_cb(1.0, "Xbox 360 Drive Preparation Completed Successfully! All partitions and FATX superblocks written.");
    Ok(())
}

fn format_fatx_partition(
    file: &mut std::fs::File,
    offset_bytes: u64,
    size_bytes: u64,
    volume_name: &str,
) -> Result<()> {
    let cluster_size: u32 = 16384; // 16KB clusters
    let root_cluster: u32 = 1;

    // Seek to partition start
    file.seek(SeekFrom::Start(offset_bytes))?;

    // Write Superblock struct
    // 0x00: Signature ("XTAF" / "FATX") = 0x58544146
    // 0x04: Volume ID
    // 0x08: Sectors per cluster
    // 0x0C: Root directory cluster ID
    let mut sb_data = vec![0u8; 4096];
    let sig_bytes = FATX_SIGNATURE.to_le_bytes();
    sb_data[0..4].copy_from_slice(&sig_bytes);

    let vol_id: u32 = 0x12345678;
    sb_data[4..8].copy_from_slice(&vol_id.to_le_bytes());

    let sectors_per_cluster: u32 = cluster_size / 512;
    sb_data[8..12].copy_from_slice(&sectors_per_cluster.to_le_bytes());

    let root_cluster_bytes = root_cluster.to_le_bytes();
    sb_data[12..16].copy_from_slice(&root_cluster_bytes);

    file.write_all(&sb_data)?;

    // Write FAT Allocation Table (4KB after partition start)
    let fat_offset = offset_bytes + FATX_FAT_OFFSET_BYTES;
    file.seek(SeekFrom::Start(fat_offset))?;

    let num_clusters = (size_bytes / cluster_size as u64) as u32;
    let fat_size_bytes = (num_clusters * 4) as usize;
    let fat_alloc_size = ((fat_size_bytes + 4095) / 4096) * 4096;

    let mut fat_buf = vec![0u8; fat_alloc_size];

    // Cluster 0 reserved (0xFFFFFFFF)
    fat_buf[0..4].copy_from_slice(&0xFFFFFFFFu32.to_le_bytes());
    // Cluster 1 root directory EOF (0xFFFFFFFF)
    fat_buf[4..8].copy_from_slice(&0xFFFFFFFFu32.to_le_bytes());

    file.write_all(&fat_buf)?;

    // Write Root Directory Cluster (after FAT table)
    let cluster_data_offset = fat_offset + fat_alloc_size as u64;
    file.seek(SeekFrom::Start(cluster_data_offset))?;

    // Fill root cluster with End-of-Directory markers (0xFF)
    let root_cluster_buf = vec![0xFFu8; cluster_size as usize];
    file.write_all(&root_cluster_buf)?;

    log::info!("Formatted FATX Partition '{}' at offset {:#X} (Size: {} MB)", volume_name, offset_bytes, size_bytes / (1024 * 1024));
    Ok(())
}
