//! In-memory transport for testing the real browser database/filesystem adapters.
use alloc::{collections::BTreeMap, string::String, vec::Vec};
use std::sync::{Arc, Mutex, OnceLock};
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StoreKey {
    String(String),
    Pair(String, String),
}
type Records = Arc<Mutex<BTreeMap<StoreKey, Vec<u8>>>>;
#[derive(Clone)]
pub struct Store(Records);
impl Store {
    pub async fn open(database: &str, _store: &str) -> Self {
        static DATABASES: OnceLock<Mutex<BTreeMap<String, Records>>> = OnceLock::new();
        Self(
            DATABASES
                .get_or_init(|| Mutex::new(BTreeMap::new()))
                .lock()
                .unwrap()
                .entry(database.into())
                .or_default()
                .clone(),
        )
    }
    pub async fn get_all_keys(&self) -> Vec<StoreKey> {
        self.0.lock().unwrap().keys().cloned().collect()
    }
    pub async fn get(&self, key: StoreKey) -> Option<Vec<u8>> {
        self.0.lock().unwrap().get(&key).cloned()
    }
    pub async fn set(&self, key: StoreKey, data: &[u8]) -> bool {
        self.0.lock().unwrap().insert(key, data.into());
        true
    }
    pub async fn delete(&self, key: StoreKey) -> bool {
        self.0.lock().unwrap().remove(&key);
        true
    }
}
