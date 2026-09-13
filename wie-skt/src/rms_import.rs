//! Decode the record-store headers supplied alongside SKT handset archives.
//! The layout is validated against paired .sb/.db files before any installation.
use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};
use wie_util::{Result, WieError};

pub struct PackagedStore {
    pub name: String,
    pub next: u32,
    pub records: BTreeMap<u32, Vec<u8>>,
}

pub fn collect(files: &BTreeMap<String, Vec<u8>>) -> Result<Vec<PackagedStore>> {
    let mut stores = BTreeMap::new();
    for (path, header) in files {
        let Some((directory, file)) = path.rsplit_once('/') else {
            continue;
        };
        if directory.rsplit('/').next() != Some("rs") {
            continue;
        }
        let Some(stem) = file.strip_suffix(".sb") else {
            continue;
        };
        let data = files
            .get(&alloc::format!("{directory}/{stem}.db"))
            .ok_or_else(|| error("Packaged RMS data file is missing"))?;
        let store = decode(header, data)?;
        if stores.insert(store.name.clone(), store).is_some() {
            return Err(error("Duplicate packaged RMS store name"));
        }
    }
    Ok(stores.into_values().collect())
}

fn error(message: &str) -> WieError {
    WieError::FatalError(message.into())
}

fn take<'a>(input: &mut &'a [u8], length: usize) -> Result<&'a [u8]> {
    let result = input.get(..length).ok_or_else(|| error("Truncated packaged RMS header"))?;
    *input = &input[length..];
    Ok(result)
}

fn u32_be(input: &mut &[u8]) -> Result<u32> {
    let bytes = take(input, 4)?;
    Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn decode(mut header: &[u8], data: &[u8]) -> Result<PackagedStore> {
    let next = u32_be(&mut header)?;
    let length = take(&mut header, 2)?;
    let length = u16::from_be_bytes([length[0], length[1]]) as usize;
    let name = core::str::from_utf8(take(&mut header, length)?)
        .map_err(|_| error("Unsupported packaged RMS name encoding"))?
        .to_string();
    if name.is_empty() || name.encode_utf16().count() > 32 || name.contains(['/', '\\', '\0']) || name == "." || name == ".." {
        return Err(error("Invalid packaged RMS name"));
    }
    let _version = u32_be(&mut header)?;
    let count = u32_be(&mut header)? as usize;
    let size = u32_be(&mut header)? as usize;
    let _last_modified = take(&mut header, 8)?;
    if next == 0 || next > i32::MAX as u32 + 1 || size != data.len() || header.len() % 12 != 0 || count != header.len() / 12 {
        return Err(error("Invalid packaged RMS sizes or sequence"));
    }
    let mut records = BTreeMap::new();
    let mut ranges = Vec::new();
    for _ in 0..count {
        let id = u32_be(&mut header)?;
        let offset = u32_be(&mut header)? as usize;
        let length = u32_be(&mut header)? as usize;
        let end = offset.checked_add(length).ok_or_else(|| error("Packaged RMS range overflow"))?;
        let bytes = data.get(offset..end).ok_or_else(|| error("Packaged RMS record exceeds data file"))?;
        if id == 0 || id >= next || records.insert(id, bytes.to_vec()).is_some() {
            return Err(error("Invalid packaged RMS record ID"));
        }
        if length != 0 {
            ranges.push((offset, end));
        }
    }
    ranges.sort_unstable();
    if ranges.windows(2).any(|pair| pair[0].1 > pair[1].0) {
        return Err(error("Overlapping packaged RMS records"));
    }
    Ok(PackagedStore { name, next, records })
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn header(name: &str, next: u32, size: u32, entries: &[(u32, u32, u32)]) -> Vec<u8> {
        let mut bytes = next.to_be_bytes().to_vec();
        bytes.extend_from_slice(&(name.len() as u16).to_be_bytes());
        bytes.extend_from_slice(name.as_bytes());
        for value in [7, entries.len() as u32, size] {
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        bytes.extend_from_slice(&1234u64.to_be_bytes());
        for &(id, offset, length) in entries {
            for value in [id, offset, length] {
                bytes.extend_from_slice(&value.to_be_bytes());
            }
        }
        bytes
    }

    #[test]
    fn embedded_name_and_sparse_ids_are_preserved() {
        let files = BTreeMap::from([
            ("wrapper/rs/#Save.sb".into(), header("실제Save", 8, 5, &[(1, 0, 2), (7, 2, 3)])),
            ("wrapper/rs/#Save.db".into(), vec![1, 2, 3, 4, 5]),
            ("assets/noise.sb".into(), vec![0]),
        ]);
        let stores = collect(&files).unwrap();
        assert_eq!(stores.len(), 1);
        assert_eq!(stores[0].name, "실제Save");
        assert_eq!(stores[0].next, 8);
        assert_eq!(stores[0].records[&7], [3, 4, 5]);
        assert!(!stores[0].records.contains_key(&2));
    }

    #[test]
    fn invalid_metadata_is_rejected_before_installation() {
        let valid = header("Save", 3, 2, &[(1, 0, 1), (2, 1, 1)]);
        for length in 0..valid.len() {
            assert!(decode(&valid[..length], &[1, 2]).is_err());
        }
        let mut trailing = valid.clone();
        trailing.push(0);
        assert!(decode(&trailing, &[1, 2]).is_err());
        for entries in [
            &[(1, 0, 1), (1, 1, 1)][..],
            &[(0, 0, 1)][..],
            &[(3, 0, 1)][..],
            &[(1, 0, 2), (2, 1, 1)][..],
            &[(1, u32::MAX, 2)][..],
        ] {
            assert!(decode(&header("Save", 3, 2, entries), &[1, 2]).is_err());
        }
        assert!(decode(&valid, &[1]).is_err());
        for name in ["", "..", "../escape", "a\\b", "nul\0name"] {
            assert!(decode(&header(name, 1, 0, &[]), &[]).is_err());
        }
        assert!(collect(&BTreeMap::from([("rs/Save.sb".into(), valid.clone())])).is_err());
        assert!(
            collect(&BTreeMap::from([
                ("rs/a.sb".into(), valid.clone()),
                ("rs/a.db".into(), vec![1, 2]),
                ("rs/b.sb".into(), valid),
                ("rs/b.db".into(), vec![1, 2]),
            ]))
            .is_err()
        );
    }

    #[test]
    fn empty_records_and_deleted_history_are_valid() {
        let store = decode(&header("Save", 9, 0, &[(3, 0, 0)]), &[]).unwrap();
        assert_eq!(store.next, 9);
        assert_eq!(store.records[&3], []);
        assert!(decode(&header("Save", 9, 0, &[]), &[]).unwrap().records.is_empty());
    }

    #[test]
    #[ignore = "requires private extracted handset .sb/.db files"]
    fn actual_handset_store_headers_match_their_data() {
        extern crate std;
        let path = std::env::var("GOMUL_SKT_RMS_FIXTURE_DIR").expect("fixture directory required");
        let mut files = BTreeMap::new();
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            files.insert(
                alloc::format!("rs/{}", path.file_name().unwrap().to_str().unwrap()),
                std::fs::read(path).unwrap(),
            );
        }
        let stores = collect(&files).unwrap();
        assert_eq!(stores.len(), 3);
        for store in &stores {
            assert_eq!(store.next, 2);
            assert_eq!(store.records.len(), 1);
            assert_eq!(store.records[&1].len(), if store.name == "skseaData" { 350 } else { 300 });
        }
    }
}
