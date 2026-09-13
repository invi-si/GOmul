//! Browser launcher metadata, matching the Android companion-data importer.
//! No guest API or instruction is patched here.
use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LaunchSettings {
    pub phone_number: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub full_framebuffer: bool,
}
impl LaunchSettings {
    pub fn parse(json: Option<&str>) -> anyhow::Result<Self> {
        let value: Self = match json {
            Some(json) => serde_json::from_str(json)?,
            None => Self::default(),
        };
        if let Some(phone) = &value.phone_number {
            anyhow::ensure!(
                phone.len() == 11 && phone.bytes().all(|b| b.is_ascii_digit()),
                "가상 전화번호는 숫자 11자리여야 합니다."
            );
        }
        anyhow::ensure!(
            matches!((value.width, value.height), (None, None))
                || value
                    .width
                    .zip(value.height)
                    .is_some_and(|(w, h)| (64..=1024).contains(&w) && (64..=1024).contains(&h)),
            "화면 크기는 가로·세로 64~1024여야 합니다."
        );
        Ok(value)
    }
    pub fn display(&self) -> Option<(u32, u32, bool)> {
        self.width.zip(self.height).map(|(w, h)| (w, h, self.full_framebuffer))
    }
}
#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchProfile {
    pub phone_number: Option<String>,
    pub numeric_directions: bool,
}
fn companion_name(name: &str) -> bool {
    matches!(name, "savedata" | "locdata" | "ranker")
        || ["it", "mk"].iter().any(|prefix| {
            name.strip_prefix(prefix)
                .is_some_and(|s| !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit()))
        })
}
pub fn inspect(files: &BTreeMap<String, Vec<u8>>, pid: &str) -> anyhow::Result<(LaunchProfile, BTreeMap<String, Vec<u8>>)> {
    let mut profile = LaunchProfile::default();
    if let Some(prefs) = files.get("P/prefs").filter(|b| b.len() == 64) {
        let digest = Sha256::digest(prefs);
        let expected = [
            0x2a, 0x70, 0x2c, 0xdf, 0x2d, 0xb4, 0xb9, 0x6f, 0x44, 0x51, 0x68, 0x72, 0xba, 0x97, 0xb6, 0x74, 0xf6, 0xb6, 0xd5, 0xf5, 0x3f, 0x1f, 0x6a,
            0x95, 0x98, 0xa4, 0xbe, 0x57, 0xba, 0x07, 0x5f, 0xcb,
        ];
        if digest.as_slice() == expected {
            profile.phone_number = Some("01012349876".into());
            profile.numeric_directions = true;
        }
    }
    let mut records = BTreeMap::new();
    let mut parent = None;
    let mut size = 0usize;
    let mut explicit_phone = None;
    for (path, data) in files {
        let (directory, leaf) = path.rsplit_once('/').unwrap_or(("", path));
        if leaf == "gomul.properties" {
            anyhow::ensure!(data.len() <= 4096, "초기 데이터 설정 파일이 너무 큽니다.");
            for line in String::from_utf8_lossy(data).lines() {
                if let Some((name, number)) = line.trim().split_once('=').or_else(|| line.trim().split_once(':')) {
                    if name.trim() != "phoneNumber" {
                        continue;
                    }
                    anyhow::ensure!(explicit_phone.is_none(), "여러 전화번호 설정이 포함되어 있습니다.");
                    explicit_phone = Some(number.trim().to_string());
                }
            }
        }
        if !companion_name(leaf) {
            continue;
        }
        anyhow::ensure!(
            !path.starts_with('/') && !path.contains('\\') && !path.split('/').any(|p| p == ".."),
            "안전하지 않은 데이터 경로입니다."
        );
        anyhow::ensure!(parent.is_none() || parent == Some(directory), "한 게임의 데이터 폴더만 선택하세요.");
        parent = Some(directory);
        size += data.len();
        anyhow::ensure!(size <= 16 * 1024 * 1024, "초기 데이터가 너무 큽니다.");
        anyhow::ensure!(records.insert(leaf.to_string(), data.clone()).is_none(), "중복 데이터 파일입니다.");
    }
    if let Some(phone) = explicit_phone {
        profile.phone_number = Some(phone);
    } else if !records.is_empty() && pid == "PD121120" {
        profile.phone_number = Some("01055145031".into());
    }
    if let Some(phone) = &profile.phone_number {
        anyhow::ensure!(
            phone.len() == 11 && phone.bytes().all(|c| c.is_ascii_digit()),
            "초기 데이터의 전화번호가 올바르지 않습니다."
        );
    }
    Ok((profile, records))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_reject_partial_dimensions_and_invalid_identity() {
        assert!(LaunchSettings::parse(Some(r#"{"width":240}"#)).is_err());
        assert!(LaunchSettings::parse(Some(r#"{"phoneNumber":"abc"}"#)).is_err());
        assert_eq!(
            LaunchSettings::parse(Some(r#"{"width":240,"height":350,"fullFramebuffer":true}"#))
                .unwrap()
                .display(),
            Some((240, 350, true))
        );
    }
    #[test]
    fn supplied_identity_wins_and_only_companion_records_are_imported() {
        let files = BTreeMap::from([
            ("data/savedata".into(), alloc::vec![7]),
            ("data/it12".into(), alloc::vec![8]),
            ("game.jar".into(), alloc::vec![9]),
            ("gomul.properties".into(), b"phoneNumber=01011112222".to_vec()),
        ]);
        let (profile, records) = inspect(&files, "PD121120").unwrap();
        assert_eq!(profile.phone_number.as_deref(), Some("01011112222"));
        assert_eq!(records.len(), 2);
        assert!(!profile.numeric_directions);
    }
    #[test]
    fn unrelated_preferences_do_not_change_controls_or_identity() {
        let (profile, records) = inspect(&BTreeMap::from([("P/prefs".into(), alloc::vec![0;64])]), "other").unwrap();
        assert!(profile.phone_number.is_none());
        assert!(!profile.numeric_directions);
        assert!(records.is_empty());
    }
    #[test]
    fn mixed_companion_folders_are_rejected() {
        assert!(
            inspect(
                &BTreeMap::from([("a/savedata".into(), alloc::vec![1]), ("b/ranker".into(), alloc::vec![2])]),
                "p"
            )
            .is_err()
        );
    }
}
