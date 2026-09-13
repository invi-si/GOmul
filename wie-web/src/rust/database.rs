use crate::indexed_db_store::{Store, StoreKey};
use alloc::{
    boxed::Box,
    collections::BTreeSet,
    format,
    string::{String, ToString},
    vec::Vec,
};
use wie_backend::RecordId;

// Match the native adapter's guest path normalization. Compound IndexedDB keys
// keep database names ending in digits separate from their record IDs.
fn normalize(name: &str) -> String {
    let cleaned = name.replace(['\\', '\0'], "_");
    let parts: Vec<_> = cleaned
        .split('/')
        .filter_map(|p| match p {
            "" | "." => None,
            ".." => Some("_"),
            p => Some(p),
        })
        .collect();
    if parts.is_empty() { "_".into() } else { parts.join("/") }
}
fn key(name: &str, id: &str) -> StoreKey {
    StoreKey::Pair(name.into(), id.into())
}
fn child_of(path: &str, parent: &str) -> bool {
    path == parent || path.strip_prefix(parent).is_some_and(|tail| tail.starts_with('/'))
}
fn legacy_id(raw: &str, name: &str) -> Option<RecordId> {
    raw.strip_prefix(name).and_then(|s| s.parse().ok())
}
pub struct DatabaseRepository {
    namespace: String,
}
impl DatabaseRepository {
    pub fn new(namespace: String) -> Self {
        Self { namespace }
    }
    fn database_name(&self, app_id: &str) -> String {
        if self.namespace.is_empty() {
            format!("wie_{app_id}")
        } else {
            format!("gomul_wasm_{}:{}{}", self.namespace.len(), self.namespace, app_id)
        }
    }
    async fn store(&self, app_id: &str) -> Store {
        let name = self.database_name(app_id);
        Store::open(&name, &name).await
    }
    async fn exists_in(store: &Store, name: &str) -> bool {
        if store.get(key(name, "deleted")).await.is_some() {
            return false;
        }
        if store.get(key(name, "dir")).await.is_some() {
            return true;
        }
        store
            .get_all_keys()
            .await
            .iter()
            .any(|k| matches!(k,StoreKey::String(raw) if legacy_id(raw,name).is_some()))
    }
    async fn materialize(store: &Store, name: &str) -> anyhow::Result<()> {
        if store.get(key(name, "dir")).await.is_some() {
            return Ok(());
        }
        let deleted = store.get(key(name, "deleted")).await.is_some();
        if !deleted {
            // Retain the old interpretation for existing saves and keep every
            // original flat key as a recovery copy. New writes are unambiguous.
            for old in store.get_all_keys().await {
                if let StoreKey::String(raw) = &old {
                    if let Some(id) = legacy_id(raw, name) {
                        if let Some(data) = store.get(old.clone()).await {
                            anyhow::ensure!(store.set(key(name, &id.to_string()), &data).await, "Could not migrate browser save");
                        }
                    }
                }
            }
        }
        let mut parent = String::new();
        for part in name.split('/') {
            if !parent.is_empty() {
                parent.push('/');
            }
            parent.push_str(part);
            anyhow::ensure!(store.set(key(&parent, "dir"), &[]).await, "Could not create browser database");
        }
        if deleted {
            anyhow::ensure!(store.delete(key(name, "deleted")).await, "Could not reopen browser database");
        }
        Ok(())
    }
}
#[async_trait::async_trait]
impl wie_backend::DatabaseRepository for DatabaseRepository {
    async fn open(&self, name: &str, app_id: &str) -> Box<dyn wie_backend::Database> {
        let store = self.store(app_id).await;
        let name = normalize(name);
        if let Err(error) = Self::materialize(&store, &name).await {
            wasm_bindgen::throw_str(&error.to_string());
        }
        Box::new(Database { store, name })
    }
    async fn exists(&self, name: &str, app_id: &str) -> bool {
        Self::exists_in(&self.store(app_id).await, &normalize(name)).await
    }
    async fn delete(&self, name: &str, app_id: &str) -> bool {
        let store = self.store(app_id).await;
        let name = normalize(name);
        if !Self::exists_in(&store, &name).await {
            return false;
        }
        let mut removed = BTreeSet::new();
        removed.insert(name.clone());
        for item in store.get_all_keys().await {
            if let StoreKey::Pair(path, _) = &item {
                if child_of(path, &name) {
                    removed.insert(path.clone());
                    if !store.delete(item).await {
                        return false;
                    }
                }
            }
        }
        // Tombstones stop retained legacy keys from resurrecting deleted stores.
        for path in removed {
            if !store.set(key(&path, "deleted"), &[]).await {
                return false;
            }
        }
        true
    }
    async fn create_directory(&self, name: &str, app_id: &str) -> bool {
        let store = self.store(app_id).await;
        let name = normalize(name);
        if Self::exists_in(&store, &name).await {
            return false;
        }
        if let Some((parent, _)) = name.rsplit_once('/') {
            if !Self::exists_in(&store, parent).await || store.get(key(parent, "1")).await.is_some() {
                return false;
            }
        }
        if !store.set(key(&name, "dir"), &[]).await {
            return false;
        }
        store.delete(key(&name, "deleted")).await
    }
    async fn list_directory(&self, name: &str, app_id: &str) -> Option<Vec<String>> {
        let store = self.store(app_id).await;
        let root = name.trim_matches('/').is_empty() || name == ".";
        let path = normalize(name);
        if !root && (!Self::exists_in(&store, &path).await || store.get(key(&path, "1")).await.is_some()) {
            return None;
        }
        let prefix = if root { String::new() } else { format!("{path}/") };
        let mut names = BTreeSet::new();
        for item in store.get_all_keys().await {
            if let StoreKey::Pair(name, kind) = item {
                if kind == "dir" {
                    if let Some(tail) = name.strip_prefix(&prefix) {
                        if !tail.is_empty() && !tail.contains('/') {
                            names.insert(tail.to_string());
                        }
                    }
                }
            }
        }
        Some(names.into_iter().collect())
    }
    async fn usage(&self, app_id: &str) -> u64 {
        let store = self.store(app_id).await;
        let keys = store.get_all_keys().await;
        let claimed: Vec<_> = keys
            .iter()
            .filter_map(|k| match k {
                StoreKey::Pair(name, kind) if kind == "dir" || kind == "deleted" => Some(name.as_str()),
                _ => None,
            })
            .collect();
        let mut total = 0;
        for k in &keys {
            if matches!(k,StoreKey::String(raw) if claimed.iter().any(|name|legacy_id(raw,name).is_some())) {
                continue;
            }
            if let Some(data) = store.get(k.clone()).await {
                total += data.len() as u64;
            }
        }
        total
    }
}
pub struct Database {
    store: Store,
    name: String,
}
#[async_trait::async_trait]
impl wie_backend::Database for Database {
    async fn add(&mut self, data: &[u8]) -> RecordId {
        let id = self.next_id().await;
        self.set(id, data).await;
        id
    }
    async fn next_id(&self) -> RecordId {
        let ids = self.get_record_ids().await;
        let mut id = 1;
        while ids.contains(&id) {
            id += 1;
        }
        id
    }
    async fn get(&self, id: RecordId) -> Option<Vec<u8>> {
        self.store.get(key(&self.name, &id.to_string())).await
    }
    async fn set(&mut self, id: RecordId, data: &[u8]) -> bool {
        self.store.set(key(&self.name, &id.to_string()), data).await
    }
    async fn delete(&mut self, id: RecordId) -> bool {
        let k = key(&self.name, &id.to_string());
        if self.store.get(k.clone()).await.is_none() {
            return false;
        }
        self.store.delete(k).await
    }
    async fn get_record_ids(&self) -> Vec<RecordId> {
        self.store
            .get_all_keys()
            .await
            .into_iter()
            .filter_map(|k| match k {
                StoreKey::Pair(name, id) if name == self.name => id.parse().ok(),
                _ => None,
            })
            .collect()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn record_names_cannot_collide() {
        assert_ne!(key("save1", "1"), key("save", "11"));
    }
    #[test]
    fn paths_match_native_normalization() {
        assert_eq!(normalize("/a//./b/../c\\d"), "a/b/_/c_d");
        assert_eq!(normalize("/"), "_");
        assert!(child_of("a/b", "a"));
        assert!(!child_of("ab", "a"));
    }
    #[test]
    fn native_record_and_directory_lifecycle() {
        futures::executor::block_on(async {
            use wie_backend::DatabaseRepository as _;
            let r = DatabaseRepository::new("lifecycle-test".into());
            let app = "app";
            assert!(!r.exists("file", app).await);
            let mut file = r.open("file", app).await;
            assert!(r.exists("file", app).await);
            assert_eq!(file.add(&[1]).await, 1);
            assert_eq!(file.add(&[2]).await, 2);
            assert!(file.delete(1).await);
            assert_eq!(file.next_id().await, 1);
            assert!(!file.delete(77).await);
            let mut other = r.open("file1", app).await;
            other.set(1, &[9]).await;
            file.set(11, &[8]).await;
            assert_eq!(other.get(1).await, Some(alloc::vec![9]));
            assert_eq!(file.get(11).await, Some(alloc::vec![8]));
            assert!(r.create_directory("folder", app).await);
            assert!(!r.create_directory("folder", app).await);
            assert!(!r.create_directory("missing/child", app).await);
            assert!(r.create_directory("folder/child", app).await);
            r.open("folder/child/data", app).await.set(1, &[3]).await;
            assert_eq!(r.list_directory("folder/child", app).await, Some(alloc::vec!["data".to_string()]));
            assert_eq!(r.list_directory("file1", app).await, None);
            assert!(r.delete("folder", app).await);
            assert!(!r.exists("folder/child/data", app).await);
            assert!(r.exists("file1", app).await);
            assert!(!r.delete("folder", app).await);
        });
    }
    #[test]
    fn legacy_save_is_retained_and_does_not_resurrect_after_delete() {
        futures::executor::block_on(async {
            use wie_backend::DatabaseRepository as _;
            let r = DatabaseRepository::new("legacy-test".into());
            let store = r.store("app").await;
            store.set(StoreKey::String("prefs1".into()), &[4, 5]).await;
            assert!(r.exists("prefs", "app").await);
            let db = r.open("prefs", "app").await;
            assert_eq!(db.get(1).await, Some(alloc::vec![4, 5]));
            assert_eq!(r.usage("app").await, 2);
            assert!(r.delete("prefs", "app").await);
            assert!(!r.exists("prefs", "app").await);
            assert!(r.open("prefs", "app").await.get(1).await.is_none());
            assert_eq!(store.get(StoreKey::String("prefs1".into())).await, Some(alloc::vec![4, 5]));
        });
    }
}
