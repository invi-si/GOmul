use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use directories::ProjectDirs;

use wie_backend::RecordId;

pub struct DatabaseRepository {
    base_path: PathBuf,
}

impl DatabaseRepository {
    pub fn at_path(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    pub fn new() -> Self {
        let base_dir = ProjectDirs::from("net", "dlunch", "wie").unwrap();

        let base_path = base_dir.data_dir().to_owned();

        Self { base_path }
    }

    fn get_path_for_database(&self, name: &str, app_id: &str) -> PathBuf {
        let sanitized_app_id: String = app_id.chars().filter(|c| !matches!(c, '/' | '\\' | '\0')).collect();
        let app_id = if sanitized_app_id.is_empty() || sanitized_app_id == "." || sanitized_app_id == ".." {
            "_"
        } else {
            &sanitized_app_id
        };

        let name: String = name.chars().map(|c| if matches!(c, '\\' | '\0') { '_' } else { c }).collect();
        let mut normalized_name = PathBuf::new();
        for segment in name.trim_start_matches('/').split('/') {
            match segment {
                "" | "." => {}
                ".." => normalized_name.push("_"),
                segment => normalized_name.push(segment),
            }
        }
        if normalized_name.as_os_str().is_empty() {
            normalized_name.push("_");
        }

        self.base_path.join(app_id).join("db").join(normalized_name)
    }

    fn get_path_for_app_databases(&self, app_id: &str) -> PathBuf {
        self.get_path_for_database("_", app_id).parent().unwrap().to_owned()
    }

    fn directory_usage(path: &std::path::Path) -> u64 {
        let Ok(entries) = fs::read_dir(path) else {
            return 0;
        };

        entries
            .filter_map(Result::ok)
            .map(|entry| {
                let Ok(file_type) = entry.file_type() else {
                    return 0;
                };
                if file_type.is_file() {
                    entry.metadata().map(|metadata| metadata.len()).unwrap_or(0)
                } else if file_type.is_dir() {
                    Self::directory_usage(&entry.path())
                } else {
                    0
                }
            })
            .sum()
    }
}

#[async_trait::async_trait]
impl wie_backend::DatabaseRepository for DatabaseRepository {
    async fn open(&self, name: &str, app_id: &str) -> Box<dyn wie_backend::Database> {
        let path = self.get_path_for_database(name, app_id);

        Box::new(Database::new(path).unwrap())
    }

    async fn exists(&self, name: &str, app_id: &str) -> bool {
        let path = self.get_path_for_database(name, app_id);

        path.exists()
    }

    async fn delete(&self, name: &str, app_id: &str) -> bool {
        let path = self.get_path_for_database(name, app_id);

        tracing::trace!("Delete database at {path:?}");

        match fs::remove_dir_all(path) {
            Ok(()) => true,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
            Err(e) => {
                tracing::warn!("Failed to delete database: {e}");
                false
            }
        }
    }

    async fn create_directory(&self, name: &str, app_id: &str) -> bool {
        let root = self.get_path_for_app_databases(app_id);
        if fs::create_dir_all(&root).is_err() {
            return false;
        }
        let path = self.get_path_for_database(name, app_id);
        // A file is represented by a database directory with numbered records.
        // Do not create a guest child inside such a file.
        if path.parent().is_some_and(|p| p.join("1").is_file()) {
            return false;
        }
        fs::create_dir(path).is_ok()
    }

    async fn list_directory(&self, name: &str, app_id: &str) -> Option<Vec<String>> {
        let root = self.get_path_for_app_databases(app_id);
        let path = if name.trim_matches('/').is_empty() || name == "." {
            root
        } else {
            self.get_path_for_database(name, app_id)
        };
        if path.join("1").is_file() {
            return None;
        }
        if !path.exists() && (name.trim_matches('/').is_empty() || name == ".") {
            return Some(Vec::new());
        }
        let mut names = Vec::new();
        for entry in fs::read_dir(path).ok()? {
            let entry = entry.ok()?;
            if entry.file_type().ok()?.is_dir() {
                names.push(entry.file_name().into_string().ok()?);
            }
        }
        names.sort();
        Some(names)
    }

    async fn usage(&self, app_id: &str) -> u64 {
        Self::directory_usage(&self.get_path_for_app_databases(app_id))
    }
}

pub struct Database {
    base_path: PathBuf,
}

impl Database {
    pub fn new(base_path: PathBuf) -> anyhow::Result<Self> {
        tracing::trace!("Opening database at {base_path:?}");

        fs::create_dir_all(&base_path)?;

        Ok(Self { base_path })
    }

    fn find_empty_record_id(&self) -> RecordId {
        let mut record_id = 1; // XXX midp requires first record to be 1

        loop {
            let path = self.base_path.join(record_id.to_string());

            if !path.exists() {
                return record_id;
            }

            record_id += 1;
        }
    }
    fn get_path_for_record(&self, id: RecordId) -> PathBuf {
        self.base_path.join(id.to_string())
    }

    fn replace_record(&self, id: RecordId, write: impl FnOnce(&mut fs::File) -> io::Result<()>) -> io::Result<()> {
        static NEXT_WRITE: AtomicU64 = AtomicU64::new(0);
        let (temporary, mut file) = loop {
            let sequence = NEXT_WRITE.fetch_add(1, Ordering::Relaxed);
            let path = self.base_path.join(format!(".gomul-record-{}-{sequence}.tmp", std::process::id()));
            match fs::OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => break (path, file),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        };
        let result = write(&mut file);
        drop(file);
        // A failed or interrupted write must leave the previous record intact.
        // Same-directory rename publishes only the complete replacement.
        let result = result.and_then(|()| fs::rename(&temporary, self.get_path_for_record(id)));
        if result.is_err() {
            let _ = fs::remove_file(temporary);
        }
        result
    }
}

#[async_trait::async_trait]
impl wie_backend::Database for Database {
    async fn next_id(&self) -> RecordId {
        self.find_empty_record_id()
    }

    async fn add(&mut self, data: &[u8]) -> RecordId {
        let id = self.find_empty_record_id();

        tracing::trace!("Adding record {id} to database {:?}", &self.base_path);

        self.replace_record(id, |file| file.write_all(data)).unwrap();

        id
    }

    async fn get(&self, id: RecordId) -> Option<Vec<u8>> {
        let path = self.get_path_for_record(id);

        tracing::trace!("Read record {id} from database {:?}", &self.base_path);

        fs::read(path).ok()
    }

    async fn set(&mut self, id: RecordId, data: &[u8]) -> bool {
        tracing::trace!("Set record {id} to database {:?}", &self.base_path);

        self.replace_record(id, |file| file.write_all(data)).is_ok()
    }

    async fn delete(&mut self, id: RecordId) -> bool {
        let path = self.get_path_for_record(id);

        tracing::trace!("Delete record {id} from database {:?}", &self.base_path);

        fs::remove_file(path).is_ok()
    }

    async fn get_record_ids(&self) -> Vec<RecordId> {
        fs::read_dir(&self.base_path)
            .unwrap()
            // Interrupted writes and imported sidecar files are not guest records.
            .filter_map(|entry| {
                let entry = entry.ok()?;
                entry.file_type().ok()?.is_file().then(|| entry.file_name().to_str()?.parse().ok())?
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::DatabaseRepository;

    #[test]
    fn database_path_includes_db_segment() {
        let repo = DatabaseRepository {
            base_path: PathBuf::from("/tmp/wie_test"),
        };
        let path = repo.get_path_for_database("records", "game123");
        assert_eq!(path, PathBuf::from("/tmp/wie_test/game123/db/records"));
    }

    #[test]
    fn database_path_strips_guest_leading_slash() {
        let repo = DatabaseRepository {
            base_path: PathBuf::from("/tmp/wie_test"),
        };
        let path = repo.get_path_for_database("/save0.dat", "PD140106");
        assert_eq!(path, PathBuf::from("/tmp/wie_test/PD140106/db/save0.dat"));
    }

    #[test]
    fn database_path_does_not_escape_app_scope() {
        let repo = DatabaseRepository {
            base_path: PathBuf::from("/tmp/wie_test"),
        };
        let path = repo.get_path_for_database("/../save0.dat", "PD140106");
        assert!(path.starts_with(PathBuf::from("/tmp/wie_test/PD140106/db")));
    }

    #[test]
    fn failed_replacement_preserves_record_and_cleans_partial_write() {
        use super::*;
        let root = std::env::temp_dir().join(format!("gomul-atomic-record-{}", std::process::id()));
        let db = Database::new(root.clone()).unwrap();
        fs::write(db.get_path_for_record(1), b"original progress").unwrap();
        let error = db.replace_record(1, |file| {
            file.write_all(b"partial")?;
            Err(io::Error::other("simulated disk write failure"))
        });
        assert!(error.is_err());
        assert_eq!(fs::read(db.get_path_for_record(1)).unwrap(), b"original progress");
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        db.replace_record(1, |file| file.write_all(b"new")).unwrap();
        assert_eq!(fs::read(db.get_path_for_record(1)).unwrap(), b"new");
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn interrupted_write_and_sidecar_are_not_records() {
        use super::*;
        use wie_backend::Database as _;
        let root = std::env::temp_dir().join(format!("gomul-record-list-{}", std::process::id()));
        let db = Database::new(root.clone()).unwrap();
        for (name, bytes) in [
            ("1", b"complete".as_slice()),
            (".gomul-record-interrupted.tmp", b"partial"),
            (".DS_Store", b"sidecar"),
        ] {
            fs::write(root.join(name), bytes).unwrap();
        }
        let mut ids = db.get_record_ids();
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        assert_eq!(ids.as_mut().poll(&mut context), std::task::Poll::Ready(vec![1]));
        fs::remove_dir_all(root).unwrap();
    }
}
