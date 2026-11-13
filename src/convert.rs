// Copyright (C) 2026 3vil17. AGPL-3.0 with additional terms, see LICENSE and NOTICE.md.
pub fn extension_for(bytes: &[u8], asset_type_id: i64) -> &'static str {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return "png";
    }
    if bytes.starts_with(b"\xff\xd8\xff") {
        return "jpg";
    }
    if bytes.starts_with(b"OggS") {
        return "ogg";
    }
    if bytes.starts_with(b"ID3") || bytes.starts_with(b"\xff\xfb") || bytes.starts_with(b"\xff\xf3") {
        return "mp3";
    }
    if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        return "webp";
    }
    if bytes.starts_with(b"RIFF") {
        return "wav";
    }
    if bytes.starts_with(b"<roblox!") {
        return "rbxm";
    }
    if bytes.starts_with(b"version ") {
        return "mesh";
    }
    if bytes.starts_with(b"<roblox") || bytes.starts_with(b"<?xml") {
        match asset_type_id {
            1 | 13 | 18 => "png",
            _ => "rbxmx",
        }
    } else {
        match asset_type_id {
            1 | 13 | 18 => "png",
            3 => "ogg",
            4 | 40 => "mesh",
            5 => "lua",
            _ => "bin",
        }
    }
}

pub fn inner_asset_id(bytes: &[u8]) -> Option<i64> {
    if !(bytes.starts_with(b"<roblox") || bytes.starts_with(b"<?xml")) {
        return None;
    }
    let text = String::from_utf8_lossy(bytes);
    for marker in ["rbxassetid://", "asset/?id=", "asset/?ID=", "id="] {
        if let Some(pos) = text.find(marker) {
            let rest = &text[pos + marker.len()..];
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(n) = digits.parse::<i64>() {
                if n > 0 {
                    return Some(n);
                }
            }
        }
    }
    None
}

pub fn type_label(asset_type_id: i64) -> &'static str {
    match asset_type_id {
        1 => "Image",
        2 => "T-Shirt",
        3 => "Audio",
        4 => "Mesh",
        5 => "Lua",
        8 => "Hat",
        9 => "Place",
        10 => "Model",
        13 => "Decal",
        17 => "Head",
        18 => "Face",
        19 => "Gear",
        21 => "Badge",
        24 => "Animation",
        27 => "Torso",
        28 => "Right Arm",
        29 => "Left Arm",
        30 => "Left Leg",
        31 => "Right Leg",
        32 => "Package",
        34 => "Game Pass",
        38 => "Plugin",
        40 => "MeshPart",
        41 => "Hair",
        42 => "Face Accessory",
        43 => "Neck Accessory",
        44 => "Shoulder Accessory",
        45 => "Front Accessory",
        46 => "Back Accessory",
        47 => "Waist Accessory",
        48..=56 => "Avatar Animation",
        57 => "Ear Accessory",
        58 => "Eye Accessory",
        61 => "Emote",
        62 => "Video",
        64..=72 => "Layered Clothing",
        76 => "Eyebrow",
        77 => "Eyelash",
        78 => "Mood",
        79 => "Dynamic Head",
        73 | 74 => "Font",
        88..=90 => "Makeup",
        11 => "Shirt",
        12 => "Pants",
        _ => "Asset",
    }
}

pub fn mime_for(bytes: &[u8]) -> &'static str {
    match extension_for(bytes, 0) {
        "ogg" => "audio/ogg",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "png" => "image/png",
        "jpg" => "image/jpeg",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    }
}

pub fn to_png(bytes: &[u8]) -> Option<Vec<u8>> {
    let img = image::load_from_memory(bytes).ok()?;
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png).ok()?;
    Some(out.into_inner())
}

const DEFAULT_IDS: [i64; 12] = [
    6372755229, 6444884337, 6444884785, 6412503613, 6444320592, 6196665106,
    591058823, 591059876, 591058104, 591057861, 591057625, 591059642,
];

pub fn is_roblox_default(id: i64, asset_type_id: i64, creator_type: &str, creator_id: i64) -> bool {
    let plain = matches!(asset_type_id, 1 | 3 | 13 | 24);
    DEFAULT_IDS.contains(&id) || (plain && creator_id == 1 && creator_type.eq_ignore_ascii_case("user"))
}

#[cfg(test)]
mod tests {
    use super::is_roblox_default;

    #[test]
    fn grid_is_default_even_without_details() {
        assert!(is_roblox_default(6372755229, 0, "", 0));
    }

    #[test]
    fn roblox_animation_is_default() {
        assert!(is_roblox_default(507766388, 24, "User", 1));
    }

    #[test]
    fn roblox_hats_and_places_are_kept() {
        assert!(!is_roblox_default(1818, 9, "User", 1));
        assert!(!is_roblox_default(1029025, 8, "User", 1));
    }

    #[test]
    fn group_one_is_not_roblox() {
        assert!(!is_roblox_default(123, 1, "Group", 1));
    }

    #[test]
    fn normal_asset_is_kept() {
        assert!(!is_roblox_default(75885924747466, 3, "Group", 5399587));
    }
}
