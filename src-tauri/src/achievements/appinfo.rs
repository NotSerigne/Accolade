// src/achievements/appinfo.rs
use std::collections::HashMap;
use std::path::PathBuf;

#[cfg(target_os = "windows")]
fn get_steam_path() -> Option<PathBuf> {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = hklm
        .open_subkey("SOFTWARE\\WOW6432Node\\Valve\\Steam")
        .or_else(|_| hklm.open_subkey("SOFTWARE\\Valve\\Steam"))
        .ok()?;
    let path: String = key.get_value("InstallPath").ok()?;
    Some(PathBuf::from(path))
}

#[cfg(not(target_os = "windows"))]
fn get_steam_path() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let candidates = [
        format!("{home}/.steam/steam"),
        format!("{home}/.local/share/Steam"),
    ];
    candidates.iter().map(PathBuf::from).find(|p| p.exists())
}

fn read_u32(data: &[u8], offset: usize) -> Option<u32> {
    let bytes = data.get(offset..offset + 4)?;
    Some(u32::from_le_bytes(bytes.try_into().ok()?))
}

fn read_u64(data: &[u8], offset: usize) -> Option<u64> {
    let bytes = data.get(offset..offset + 8)?;
    Some(u64::from_le_bytes(bytes.try_into().ok()?))
}

fn read_string_table(data: &[u8], offset: usize) -> Vec<String> {
    let mut strings = Vec::new();
    let count = match read_u32(data, offset) {
        Some(c) => c as usize,
        None => return strings,
    };
    let mut pos = offset + 4;
    for _ in 0..count {
        if pos >= data.len() {
            break;
        }
        let end = match data[pos..].iter().position(|&b| b == 0) {
            Some(e) => e,
            None => break,
        };
        let s = std::str::from_utf8(&data[pos..pos + end])
            .unwrap_or("")
            .to_string();
        strings.push(s);
        pos += end + 1;
    }
    println!("DEBUG string table: {} strings loaded", strings.len());

    // Diagnostic : premiers index de la string table
    for i in 0..10.min(strings.len()) {
        println!("DEBUG string_table[{}] = {:?}", i, strings[i]);
    }

    strings
}

pub fn read_client_icons() -> HashMap<u32, String> {
    let mut icons = HashMap::new();

    println!("DEBUG read_client_icons called");

    let steam_path = match get_steam_path() {
        Some(p) => p,
        None => {
            println!("DEBUG steam path not found");
            log::warn!("Steam path not found, skipping appinfo.vdf parsing");
            return icons;
        }
    };

    let appinfo_path = steam_path.join("appcache").join("appinfo.vdf");
    if !appinfo_path.exists() {
        log::warn!("appinfo.vdf not found at {:?}", appinfo_path);
        return icons;
    }

    let data = match std::fs::read(&appinfo_path) {
        Ok(d) => {
            println!("DEBUG appinfo read OK, {} bytes", d.len());
            d
        }
        Err(e) => {
            log::error!("Failed to read appinfo.vdf: {e}");
            return icons;
        }
    };

    if data.len() < 16 {
        log::error!("appinfo.vdf too small");
        return icons;
    }

    let magic = read_u32(&data, 0).unwrap_or(0);
    println!("DEBUG magic: {:#010x}", magic);

    let (file_header_size, has_binary_sha1, has_string_table) = match magic {
        0x07564429 => (16usize, true, true),
        0x07564428 => (8usize, true, false),
        0x07564426 | 0x07564427 => (8usize, false, false),
        _ => {
            log::error!("appinfo.vdf: unknown magic {:#010x}", magic);
            return icons;
        }
    };

    let string_table: Vec<String> = if has_string_table {
        let st_offset = match read_u64(&data, 8) {
            Some(o) => o as usize,
            None => {
                log::error!("Failed to read string table offset");
                return icons;
            }
        };
        println!("DEBUG string table offset: {}", st_offset);
        read_string_table(&data, st_offset)
    } else {
        Vec::new()
    };

    if let Some(idx) = string_table.iter().position(|s| s == "clienticon") {
        println!("DEBUG 'clienticon' in string table at index {}", idx);
    } else {
        println!("DEBUG 'clienticon' NOT in string table");
    }

    let entry_header_size: usize = if has_binary_sha1 { 64 } else { 44 };
    let remaining_header = entry_header_size - 4;

    println!(
        "DEBUG file_header_size={} entry_header_size={} remaining_header={}",
        file_header_size, entry_header_size, remaining_header
    );

    let mut offset = file_header_size;
    let mut block_count = 0u32;
    let mut large_blocks = 0u32;

    loop {
        if offset + 4 > data.len() {
            println!("DEBUG offset overflow at {}", offset);
            break;
        }

        let app_id = read_u32(&data, offset).unwrap_or(0);
        offset += 4;

        if app_id == 0 {
            println!(
                "DEBUG hit app_id=0 after {} blocks ({} large)",
                block_count, large_blocks
            );
            break;
        }

        if offset + entry_header_size > data.len() {
            println!("DEBUG entry header overflow");
            break;
        }

        let size = read_u32(&data, offset).unwrap_or(0) as usize;
        offset += entry_header_size;

        let vdf_size = size.saturating_sub(remaining_header);

        block_count += 1;
        if vdf_size > 1000 {
            large_blocks += 1;
        }

        if block_count <= 5 {
            println!(
                "DEBUG block {} app_id={} size={} vdf_size={} offset_after_header={}",
                block_count, app_id, size, vdf_size, offset
            );
        }

        if vdf_size == 0 || offset + vdf_size > data.len() {
            println!(
                "DEBUG block {} invalid vdf_size={} offset={}",
                block_count, vdf_size, offset
            );
            break;
        }

        let block = &data[offset..offset + vdf_size];

        if block_count == 2 {
            println!(
                "DEBUG block 2 first 40 bytes: {:02x?}",
                &block[..40.min(block.len())]
            );

            // Diagnostic : type byte + key_index pour les premières entrées
            // En v41 : [type: u8] [key_index: u32] [value...]
            println!("DEBUG block 2 - premières entrées interprétées comme v41:");
            let mut p = 0usize;
            for i in 0..8 {
                if p + 5 > block.len() {
                    break;
                }
                let type_byte = block[p];
                let key_idx = read_u32(block, p + 1).unwrap_or(0xFFFFFFFF) as usize;
                let key_name = string_table.get(key_idx).map(|s| s.as_str()).unwrap_or("?");
                println!(
                    "DEBUG   entry[{}] pos={} type={:#04x} key_idx={} key={:?}",
                    i, p, type_byte, key_idx, key_name
                );
                // Avancer selon le type
                let value_size = match type_byte {
                    0x00 => 0, // début sous-objet, pas de valeur
                    0x01 => 4, // string table index (u32)
                    0x02 => 4, // u32
                    0x03 => 4, // float
                    0x07 => 8, // u64
                    0x08 => 0, // fin sous-objet, pas de valeur
                    _ => break,
                };
                p += 1 + 4 + value_size;
                if type_byte == 0x08 {
                    // fin de bloc
                    break;
                }
            }
        }

        if let Some(hash) = find_clienticon(block, &string_table) {
            icons.insert(app_id, hash);
        }

        offset += vdf_size;
    }

    for (app_id, hash) in icons.iter().take(5) {
        println!("DEBUG icon: app_id={} hash={}", app_id, hash);
    }
    println!("DEBUG client icons loaded: {}", icons.len());
    log::info!("Loaded {} client icons from appinfo.vdf", icons.len());
    icons
}

fn find_clienticon(block: &[u8], string_table: &[String]) -> Option<String> {
    if string_table.is_empty() {
        // ancien format, inchangé
        let needle = b"clienticon\x00";
        if let Some(pos) = block.windows(needle.len()).position(|w| w == needle) {
            let value_start = pos + needle.len();
            if let Some(end) = block[value_start..].iter().position(|&b| b == 0) {
                let value = std::str::from_utf8(&block[value_start..value_start + end]).ok()?;
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
        return None;
    }

    let clienticon_idx = string_table.iter().position(|s| s == "clienticon")? as u32;

    let mut pos = 0usize;
    while pos + 5 <= block.len() {
        let type_byte = block[pos];

        if type_byte == 0x08 { break; }

        let key_idx = match read_u32(block, pos + 1) {
            Some(i) => i,
            None => break,
        };

        let value_start = pos + 5;

        match type_byte {
            0x00 => {
                // début sous-objet, pas de valeur
                pos += 5;
            }
            0x01 => {
                // string null-terminée inline
                let end = match block[value_start..].iter().position(|&b| b == 0) {
                    Some(e) => e,
                    None => break,
                };
                if key_idx == clienticon_idx {
                    let val = std::str::from_utf8(&block[value_start..value_start + end]).ok()?;
                    if !val.is_empty() {
                        return Some(val.to_string());
                    }
                }
                pos += 5 + end + 1;
            }
            0x02 => { pos += 5 + 4; }  // u32
            0x03 => { pos += 5 + 4; }  // float
            0x07 => { pos += 5 + 8; }  // u64
            _ => break,
        }
    }

    None
}