//! Persistent MIDP records. Record zero is outside the guest's valid ID range.
//! Store the next ID and contents together so a host write cannot commit one
//! without the other. Existing positive records remain untouched recovery data.
use alloc::{boxed::Box, collections::BTreeMap, vec::Vec};
use spin::Mutex;
use wie_backend::{Database, RecordId};

const MAGIC: &[u8] = b"GOMULRMS\x01";
const DELETED: &[u8] = b"GOMULRMS-DELETED\x01";

pub struct RecordData {
    backend: Mutex<Box<dyn Database>>,
    next: u32,
    records: BTreeMap<u32, Vec<u8>>,
}

impl RecordData {
    pub async fn is_deleted(backend: &mut dyn Database) -> bool {
        backend.get(0).await.as_deref() == Some(DELETED)
    }

    pub async fn mark_deleted(backend: &mut dyn Database) -> bool {
        backend.set(0, DELETED).await
    }
    pub async fn install(backend: Box<dyn Database>, next: u32, records: BTreeMap<u32, Vec<u8>>) -> bool {
        if next == 0 || next > i32::MAX as u32 + 1 || records.keys().any(|id| *id == 0 || *id >= next) {
            return false;
        }
        let mut data = Self {
            backend: Mutex::new(backend),
            next: 1,
            records: BTreeMap::new(),
        };
        data.commit(records, next).await
    }
    pub async fn load(backend: Box<dyn Database>) -> Result<Self, &'static str> {
        let mut result = Self {
            backend: Mutex::new(backend),
            next: 1,
            records: BTreeMap::new(),
        };
        let saved = result.backend.get_mut().get(0).await;
        if let Some(bytes) = saved {
            let mut input = bytes.strip_prefix(MAGIC).ok_or("Unrecognized RMS metadata; existing data retained")?;
            result.next = take_u32(&mut input)?;
            if result.next == 0 || result.next > i32::MAX as u32 + 1 {
                return Err("Invalid RMS next record ID");
            }
            let count = take_u32(&mut input)?;
            for _ in 0..count {
                let id = take_u32(&mut input)?;
                let length = take_u32(&mut input)? as usize;
                let data = input.get(..length).ok_or("Truncated RMS record")?;
                if id == 0 || id >= result.next || result.records.insert(id, data.to_vec()).is_some() {
                    return Err("Invalid RMS record ID");
                }
                input = &input[length..];
            }
            if !input.is_empty() {
                return Err("Unexpected RMS trailing data");
            }
        } else {
            // The old format did not retain deleted IDs. Recover the highest
            // surviving ID; never renumber or erase the old records.
            let ids = result.backend.get_mut().get_record_ids().await;
            for id in ids {
                if id == 0 || id > i32::MAX as u32 {
                    return Err("Invalid legacy RMS record ID");
                }
                let data = result.backend.get_mut().get(id).await.ok_or("Missing legacy RMS record")?;
                result.next = result.next.max(id + 1);
                result.records.insert(id, data);
            }
        }
        Ok(result)
    }

    async fn commit(&mut self, records: BTreeMap<u32, Vec<u8>>, next: u32) -> bool {
        let mut bytes = MAGIC.to_vec();
        bytes.extend_from_slice(&next.to_le_bytes());
        bytes.extend_from_slice(&(records.len() as u32).to_le_bytes());
        for (id, data) in &records {
            let Ok(length) = u32::try_from(data.len()) else {
                return false;
            };
            bytes.extend_from_slice(&id.to_le_bytes());
            bytes.extend_from_slice(&length.to_le_bytes());
            bytes.extend_from_slice(data);
        }
        if !self.backend.get_mut().set(0, &bytes).await {
            return false;
        }
        self.records = records;
        self.next = next;
        true
    }
}

fn take_u32(input: &mut &[u8]) -> Result<u32, &'static str> {
    let bytes = input.get(..4).ok_or("Truncated RMS metadata")?;
    let value = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    *input = &input[4..];
    Ok(value)
}

#[async_trait::async_trait]
impl Database for RecordData {
    async fn next_id(&self) -> RecordId {
        self.next
    }
    async fn add(&mut self, data: &[u8]) -> RecordId {
        if self.next > i32::MAX as u32 {
            return 0;
        }
        let id = self.next;
        let mut records = self.records.clone();
        records.insert(id, data.to_vec());
        if self.commit(records, id + 1).await { id } else { 0 }
    }
    async fn get(&self, id: RecordId) -> Option<Vec<u8>> {
        self.records.get(&id).cloned()
    }
    async fn set(&mut self, id: RecordId, data: &[u8]) -> bool {
        if !self.records.contains_key(&id) {
            return false;
        }
        let mut records = self.records.clone();
        records.insert(id, data.to_vec());
        self.commit(records, self.next).await
    }
    async fn delete(&mut self, id: RecordId) -> bool {
        let mut records = self.records.clone();
        if records.remove(&id).is_none() {
            return false;
        }
        self.commit(records, self.next).await
    }
    async fn get_record_ids(&self) -> Vec<RecordId> {
        self.records.keys().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_utils::{TestPlatform, run_jvm_test};
    use wie_backend::Platform;

    #[test]
    fn migration_and_deleted_highest_id_survive_reopen() -> wie_util::Result<()> {
        run_jvm_test(Box::new([]), |_| async {
            let platform = TestPlatform::new();
            let mut old = platform.database_repository().open("legacy", "suite").await;
            assert!(old.set(1, &[11]).await);
            assert!(old.set(7, &[77]).await);
            let mut records = RecordData::load(platform.database_repository().open("legacy", "suite").await)
                .await
                .unwrap();
            assert_eq!(records.next_id().await, 8);
            assert!(records.delete(7).await);
            drop(records);
            let mut records = RecordData::load(platform.database_repository().open("legacy", "suite").await)
                .await
                .unwrap();
            assert_eq!(records.add(&[88]).await, 8);
            assert_eq!(records.get_record_ids().await, [1, 8]);
            assert_eq!(records.get(7).await, None);
            assert_eq!(old.get(7).await, Some(alloc::vec![77]));
            assert!(records.delete(8).await);
            assert!(records.delete(1).await);
            drop(records);
            let mut records = RecordData::load(platform.database_repository().open("legacy", "suite").await)
                .await
                .unwrap();
            assert_eq!(records.add(&[]).await, 9);
            assert_eq!(records.get(9).await, Some(alloc::vec![]));
            assert_eq!(records.get(0).await, None);
            Ok(())
        })
    }

    struct RefuseWrites(Mutex<Box<dyn Database>>);
    #[async_trait::async_trait]
    impl Database for RefuseWrites {
        async fn next_id(&self) -> RecordId {
            self.0.lock().next_id().await
        }
        async fn add(&mut self, _: &[u8]) -> RecordId {
            0
        }
        async fn get(&self, id: RecordId) -> Option<Vec<u8>> {
            self.0.lock().get(id).await
        }
        async fn set(&mut self, _: RecordId, _: &[u8]) -> bool {
            false
        }
        async fn delete(&mut self, _: RecordId) -> bool {
            false
        }
        async fn get_record_ids(&self) -> Vec<RecordId> {
            self.0.lock().get_record_ids().await
        }
    }

    #[test]
    fn failed_write_does_not_consume_id_or_change_records() -> wie_util::Result<()> {
        run_jvm_test(Box::new([]), |_| async {
            let platform = TestPlatform::new();
            let mut records = RecordData::load(platform.database_repository().open("failure", "suite").await)
                .await
                .unwrap();
            assert_eq!(records.add(&[42]).await, 1);
            drop(records);
            let mut records = RecordData::load(Box::new(RefuseWrites(Mutex::new(
                platform.database_repository().open("failure", "suite").await,
            ))))
            .await
            .unwrap();
            assert_eq!(records.add(&[13]).await, 0);
            assert!(!records.set(1, &[99]).await);
            assert!(!records.delete(1).await);
            assert_eq!(records.next_id().await, 2);
            assert_eq!(records.get(1).await, Some(alloc::vec![42]));
            let records = RecordData::load(platform.database_repository().open("failure", "suite").await)
                .await
                .unwrap();
            assert_eq!(records.next_id().await, 2);
            assert_eq!(records.get(1).await, Some(alloc::vec![42]));
            Ok(())
        })
    }

    #[test]
    fn corrupt_metadata_is_not_replaced_with_legacy_data() -> wie_util::Result<()> {
        run_jvm_test(Box::new([]), |_| async {
            let platform = TestPlatform::new();
            let mut backend = platform.database_repository().open("corrupt", "suite").await;
            backend.set(1, &[42]).await;
            for malformed in [b"unknown".to_vec(), MAGIC.to_vec(), [MAGIC, &[1, 0, 0, 0, 1, 0, 0, 0]].concat()] {
                backend.set(0, &malformed).await;
                assert!(
                    RecordData::load(platform.database_repository().open("corrupt", "suite").await)
                        .await
                        .is_err()
                );
                assert_eq!(backend.get(0).await, Some(malformed));
                assert_eq!(backend.get(1).await, Some(alloc::vec![42]));
            }
            Ok(())
        })
    }
}
