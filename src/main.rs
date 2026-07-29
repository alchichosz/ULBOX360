pub mod fatx;
mod iso2god_runner;
mod xiso_runner;
mod fatx_runner;
mod ftp_runner;
mod library_runner;
mod drive_prep_runner;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use slint::ComponentHandle;
use fatx::partition::PartitionMapEntry;

slint::include_modules!();

fn read_iso_metadata(path: &Path) -> Option<(String, String, u64)> {
    let file = std::fs::File::open(path).ok()?;
    let meta = std::fs::metadata(path).ok()?;
    let mut reader = iso2god::iso::IsoReader::read(file).ok()?;
    let title_info = iso2god::executable::TitleInfo::from_image(&mut reader).ok()?;
    let title_id = format!("{:08X}", title_info.execution_info.title_id);
    let game_title = iso2god::game_list::find_title_by_id(title_info.execution_info.title_id)
        .unwrap_or_else(|| "Unknown Game".to_string());
    Some((game_title, title_id, meta.len()))
}

fn format_partition_info(entry: &PartitionMapEntry) -> String {
    format!("{} (Offset: {:#X}, Size: {} MB)", entry.name, entry.offset_bytes, entry.size_bytes / (1024 * 1024))
}

pub enum ScanResult {
    Found(String, PartitionMapEntry),
    PermissionDenied(String),
}

fn scan_system_drives() -> Vec<ScanResult> {
    let mut results = Vec::new();
    let mut candidates = Vec::new();

    // Check system block devices
    if let Ok(entries) = std::fs::read_dir("/dev") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("sd") || name.starts_with("nvme") || name.starts_with("mmcblk") {
                candidates.push(format!("/dev/{}", name));
            }
        }
    }

    // Check disk images in user's home / Desktop / Downloads
    if let Ok(user_home) = std::env::var("HOME") {
        let search_dirs = vec![
            user_home.clone(),
            format!("{}/Desktop", user_home),
            format!("{}/Downloads", user_home),
        ];
        for d in search_dirs {
            if let Ok(entries) = std::fs::read_dir(d) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(ext) = path.extension() {
                        let ext_str = ext.to_string_lossy().to_lowercase();
                        if ext_str == "img" || ext_str == "bin" || ext_str == "raw" {
                            candidates.push(path.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
    }

    for dev_path in candidates {
        match std::fs::File::open(&dev_path) {
            Ok(_) => {
                if let Some(entry) = fatx::fs::FatxFs::auto_detect_partition(&dev_path) {
                    results.push(ScanResult::Found(dev_path, entry));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                results.push(ScanResult::PermissionDenied(dev_path));
            }
            _ => {}
        }
    }

    results
}

fn main() -> Result<(), slint::PlatformError> {
    let _ = env_logger::try_init();

    // If running with root privileges on Linux, ensure Qt uses XCB/X11 display fallback to prevent Wayland socket missing crashes
    if unsafe { libc::getuid() == 0 } {
        if std::env::var("QT_QPA_PLATFORM").is_err() {
            std::env::set_var("QT_QPA_PLATFORM", "xcb");
        }
        if std::env::var("DISPLAY").is_err() {
            std::env::set_var("DISPLAY", ":0");
        }
        if std::env::var("XDG_RUNTIME_DIR").is_err() {
            std::env::set_var("XDG_RUNTIME_DIR", "/run/user/1000");
        }
    }

    let ui = AppWindow::new()?;

    // Shared FUSE Mount Session state
    let mount_session: Arc<Mutex<Option<fuser::BackgroundSession>>> = Arc::new(Mutex::new(None));

    // Create a Tokio runtime for async tasks (XISO packing/unpacking)
    let tokio_runtime = tokio::runtime::Runtime::new()
        .expect("Failed to initialize Tokio runtime");

    // Shared state queues for batch converting
    let god_iso_queue: Arc<Mutex<Vec<PathBuf>>> = Arc::new(Mutex::new(Vec::new()));
    let xiso_source_queue: Arc<Mutex<Vec<PathBuf>>> = Arc::new(Mutex::new(Vec::new()));

    // ==========================================
    // ISO to GOD Tab
    // ==========================================
    let ui_weak = ui.as_weak();
    let god_queue_clone = god_iso_queue.clone();
    ui.on_select_god_iso(move || {
        if let Some(paths) = rfd::FileDialog::new()
            .add_filter("Xbox 360 ISO", &["iso"])
            .pick_files()
        {
            let mut queue = god_queue_clone.lock().expect("god_queue lock");
            *queue = paths;
            if let Some(ui) = ui_weak.upgrade() {
                if queue.len() == 1 {
                    ui.set_god_iso_path(queue[0].to_string_lossy().to_string().into());
                } else {
                    ui.set_god_iso_path(format!("{} ISO files selected (Batch Mode)", queue.len()).into());
                }

                // Gather metadata for selected ISOs
                let mut info_lines = Vec::new();
                let mut title_ids = std::collections::HashSet::new();
                let mut dual_disc_detected = false;

                for (idx, p) in queue.iter().enumerate() {
                    let file_name = p.file_name().unwrap_or_default().to_string_lossy();
                    if let Some((title, tid, size)) = read_iso_metadata(p) {
                        let size_gb = size as f64 / (1024.0 * 1024.0 * 1024.0);
                        if !title_ids.insert(tid.clone()) {
                            dual_disc_detected = true;
                        }
                        info_lines.push(format!("Disc {}: {} [Title ID: {}] - {:.2} GB", idx + 1, title, tid, size_gb));
                    } else {
                        info_lines.push(format!("Disc {}: {} (Metadata unavailable)", idx + 1, file_name));
                    }
                }

                if dual_disc_detected {
                    info_lines.push("*** Dual-Disc Game Detected! Select 2-Disc Installation Mode below if needed. ***".to_string());
                }

                ui.set_god_queue_info(info_lines.join("\n").into());
            }
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_select_god_dest(move || {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_god_dest_path(path.to_string_lossy().to_string().into());
            }
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_select_multidisc_mode(move |mode| {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_god_multidisc_mode(mode);
            let status_text = match mode {
                0 => "Mode: Standard GOD Conversion",
                1 => "Mode: 2-Disc GTA V Style (Disc 1: Install Content, Disc 2: Play Disc GOD)",
                2 => "Mode: 2-Disc GOTY DLC Style (Disc 1: Game GOD, Disc 2: DLC Packages)",
                _ => "Mode: Standard GOD Conversion",
            };
            ui.set_god_multidisc_status(status_text.into());
        }
    });

    let ui_weak = ui.as_weak();
    let god_queue_clone = god_iso_queue.clone();
    let tokio_handle_god = tokio_runtime.handle().clone();
    ui.on_convert_to_god(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let dest_dir = PathBuf::from(ui.get_god_dest_path().as_str());
            let trim = ui.get_god_trim();
            let threads_str = ui.get_god_threads().to_string();
            let num_threads: usize = threads_str.parse().unwrap_or(4);
            let mode = ui.get_god_multidisc_mode();

            let mut files = god_queue_clone.lock().expect("god_queue lock").clone();
            if files.is_empty() {
                let direct_path = PathBuf::from(ui.get_god_iso_path().as_str());
                if !direct_path.to_string_lossy().is_empty() && direct_path.exists() {
                    files.push(direct_path);
                }
            }

            if files.is_empty() || dest_dir.to_string_lossy().is_empty() {
                ui.set_god_status("Error: Please select source ISO and destination path.".into());
                return;
            }

            ui.set_god_progress(0.0);

            let ui_thread_weak = ui_weak.clone();

            if mode == 1 {
                // GTA V Style: Disc 1 = Install Content (Unpack), Disc 2 = Play Disc (GOD)
                if files.len() < 2 {
                    ui.set_god_status("Error: GTA V 2-Disc mode requires selecting both Disc 1 & Disc 2 ISOs.".into());
                    return;
                }
                ui.set_god_status("Starting 2-Disc GTA V Style Installation...".into());
                let disc1_iso = files[0].clone();
                let disc2_iso = files[1].clone();
                let tokio_h = tokio_handle_god.clone();

                std::thread::spawn(move || {
                    let ui_cb = ui_thread_weak.clone();

                    // Step 1: Unpack Disc 1 mandatory install content
                    let ui_cb1 = ui_cb.clone();
                    let progress_cb1 = Arc::new(move |file_p: f32, msg: &str| {
                        let p = file_p * 0.45;
                        let status_msg = format!("[Step 1/2] Disc 1 Install Content: {}", msg);
                        let _ = ui_cb1.upgrade_in_event_loop(move |ui| {
                            ui.set_god_progress(p);
                            ui.set_god_status(status_msg.into());
                        });
                    });

                    let dest_dir_clone = dest_dir.clone();
                    let unpack_res = tokio_h.block_on(async {
                        xiso_runner::unpack_xiso(disc1_iso, dest_dir_clone, progress_cb1).await
                    });

                    if let Err(e) = unpack_res {
                        let _ = ui_cb.upgrade_in_event_loop(move |ui| {
                            ui.set_god_status(format!("Disc 1 Install extraction failed: {}", e).into());
                        });
                        return;
                    }

                    // Step 2: Convert Disc 2 (Play Disc) to GOD container
                    let ui_cb2 = ui_cb.clone();
                    let progress_cb2 = Arc::new(move |file_p: f32, msg: &str| {
                        let p = 0.45 + file_p * 0.55;
                        let status_msg = format!("[Step 2/2] Disc 2 Play GOD: {}", msg);
                        let _ = ui_cb2.upgrade_in_event_loop(move |ui| {
                            ui.set_god_progress(p);
                            ui.set_god_status(status_msg.into());
                        });
                    });

                    match iso2god_runner::convert_iso_to_god(disc2_iso, dest_dir, trim, num_threads, progress_cb2) {
                        Ok(_) => {
                            let _ = ui_cb.upgrade_in_event_loop(move |ui| {
                                ui.set_god_progress(1.0);
                                ui.set_god_status("GTA V 2-Disc Install completed successfully! Both GOD Play Disc and mandatory Content packages are ready.".into());
                            });
                        }
                        Err(e) => {
                            let _ = ui_cb.upgrade_in_event_loop(move |ui| {
                                ui.set_god_status(format!("Disc 2 GOD conversion failed: {:?}", e).into());
                            });
                        }
                    }
                });

            } else if mode == 2 {
                // GOTY DLC Style: Disc 1 = Main Game (GOD), Disc 2 = DLC Installer (Unpack Content)
                if files.len() < 2 {
                    ui.set_god_status("Error: GOTY DLC 2-Disc mode requires selecting both Main Game Disc 1 & DLC Disc 2 ISOs.".into());
                    return;
                }
                ui.set_god_status("Starting 2-Disc GOTY DLC Installation...".into());
                let disc1_iso = files[0].clone();
                let disc2_iso = files[1].clone();
                let tokio_h = tokio_handle_god.clone();

                std::thread::spawn(move || {
                    let ui_cb = ui_thread_weak.clone();

                    // Step 1: Convert Disc 1 (Main Game) to GOD container
                    let ui_cb1 = ui_cb.clone();
                    let progress_cb1 = Arc::new(move |file_p: f32, msg: &str| {
                        let p = file_p * 0.55;
                        let status_msg = format!("[Step 1/2] Disc 1 Main Game GOD: {}", msg);
                        let _ = ui_cb1.upgrade_in_event_loop(move |ui| {
                            ui.set_god_progress(p);
                            ui.set_god_status(status_msg.into());
                        });
                    });

                    if let Err(e) = iso2god_runner::convert_iso_to_god(disc1_iso, dest_dir.clone(), trim, num_threads, progress_cb1) {
                        let _ = ui_cb.upgrade_in_event_loop(move |ui| {
                            ui.set_god_status(format!("Disc 1 Main Game GOD conversion failed: {:?}", e).into());
                        });
                        return;
                    }

                    // Step 2: Extract Disc 2 DLC packages directly into Content folder
                    let ui_cb2 = ui_cb.clone();
                    let progress_cb2 = Arc::new(move |file_p: f32, msg: &str| {
                        let p = 0.55 + file_p * 0.45;
                        let status_msg = format!("[Step 2/2] Disc 2 DLC Content: {}", msg);
                        let _ = ui_cb2.upgrade_in_event_loop(move |ui| {
                            ui.set_god_progress(p);
                            ui.set_god_status(status_msg.into());
                        });
                    });

                    let unpack_res = tokio_h.block_on(async {
                        xiso_runner::unpack_xiso(disc2_iso, dest_dir, progress_cb2).await
                    });

                    match unpack_res {
                        Ok(_) => {
                            let _ = ui_cb.upgrade_in_event_loop(move |ui| {
                                ui.set_god_progress(1.0);
                                ui.set_god_status("GOTY DLC 2-Disc Install completed successfully! Main Game GOD and DLC packages are ready in Content folder.".into());
                            });
                        }
                        Err(e) => {
                            let _ = ui_cb.upgrade_in_event_loop(move |ui| {
                                ui.set_god_status(format!("Disc 2 DLC extraction failed: {}", e).into());
                            });
                        }
                    }
                });

            } else {
                // Mode 0: Standard Sequential Batch Convert to GOD
                ui.set_god_status("Initializing Batch GOD Conversion...".into());
                std::thread::spawn(move || {
                    let total_files = files.len();
                    for (idx, source_iso) in files.into_iter().enumerate() {
                        let file_name = source_iso.file_name().unwrap_or_default().to_string_lossy().to_string();
                        let ui_cb_weak = ui_thread_weak.clone();

                        let progress_cb = Arc::new(move |file_progress: f32, msg: &str| {
                            let overall_progress = (idx as f32 + file_progress) / total_files as f32;
                            let status_msg = format!("[{}/{}] {}: {}", idx + 1, total_files, file_name, msg);
                            let _ = ui_cb_weak.upgrade_in_event_loop(move |ui| {
                                ui.set_god_progress(overall_progress);
                                ui.set_god_status(status_msg.into());
                            });
                        });

                        progress_cb(0.0, "Starting...");
                        match iso2god_runner::convert_iso_to_god(source_iso, dest_dir.clone(), trim, num_threads, progress_cb.clone()) {
                            Ok(_) => {
                                progress_cb(1.0, "Done.");
                            }
                            Err(e) => {
                                progress_cb(0.0, &format!("Failed: {:?}", e));
                                std::thread::sleep(std::time::Duration::from_secs(3));
                            }
                        }
                    }

                    let _ = ui_thread_weak.upgrade_in_event_loop(move |ui| {
                        ui.set_god_progress(1.0);
                        ui.set_god_status("Batch GOD conversion completed successfully!".into());
                    });
                });
            }
        }
    });

    // ==========================================
    // XISO Packer/Unpacker Tab
    // ==========================================
    let ui_weak = ui.as_weak();
    let xiso_queue_clone = xiso_source_queue.clone();
    ui.on_select_xiso_source(move |is_file| {
        if is_file {
            if let Some(paths) = rfd::FileDialog::new()
                .add_filter("Xbox ISO", &["iso", "xiso"])
                .pick_files()
            {
                let mut queue = xiso_queue_clone.lock().expect("xiso_queue lock");
                *queue = paths;
                if let Some(ui) = ui_weak.upgrade() {
                    if queue.len() == 1 {
                        ui.set_xiso_source_path(queue[0].to_string_lossy().to_string().into());
                    } else {
                        ui.set_xiso_source_path(format!("{} XISO files selected (Batch Mode)", queue.len()).into());
                    }
                }
            }
        } else {
            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                let mut queue = xiso_queue_clone.lock().expect("xiso_queue lock");
                *queue = vec![path.clone()];
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_xiso_source_path(path.to_string_lossy().to_string().into());
                }
            }
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_select_xiso_dest(move |is_file| {
        let path = if is_file {
            rfd::FileDialog::new()
                .add_filter("Xbox ISO", &["iso", "xiso"])
                .save_file()
        } else {
            rfd::FileDialog::new().pick_folder()
        };

        if let Some(p) = path {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_xiso_dest_path(p.to_string_lossy().to_string().into());
            }
        }
    });

    let ui_weak = ui.as_weak();
    let xiso_queue_clone = xiso_source_queue.clone();
    let tokio_handle = tokio_runtime.handle().clone();
    ui.on_unpack_xiso(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let dest_dir = PathBuf::from(ui.get_xiso_dest_path().as_str());

            let mut files = xiso_queue_clone.lock().expect("xiso_queue lock").clone();
            if files.is_empty() {
                let direct_path = PathBuf::from(ui.get_xiso_source_path().as_str());
                if !direct_path.to_string_lossy().is_empty() && direct_path.exists() {
                    files.push(direct_path);
                }
            }

            if files.is_empty() || dest_dir.to_string_lossy().is_empty() {
                ui.set_xiso_status("Error: Select source XISO files and destination folder.".into());
                return;
            }

            ui.set_xiso_progress(0.0);
            ui.set_xiso_status("Initializing Batch Unpack...".into());

            let ui_thread_weak = ui_weak.clone();
            tokio_handle.spawn(async move {
                let total_files = files.len();
                for (idx, source_iso) in files.into_iter().enumerate() {
                    let file_stem = source_iso.file_stem().unwrap_or_default().to_string_lossy().to_string();
                    let current_dest_dir = if total_files > 1 {
                        dest_dir.join(&file_stem)
                    } else {
                        dest_dir.clone()
                    };

                    let ui_cb_weak = ui_thread_weak.clone();
                    let stem_clone = file_stem.clone();
                    let progress_cb = Arc::new(move |file_progress: f32, msg: &str| {
                        let overall_progress = (idx as f32 + file_progress) / total_files as f32;
                        let status_msg = format!("[{}/{}] {}: {}", idx + 1, total_files, stem_clone, msg);
                        let _ = ui_cb_weak.upgrade_in_event_loop(move |ui| {
                            ui.set_xiso_progress(overall_progress);
                            ui.set_xiso_status(status_msg.into());
                        });
                    });

                    progress_cb(0.0, "Unpacking...");
                    match xiso_runner::unpack_xiso(source_iso, current_dest_dir, progress_cb.clone()).await {
                        Ok(_) => {
                            progress_cb(1.0, "Done.");
                        }
                        Err(e) => {
                            progress_cb(0.0, &format!("Failed: {}", e));
                            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                        }
                    }
                }

                let _ = ui_thread_weak.upgrade_in_event_loop(move |ui| {
                    ui.set_xiso_progress(1.0);
                    ui.set_xiso_status("Batch unpack completed successfully!".into());
                });
            });
        }
    });

    let ui_weak = ui.as_weak();
    let tokio_handle = tokio_runtime.handle().clone();
    ui.on_pack_xiso(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let source_dir = PathBuf::from(ui.get_xiso_source_path().as_str());
            let dest_iso = PathBuf::from(ui.get_xiso_dest_path().as_str());

            if source_dir.to_string_lossy().is_empty() || dest_iso.to_string_lossy().is_empty() {
                ui.set_xiso_status("Error: Select source folder and destination ISO file.".into());
                return;
            }

            ui.set_xiso_progress(0.0);
            ui.set_xiso_status("Packing folder to XISO...".into());

            let ui_thread_weak = ui_weak.clone();
            tokio_handle.spawn(async move {
                let progress_cb = Arc::new(move |progress: f32, msg: &str| {
                    let msg_str = msg.to_string();
                    let _ = ui_thread_weak.upgrade_in_event_loop(move |ui| {
                        ui.set_xiso_progress(progress);
                        ui.set_xiso_status(msg_str.into());
                    });
                });

                match xiso_runner::pack_xiso(source_dir, dest_iso, progress_cb.clone()).await {
                    Ok(_) => {}
                    Err(e) => {
                        progress_cb(0.0, &format!("Error: {}", e));
                    }
                }
            });
        }
    });

    // ==========================================
    // FATX Mounter Tab
    // ==========================================
    let ui_weak = ui.as_weak();
    ui.on_select_fatx_img(move || {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Xbox Disk Image/Device", &["img", "bin", "raw"])
            .pick_file()
        {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_fatx_img_path(path.to_string_lossy().to_string().into());
            }
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_select_fatx_mount(move || {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_fatx_mount_path(path.to_string_lossy().to_string().into());
            }
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_select_partition(move |letter| {
        if let Some(entry) = PartitionMapEntry::from_letter(letter.as_str()) {
            let info = format_partition_info(entry);
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_fatx_partition(letter);
                ui.set_fatx_partition_info(info.into());
            }
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_auto_detect_partition(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let img_path = ui.get_fatx_img_path().as_str().to_string();
            if img_path.is_empty() {
                ui.set_fatx_status("Error: Select device/image path first.".into());
                return;
            }

            ui.set_fatx_status("Scanning offsets for FATX signature...".into());
            let ui_thread_weak = ui_weak.clone();
            std::thread::spawn(move || {
                if let Some(detected) = fatx::fs::FatxFs::auto_detect_partition(&img_path) {
                    let info_str = format_partition_info(&detected);
                    let _ = ui_thread_weak.upgrade_in_event_loop(move |ui| {
                        ui.set_fatx_partition(detected.letter.into());
                        ui.set_fatx_partition_info(info_str.into());
                        ui.set_fatx_status(format!("Auto-detected partition: {}", detected.name).into());
                    });
                } else {
                    let _ = ui_thread_weak.upgrade_in_event_loop(move |ui| {
                        ui.set_fatx_status("Auto-detect failed: No FATX signature found at known offsets.".into());
                    });
                }
            });
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_scan_system_drives(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_fatx_status("Scanning connected block devices & disk images for FATX partitions...".into());
            let ui_thread_weak = ui_weak.clone();
            std::thread::spawn(move || {
                let results = scan_system_drives();
                let _ = ui_thread_weak.upgrade_in_event_loop(move |ui| {
                    let mut found_any = false;
                    let mut perm_denied_dev = None;

                    for r in results {
                        match r {
                            ScanResult::Found(dev, entry) => {
                                ui.set_fatx_img_path(dev.as_str().into());
                                ui.set_fatx_partition(entry.letter.into());
                                ui.set_fatx_partition_info(format_partition_info(&entry).into());
                                ui.set_fatx_status(format!("Found Xbox Drive: {} at {}", entry.name, dev).into());
                                found_any = true;
                                break;
                            }
                            ScanResult::PermissionDenied(dev) => {
                                if perm_denied_dev.is_none() {
                                    perm_denied_dev = Some(dev);
                                }
                            }
                        }
                    }

                    if !found_any {
                        if let Some(dev) = perm_denied_dev {
                            ui.set_fatx_img_path(dev.as_str().into());
                            ui.set_fatx_status(format!("Detected drive {}, but PERMISSION DENIED. Click 'Fix Drive Permissions' to grant access.", dev).into());
                        } else {
                            ui.set_fatx_status("No FATX drive signature found. Type device path (e.g. /dev/sda) or browse file manually.".into());
                        }
                    }
                });
            });
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_relaunch_root(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let dev_path = ui.get_fatx_img_path().as_str().to_string();
            let target_dev = if dev_path.is_empty() {
                "/dev/sda".to_string()
            } else {
                dev_path
            };

            ui.set_fatx_status(format!("Authenticating drive permissions for {} via Polkit...", target_dev).into());
            let ui_thread_weak = ui_weak.clone();

            std::thread::spawn(move || {
                let status = std::process::Command::new("pkexec")
                    .arg("chmod")
                    .arg("666")
                    .arg(&target_dev)
                    .status();

                let _ = ui_thread_weak.upgrade_in_event_loop(move |ui| {
                    match status {
                        Ok(exit_status) if exit_status.success() => {
                            ui.set_fatx_img_path(target_dev.as_str().into());
                            if let Some(detected) = fatx::fs::FatxFs::auto_detect_partition(&target_dev) {
                                let info_str = format_partition_info(&detected);
                                ui.set_fatx_partition(detected.letter.into());
                                ui.set_fatx_partition_info(info_str.into());
                                ui.set_fatx_status(format!("Drive permission granted! Auto-detected: {} on {}", detected.name, target_dev).into());
                            } else {
                                ui.set_fatx_status(format!("Drive permission granted for {}, ready to mount.", target_dev).into());
                            }
                        }
                        _ => {
                            ui.set_fatx_status("Drive permission grant cancelled or failed.".into());
                        }
                    }
                });
            });
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_open_mount_folder(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let mount_path = ui.get_fatx_mount_path().as_str().to_string();
            if !mount_path.is_empty() {
                let _ = std::process::Command::new("xdg-open").arg(&mount_path).spawn();
            }
        }
    });

    let ui_weak = ui.as_weak();
    let mount_session_clone = mount_session.clone();
    ui.on_mount_fatx(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let img_path = ui.get_fatx_img_path().as_str().to_string();
            let mount_path = ui.get_fatx_mount_path().as_str().to_string();
            let partition_letter = ui.get_fatx_partition().as_str().to_string();

            if img_path.is_empty() || mount_path.is_empty() {
                ui.set_fatx_status("Error: Select image file/device and mount point.".into());
                return;
            }

            let partition_entry = PartitionMapEntry::from_letter(&partition_letter)
                .cloned()
                .unwrap_or(PartitionMapEntry {
                    letter: "360_p3",
                    name: "Xbox 360 Partition 3 (Data)",
                    offset_bytes: 0x130EB0000,
                    size_bytes: 0x20000000000,
                });

            ui.set_fatx_status(format!("Mounting {} (Offset {:#X})...", partition_entry.name, partition_entry.offset_bytes).into());

            let mount_session_thread = mount_session_clone.clone();
            let ui_thread_weak = ui_weak.clone();

            std::thread::spawn(move || {
                match fatx_runner::mount_fatx(img_path.clone(), mount_path, partition_entry.offset_bytes, partition_entry.size_bytes) {
                    Ok(session) => {
                        let mut guard = mount_session_thread.lock().expect("mount_session lock");
                        *guard = Some(session);

                        let _ = ui_thread_weak.upgrade_in_event_loop(move |ui| {
                            ui.set_fatx_is_mounted(true);
                            ui.set_fatx_status(format!("Mounted {}! Click 'Open Mounted Folder' to browse.", partition_entry.name).into());
                        });
                    }
                    Err(e) => {
                        let err_msg = format!("{:?}", e);
                        let is_perm_err = err_msg.contains("PermissionDenied") || err_msg.contains("Os { code: 13");
                        let _ = ui_thread_weak.upgrade_in_event_loop(move |ui| {
                            if is_perm_err {
                                ui.set_fatx_status(format!("Mount Failed (Permission Denied on {}). Click 'Elevate to Root' to authenticate via pkexec.", img_path).into());
                            } else {
                                ui.set_fatx_status(format!("Mount failed: {:?}", e).into());
                            }
                        });
                    }
                }
            });
        }
    });

    let ui_weak = ui.as_weak();
    let mount_session_clone = mount_session.clone();
    ui.on_unmount_fatx(move || {
        let mut guard = mount_session_clone.lock().expect("mount_session lock");
        if guard.is_some() {
            // Dropping the BackgroundSession will trigger an unmount
            *guard = None;
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_fatx_is_mounted(false);
                ui.set_fatx_status("Unmounted successfully.".into());
            }
        }
    });

    // ==========================================
    // X Library Tab
    // ==========================================
    let ui_weak = ui.as_weak();
    ui.on_select_lib_path(move || {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_lib_scan_path(path.to_string_lossy().to_string().into());
            }
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_scan_lib(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let scan_path = ui.get_lib_scan_path().as_str().to_string();
            if scan_path.is_empty() {
                ui.set_lib_status("Error: Select a Content folder or GOD directory.".into());
                return;
            }

            ui.set_lib_status(format!("Scanning {} for Xbox Titles and Content...", scan_path).into());
            let ui_thread_weak = ui_weak.clone();

            std::thread::spawn(move || {
                let items = library_runner::scan_x_library(&scan_path);
                let count = items.len();

                let slint_items: Vec<LibraryItemData> = items
                    .into_iter()
                    .map(|item| LibraryItemData {
                        title_name: item.title_name.into(),
                        title_id: item.title_id.into(),
                        content_type: item.content_type.into(),
                        path: item.path.into(),
                        size_mb: item.size_mb.to_string().into(),
                    })
                    .collect();

                let _ = ui_thread_weak.upgrade_in_event_loop(move |ui| {
                    let slint_model = std::rc::Rc::new(slint::VecModel::from(slint_items));
                    ui.set_lib_items(slint_model.into());
                    ui.set_lib_status(format!("Scan complete! Found {} Xbox Titles / Packages.", count).into());
                });
            });
        }
    });

    // ==========================================
    // Drive Preparer Tab
    // ==========================================
    let ui_weak = ui.as_weak();
    ui.on_select_prep_device(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_prep_status("Scanning system drives...".into());
            let ui_thread_weak = ui_weak.clone();
            std::thread::spawn(move || {
                let results = scan_system_drives();
                let _ = ui_thread_weak.upgrade_in_event_loop(move |ui| {
                    for r in results {
                        match r {
                            ScanResult::Found(dev, entry) => {
                                ui.set_prep_device_path(dev.as_str().into());
                                ui.set_prep_status(format!("Selected Target Disk: {} ({})", dev, entry.name).into());
                                return;
                            }
                            ScanResult::PermissionDenied(dev) => {
                                ui.set_prep_device_path(dev.as_str().into());
                                ui.set_prep_status(format!("Selected Target Disk: {} (Permission Denied - click Fix Permissions)", dev).into());
                                return;
                            }
                        }
                    }
                    ui.set_prep_status("No raw drive auto-detected. Type device path (e.g. /dev/sdb) manually.".into());
                });
            });
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_prepare_xbox_drive(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let dev_path = ui.get_prep_device_path().as_str().to_string();
            let include_hddx = ui.get_prep_include_hddx();

            if dev_path.is_empty() {
                ui.set_prep_status("Error: Specify target drive device path (e.g. /dev/sda or /dev/sdb).".into());
                return;
            }

            ui.set_prep_progress(0.0);
            ui.set_prep_status(format!("Preparing Xbox 360 Drive {} (HDDX Emulation: {})...", dev_path, include_hddx).into());

            let ui_thread_weak = ui_weak.clone();
            std::thread::spawn(move || {
                let progress_cb = Arc::new(move |progress: f32, msg: &str| {
                    let msg_str = msg.to_string();
                    let _ = ui_thread_weak.upgrade_in_event_loop(move |ui| {
                        ui.set_prep_progress(progress);
                        ui.set_prep_status(msg_str.into());
                    });
                });

                let config = drive_prep_runner::DrivePrepConfig {
                    device_path: dev_path,
                    include_hddx,
                    partition3_size_gb: None,
                };

                match drive_prep_runner::prepare_xbox360_drive(config, progress_cb.clone()) {
                    Ok(_) => {}
                    Err(e) => {
                        progress_cb(0.0, &format!("Drive Prep Failed: {:?}", e));
                    }
                }
            });
        }
    });

    // ==========================================
    // Aurora FTP Upload Tab
    // ==========================================
    let ui_weak = ui.as_weak();
    ui.on_select_ftp_local(move || {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_ftp_local_path(path.to_string_lossy().to_string().into());
            }
        }
    });

    let ui_weak = ui.as_weak();
    ui.on_start_ftp_upload(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let ip = ui.get_ftp_ip().as_str().to_string();
            let port_str = ui.get_ftp_port().as_str().to_string();
            let port: u16 = port_str.parse().unwrap_or(21);
            let user = ui.get_ftp_user().as_str().to_string();
            let pass = ui.get_ftp_pass().as_str().to_string();
            let local_path = PathBuf::from(ui.get_ftp_local_path().as_str());

            if ip.is_empty() || local_path.to_string_lossy().is_empty() {
                ui.set_ftp_status("Error: Input console IP and select local folder.".into());
                return;
            }

            ui.set_ftp_progress(0.0);
            ui.set_ftp_status("Uploading...".into());

            // Default remote base directory for Aurora scan path (usually on Hdd1)
            let remote_dir = "/Hdd1/Games/";

            let ui_thread_weak = ui_weak.clone();
            std::thread::spawn(move || {
                let progress_cb = Arc::new(move |progress: f32, msg: &str| {
                    let msg_str = msg.to_string();
                    let _ = ui_thread_weak.upgrade_in_event_loop(move |ui| {
                        ui.set_ftp_progress(progress);
                        ui.set_ftp_status(msg_str.into());
                    });
                });

                match ftp_runner::upload_directory_to_ftp(&ip, port, &user, &pass, local_path, remote_dir, progress_cb.clone()) {
                    Ok(_) => {}
                    Err(e) => {
                        progress_cb(0.0, &format!("Upload failed: {:?}", e));
                    }
                }
            });
        }
    });

    ui.run()
}
