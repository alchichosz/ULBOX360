use std::path::Path;
use std::fs;

#[derive(Clone, Debug)]
pub struct LibraryItem {
    pub title_name: String,
    pub title_id: String,
    pub content_type: String,
    pub path: String,
    pub size_mb: u64,
}

pub fn scan_x_library<P: AsRef<Path>>(root_dir: P) -> Vec<LibraryItem> {
    let mut items = Vec::new();
    let root = root_dir.as_ref();

    if !root.exists() || !root.is_dir() {
        return items;
    }

    // Check if user selected Content or Content/0000000000000000
    let search_root = if root.join("0000000000000000").is_dir() {
        root.join("0000000000000000")
    } else {
        root.to_path_buf()
    };

    if let Ok(entries) = fs::read_dir(&search_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let folder_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();

            // Match 8-character hexadecimal Title ID (e.g. 545408A7, 584111F7, F04217L)
            if folder_name.len() == 8 {
                if let Ok(title_id_num) = u32::from_str_radix(&folder_name, 16) {
                    let game_name = iso2god::game_list::find_title_by_id(title_id_num)
                        .unwrap_or_else(|| format!("Title ID: {}", folder_name));

                    // Inspect subdirectories for content types (00007000 = GOD, 00000002 = DLC, 000D0000 = Arcade, 000B0000 = TU)
                    let mut found_content = false;
                    if let Ok(sub_entries) = fs::read_dir(&path) {
                        for sub in sub_entries.flatten() {
                            let sub_path = sub.path();
                            if !sub_path.is_dir() {
                                continue;
                            }
                            let sub_name = sub_path.file_name().unwrap_or_default().to_string_lossy().to_string();

                            let (type_name, disc_hint) = match sub_name.as_str() {
                                "00007000" => ("GOD Container (Game)", ""),
                                "00000002" => ("DLC (Add-on Package)", ""),
                                "000D0000" => ("Xbox Live Arcade (XBLA)", ""),
                                "000B0000" => ("Title Update (TU)", ""),
                                "00000001" => ("Saved Game", ""),
                                "00004000" => ("Gamer Picture", ""),
                                "00030000" => ("Avatar Item", ""),
                                _ => ("Custom Package", ""),
                            };

                            let folder_size_bytes = calculate_folder_size(&sub_path);
                            let size_mb = folder_size_bytes / (1024 * 1024);

                            let display_title = if disc_hint.is_empty() {
                                game_name.clone()
                            } else {
                                format!("{} ({})", game_name, disc_hint)
                            };

                            items.push(LibraryItem {
                                title_name: display_title,
                                title_id: folder_name.clone(),
                                content_type: type_name.to_string(),
                                path: sub_path.to_string_lossy().to_string(),
                                size_mb,
                            });
                            found_content = true;
                        }
                    }

                    if !found_content {
                        let folder_size_bytes = calculate_folder_size(&path);
                        let size_mb = folder_size_bytes / (1024 * 1024);
                        items.push(LibraryItem {
                            title_name: game_name,
                            title_id: folder_name.clone(),
                            content_type: "Xbox 360 Folder".to_string(),
                            path: path.to_string_lossy().to_string(),
                            size_mb,
                        });
                    }
                } else if folder_name.to_uppercase().contains("GTA") || folder_name.to_uppercase().contains("MINECRAFT") {
                    let folder_size_bytes = calculate_folder_size(&path);
                    let size_mb = folder_size_bytes / (1024 * 1024);
                    items.push(LibraryItem {
                        title_name: folder_name.clone(),
                        title_id: "CUSTOM".to_string(),
                        content_type: "Game Folder".to_string(),
                        path: path.to_string_lossy().to_string(),
                        size_mb,
                    });
                }
            }
        }
    }

    items
}

fn calculate_folder_size(path: &Path) -> u64 {
    let mut total_size = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_file() {
                if let Ok(meta) = entry_path.metadata() {
                    total_size += meta.len();
                }
            } else if entry_path.is_dir() {
                total_size += calculate_folder_size(&entry_path);
            }
        }
    }
    total_size
}
