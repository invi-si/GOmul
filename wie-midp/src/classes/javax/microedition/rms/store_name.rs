use alloc::{string::String, vec::Vec};

const PREFIX: &str = "__gomul_rms_";

// Retain existing ordinary names. Encode names affected by filesystem path
// normalization, including the reserved prefix itself, without collisions.
pub fn storage_name(name: &str) -> String {
    if !name.starts_with(PREFIX) && !name.contains(['/', '\\', '\0']) && name != "." && name != ".." {
        return name.into();
    }
    const HEX: &[u8] = b"0123456789abcdef";
    let mut result = String::from(PREFIX);
    for byte in name.bytes() {
        result.push(HEX[(byte >> 4) as usize] as char);
        result.push(HEX[(byte & 15) as usize] as char);
    }
    result
}

pub fn guest_name(name: &str) -> Option<String> {
    let Some(hex) = name.strip_prefix(PREFIX) else {
        return Some(name.into());
    };
    if hex.len() % 2 != 0 {
        return None;
    }
    let bytes: Option<Vec<_>> = hex
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| Some(((pair[0] as char).to_digit(16)? * 16 + (pair[1] as char).to_digit(16)?) as u8))
        .collect();
    String::from_utf8(bytes?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_are_flat_reversible_and_distinct() {
        for name in ["normal", "a/b", "a\\b", ".", "..", "\0", "한글/저장", "__gomul_rms_612f62"] {
            let key = storage_name(name);
            assert!(!key.contains(['/', '\\', '\0']));
            assert_eq!(guest_name(&key).as_deref(), Some(name));
        }
        assert_ne!(storage_name("a/b"), storage_name("__gomul_rms_612f62"));
        assert_eq!(storage_name("seaData1"), "seaData1");
    }
}
