use alloc::{
    boxed::Box,
    format,
    rc::Rc,
    string::{String, ToString},
    vec,
};
use core::cell::RefCell;
use core::cmp::{max, min};

use wie_backend::Filesystem;

use crate::indexed_db_store::{Store, StoreKey};

const DB_NAME: &str = "wie_filesystem";
const STORE_NAME: &str = "files";

fn make_key(aid: &str, path: &str) -> StoreKey {
    StoreKey::Pair(aid.to_string(), path.to_string())
}

pub struct WebFilesystem {
    store: Rc<RefCell<Option<Store>>>,
    database_name: String,
}

// single threaded wasm; RefCell + Rc are only touched sequentially.
unsafe impl Send for WebFilesystem {}
unsafe impl Sync for WebFilesystem {}

impl WebFilesystem {
    pub fn new(namespace: String) -> Self {
        Self {
            store: Rc::new(RefCell::new(None)),
            database_name: if namespace.is_empty() {
                DB_NAME.into()
            } else {
                format!("gomul_wasm_fs_{namespace}")
            },
        }
    }

    async fn store(&self) -> Store {
        if let Some(store) = self.store.borrow().as_ref() {
            return store.clone();
        }
        let store = Store::open(&self.database_name, STORE_NAME).await;
        *self.store.borrow_mut() = Some(store.clone());
        store
    }
}

#[async_trait::async_trait]
impl Filesystem for WebFilesystem {
    async fn exists(&self, aid: &str, path: &str) -> bool {
        self.store().await.get(make_key(aid, path)).await.is_some()
    }

    async fn size(&self, aid: &str, path: &str) -> Option<usize> {
        self.store().await.get(make_key(aid, path)).await.map(|v| v.len())
    }

    async fn read(&self, aid: &str, path: &str, offset: usize, count: usize, buf: &mut [u8]) -> Option<usize> {
        let data = self.store().await.get(make_key(aid, path)).await?;

        if offset >= data.len() {
            return Some(0);
        }

        let size_to_read = min(count, data.len() - offset);
        buf[..size_to_read].copy_from_slice(&data[offset..offset + size_to_read]);
        Some(size_to_read)
    }

    async fn write(&self, aid: &str, path: &str, offset: usize, data: &[u8]) -> usize {
        let key = make_key(aid, path);
        let store = self.store().await;
        let existing = store.get(key.clone()).await.unwrap_or_default();
        let new_len = max(existing.len(), offset + data.len());

        let mut next = vec![0u8; new_len];
        next[..existing.len()].copy_from_slice(&existing);
        next[offset..offset + data.len()].copy_from_slice(data);

        if store.set(key, &next).await { data.len() } else { 0 }
    }

    async fn truncate(&self, aid: &str, path: &str, len: usize) {
        let key = make_key(aid, path);
        let store = self.store().await;
        let existing = store.get(key.clone()).await.unwrap_or_default();

        let mut next = vec![0u8; len];
        let copy_len = min(existing.len(), len);
        next[..copy_len].copy_from_slice(&existing[..copy_len]);

        store.set(key, &next).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sparse_writes_truncation_and_app_isolation() {
        futures::executor::block_on(async {
            let fs = WebFilesystem::new("filesystem-test".into());
            assert_eq!(fs.write("a", "file", 3, &[7]).await, 1);
            let mut buf = [1; 6];
            assert_eq!(fs.read("a", "file", 0, 6, &mut buf).await, Some(4));
            assert_eq!(&buf[..4], &[0, 0, 0, 7]);
            assert!(!fs.exists("b", "file").await);
            fs.truncate("a", "file", 6).await;
            assert_eq!(fs.size("a", "file").await, Some(6));
            fs.truncate("a", "file", 2).await;
            assert_eq!(fs.read("a", "file", 2, 6, &mut buf).await, Some(0));
            assert_eq!(fs.write("a", "empty", 0, &[]).await, 0);
            assert!(fs.exists("a", "empty").await);
        });
    }
}
