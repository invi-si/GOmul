use core::{mem::size_of, pin::Pin, task::Poll};

use alloc::{borrow::ToOwned, boxed::Box, collections::BTreeMap, format, string::String, vec, vec::Vec};

use bytemuck::Zeroable;
use futures::future::poll_fn;
use jvm::{ClassInstance, Result as JvmResult, runtime::JavaLangString};

use wie_backend::{Emulator, Event, Options, Platform, System, TaskRunner};
use wie_core_arm::{Allocator, ArmCore};
use wie_jvm_support::JvmSupport;
use wie_util::{Result, WieError, write_generic};

use crate::{
    adf::{KtfAdf, find_client_bin},
    runtime::{KtfJvmSupport, KtfJvmThreadContext},
};

pub const IMAGE_BASE: u32 = 0x100000;

struct KtfTaskRunner {
    core: ArmCore,
}

#[async_trait::async_trait]
impl TaskRunner for KtfTaskRunner {
    async fn run(&self, mut future: Pin<Box<dyn Future<Output = Result<()>> + Send>>) -> Result<()> {
        let mut core = self.core.clone();
        let ptr_thread_context = Allocator::alloc(&mut core, size_of::<KtfJvmThreadContext>() as u32)?;
        write_generic(&mut core, ptr_thread_context, KtfJvmThreadContext::zeroed())?;

        let mut poll_core = self.core.clone();
        let result = self
            .core
            .run_in_thread(move || {
                poll_fn(move |context| {
                    // KTF's first native init argument points at this cell and dereferences it for each stack check and try block.
                    if let Err(error) = KtfJvmSupport::set_current_thread_context(&mut poll_core, ptr_thread_context) {
                        return Poll::Ready(Err(error));
                    }

                    future.as_mut().poll(context)
                })
            })?
            .await;

        Allocator::free(&mut core, ptr_thread_context, size_of::<KtfJvmThreadContext>() as u32)?;

        result
    }
}

pub struct KtfEmulator {
    core: ArmCore,
    system: System,
}

impl Drop for KtfEmulator {
    fn drop(&mut self) {
        self.system.shutdown();
        self.core.shutdown();
    }
}

impl KtfEmulator {
    pub fn from_archive(platform: Box<dyn Platform>, files: BTreeMap<String, Vec<u8>>, options: Options) -> Result<Self> {
        let adf = files
            .get("__adf__")
            .ok_or_else(|| WieError::FatalError("Missing __adf__ in KTF archive".into()))?;
        let adf = KtfAdf::parse(adf);

        tracing::info!("Loading app {}, pid {}, mclass {}", adf.aid, adf.pid, adf.mclass);
        if let Some((width, height)) = adf.display_size
            && let Err(error) = platform.screen().resize(width, height)
        {
            tracing::warn!("Ignoring unsupported display size {width}x{height}: {error}");
        }

        let jar_filename = format!("{}.jar", adf.aid);

        Self::load(platform, &jar_filename, &adf.pid, &adf.aid, Some(adf.mclass), &files, options)
    }

    pub fn from_jar(
        platform: Box<dyn Platform>,
        jar_filename: &str,
        jar: Vec<u8>,
        pid: &str,
        aid: &str,
        main_class_name: Option<String>,
        options: Options,
    ) -> Result<Self> {
        let files = [(jar_filename.to_owned(), jar)].into_iter().collect();

        Self::load(platform, jar_filename, pid, aid, main_class_name, &files, options)
    }

    pub fn loadable_archive(files: &BTreeMap<String, Vec<u8>>) -> bool {
        files.contains_key("__adf__")
    }

    pub fn archive_title(files: &BTreeMap<String, Vec<u8>>) -> Option<String> {
        let title = KtfAdf::parse(files.get("__adf__")?).name;
        (!title.is_empty()).then_some(title)
    }

    pub fn archive_id(files: &BTreeMap<String, Vec<u8>>) -> Option<String> {
        let id = KtfAdf::parse(files.get("__adf__")?).pid;
        (!id.is_empty()).then_some(id)
    }

    pub fn archive_icon(files: &BTreeMap<String, Vec<u8>>) -> Option<Vec<u8>> {
        files.get("big.icon").cloned()
    }

    pub fn loadable_jar(jar: &[u8]) -> bool {
        find_client_bin(jar).is_ok()
    }

    fn load(
        platform: Box<dyn Platform>,
        jar_filename: &str,
        pid: &str,
        aid: &str,
        main_class_name: Option<String>,
        files: &BTreeMap<String, Vec<u8>>,
        mut options: Options,
    ) -> Result<Self> {
        if files.get(jar_filename).is_some_and(|jar| jar.starts_with(b"odcf")) {
            return Err(WieError::FatalError(
                "This game contains an unsupported ODCF-wrapped payload instead of a plain JAR. Import a compatible plain JAR package.".into(),
            ));
        }
        let mut core = ArmCore::new(options.enable_gdbserver, options.profile.take())?;
        let system = System::new(platform, pid, aid, KtfTaskRunner { core: core.clone() });

        let mut private_data = BTreeMap::new();
        for (path, data) in files {
            if let Some(name) = path.strip_prefix("P/").or_else(|| path.strip_prefix("p/")) {
                private_data.insert(name.to_owned(), data.clone());
                // Preserve the existing filesystem view for Java file callers.
                // Native database opens use the mutable imported records instead.
                system.filesystem().add_virtual(name, data.clone());
            } else {
                system.filesystem().add_virtual(path, data.clone());
            }
        }

        Allocator::init(&mut core)?;

        let mut core_clone = core.clone();
        let mut system_clone = system.clone();
        let jar_filename_clone = jar_filename.to_owned();
        system.spawn(async move || {
            install_private_data(&system_clone, &private_data).await?;
            Self::start(&mut core_clone, &mut system_clone, jar_filename_clone, main_class_name).await
        });

        Ok(Self { core, system })
    }

    #[tracing::instrument(name = "start", skip_all)]
    async fn start(core: &mut ArmCore, system: &mut System, jar_filename: String, main_class_name: Option<String>) -> Result<()> {
        let (jvm, class_loader) = KtfJvmSupport::init(core, system, Some(&jar_filename)).await?;

        let main_class_name = if let Some(x) = main_class_name {
            x
        } else {
            return Err(WieError::FatalError("Main class not found".into()));
        };

        let main_class_name = main_class_name.replace('.', "/");

        let main_class_name_java = JavaLangString::from_rust_string(&jvm, &main_class_name).await.unwrap();
        let _main_class: Box<dyn ClassInstance> = jvm
            .invoke_virtual(
                &class_loader,
                "net/wie/KtfClassLoader",
                "loadClass",
                "(Ljava/lang/String;)Ljava/lang/Class;",
                (main_class_name_java.clone(),),
            )
            .await
            .unwrap();

        let mut args_array = jvm.instantiate_array("Ljava/lang/String;", 1).await.unwrap();
        jvm.store_array(&mut args_array, 0, vec![main_class_name_java]).await.unwrap();
        let result: JvmResult<()> = jvm
            .invoke_static("org/kwis/msp/lcdui/Main", "main", "([Ljava/lang/String;)V", (args_array,))
            .await;

        if let Err(x) = result {
            return Err(JvmSupport::to_wie_err(&jvm, x).await);
        }

        Ok(())
    }
}

impl Emulator for KtfEmulator {
    fn handle_event(&mut self, event: Event) {
        self.system.event_queue().push(event)
    }

    fn tick(&mut self) -> Result<()> {
        self.system.tick().map_err(|x| {
            let reg_stack = self.core.dump_reg_stack(IMAGE_BASE);
            match x {
                WieError::FatalError(msg) => WieError::FatalError(format!("{msg}\n{reg_stack}")),
                _ => WieError::FatalError(format!("{x}\n{reg_stack}")),
            }
        })
    }
}

// Installation state is persisted with the application's save files, so resets
// and replay snapshots include it. It is not a host-side runtime registry.
// Observed dense KTF Java database export: 45-byte qtpdb index, big-endian
// record width/count, no deleted slots, and matching contiguous .db payload.
// Other index variants are left untouched rather than assigning guessed IDs.
fn dense_java_records<'a>(index: &[u8], bytes: &'a [u8]) -> Option<core::slice::ChunksExact<'a, u8>> {
    if index.len() != 45 || !index.starts_with(b"qtpdb") {
        return None;
    }
    let word = |offset| u32::from_be_bytes(index[offset..offset + 4].try_into().unwrap());
    let width = word(5) as usize;
    let count = word(9) as usize;
    if width == 0 || count == 0 || word(13) != 0 || word(17) as usize != count || width.checked_mul(count)? != bytes.len() {
        return None;
    }
    Some(bytes.chunks_exact(width))
}

async fn install_private_data(system: &System, data: &BTreeMap<String, Vec<u8>>) -> Result<()> {
    const INSTALLED: &str = "__gomul_ktf_private_data__";
    if data.is_empty() {
        return Ok(());
    }
    if [system.pid(), system.aid()]
        .iter()
        .any(|part| part.is_empty() || *part == "." || *part == ".." || part.contains(['/', '\\']))
    {
        return Err(WieError::FatalError("Invalid KTF installation identity".into()));
    }
    if system.filesystem().exists(INSTALLED).await {
        return Ok(());
    }
    for (name, bytes) in data {
        if name.starts_with('/') || !system.filesystem().is_valid_path(name) {
            return Err(WieError::FatalError("Unsafe private-data path in KTF package".into()));
        }
        if !system.platform().database_repository().exists(name, system.pid()).await {
            system.platform().database_repository().open(name, system.pid()).await.set(1, bytes).await;
        }
        if let Some(base) = name.strip_suffix(".idx") {
            if let Some(records) = data
                .get(&alloc::format!("{base}.db"))
                .and_then(|payload| dense_java_records(bytes, payload))
            {
                if !system.platform().database_repository().exists(base, system.pid()).await {
                    let mut database = system.platform().database_repository().open(base, system.pid()).await;
                    for (index, record) in records.enumerate() {
                        database.set(index as u32 + 1, record).await;
                    }
                }
            }
        }
    }
    if system.filesystem().write(INSTALLED, 0, &[1]).await != 1 {
        return Err(WieError::FatalError("Could not persist KTF private-data installation".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use alloc::{boxed::Box, sync::Arc, vec::Vec};
    use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

    use test_utils::TestPlatform;
    use wie_backend::{System, YieldFuture};
    use wie_core_arm::{Allocator, ArmCore};
    use wie_util::{Result, WieError};

    fn dense_index(width: u32, count: u32) -> Vec<u8> {
        let mut index = alloc::vec![0; 45];
        index[..5].copy_from_slice(b"qtpdb");
        for (offset, value) in [(5, width), (9, count), (17, count)] {
            index[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
        }
        index
    }

    #[test]
    fn dense_java_database_import_splits_records_and_preserves_progress() -> Result<()> {
        use futures::FutureExt;
        let system = System::new(Box::new(TestPlatform::new()), "PID", "AID", wie_backend::DefaultTaskRunner);
        let data = [
            ("state.idx".into(), dense_index(2, 3)),
            ("state.db".into(), alloc::vec![1, 2, 3, 4, 5, 6]),
            ("existing.idx".into(), dense_index(2, 1)),
            ("existing.db".into(), alloc::vec![0, 0]),
        ]
        .into_iter()
        .collect();
        async {
            let repository = system.platform().database_repository();
            repository.open("existing", system.pid()).await.set(1, &[99]).await;
            super::install_private_data(&system, &data).await?;
            let database = repository.open("state", system.pid()).await;
            assert_eq!(database.get(1).await, Some(alloc::vec![1, 2]));
            assert_eq!(database.get(2).await, Some(alloc::vec![3, 4]));
            assert_eq!(database.get(3).await, Some(alloc::vec![5, 6]));
            assert_eq!(database.get(4).await, None);
            assert_eq!(repository.open("existing", system.pid()).await.get(1).await, Some(alloc::vec![99]));
            repository.delete("state", system.pid()).await;
            super::install_private_data(&system, &data).await?;
            assert!(!repository.exists("state", system.pid()).await);
            Ok(())
        }
        .now_or_never()
        .expect("TestPlatform operations are synchronous")
    }

    #[test]
    fn unknown_sparse_and_mismatched_java_indexes_are_not_imported() {
        let good = dense_index(2, 3);
        for (offset, value) in [(4, 255), (13, 1), (17, 1)] {
            let mut index = good.clone();
            index[offset] = value;
            assert!(super::dense_java_records(&index, &[0; 6]).is_none());
        }
        assert!(super::dense_java_records(&good[..44], &[0; 6]).is_none());
        assert!(super::dense_java_records(&good, &[0; 5]).is_none());
        assert!(super::dense_java_records(&dense_index(0, 3), &[]).is_none());
    }

    use super::{KtfJvmSupport, KtfTaskRunner};

    #[test]
    fn private_data_import_preserves_updates_deletions_and_existing_saves() -> Result<()> {
        use futures::FutureExt;
        let system = System::new(Box::new(TestPlatform::new()), "PID", "AID", wie_backend::DefaultTaskRunner);
        let data = [
            ("first.bin".into(), alloc::vec![10, 11]),
            ("Mixed.BIN".into(), alloc::vec![20]),
            ("p/nested.bin".into(), alloc::vec![30]),
        ]
        .into_iter()
        .collect();
        let test = async {
            let repository = system.platform().database_repository();
            repository.open("Mixed.BIN", system.pid()).await.set(1, &[99]).await;
            super::install_private_data(&system, &data).await?;
            assert_eq!(repository.open("first.bin", system.pid()).await.get(1).await, Some(alloc::vec![10, 11]));
            assert_eq!(repository.open("Mixed.BIN", system.pid()).await.get(1).await, Some(alloc::vec![99]));
            assert!(!repository.exists("mixed.bin", system.pid()).await);
            assert!(repository.exists("p/nested.bin", system.pid()).await);
            repository.delete("first.bin", system.pid()).await;
            repository.open("Mixed.BIN", system.pid()).await.set(1, &[42]).await;
            super::install_private_data(&system.clone(), &data).await?;
            assert!(!repository.exists("first.bin", system.pid()).await);
            assert_eq!(repository.open("Mixed.BIN", system.pid()).await.get(1).await, Some(alloc::vec![42]));
            Ok(())
        };
        test.now_or_never().expect("TestPlatform operations are synchronous")
    }

    #[test]
    fn odcf_payload_is_rejected_before_jvm_initialization() {
        let result = super::KtfEmulator::from_jar(
            Box::new(TestPlatform::new()),
            "wrapped.jar",
            alloc::vec![b'o', b'd', b'c', b'f', 0, 2, 0, 0],
            "test",
            "test",
            Some("Main".into()),
            wie_backend::Options {
                enable_gdbserver: false,
                profile: None,
            },
        );
        assert!(matches!(result, Err(WieError::FatalError(message)) if message.contains("ODCF")));
    }

    #[test]
    fn switches_jvm_thread_context_between_tasks() -> Result<()> {
        let mut core = ArmCore::new(false, None)?;
        Allocator::init(&mut core)?;

        let mut system = System::new(Box::new(TestPlatform::new()), "", "", KtfTaskRunner { core: core.clone() });
        let contexts = Arc::new([AtomicU32::new(0), AtomicU32::new(0)]);
        let completed = Arc::new(AtomicUsize::new(0));

        for index in 0..2 {
            let core = core.clone();
            let contexts = contexts.clone();
            let completed = completed.clone();
            system.spawn(async move || {
                let before = KtfJvmSupport::current_thread_context(&core)?;
                YieldFuture::new().await;
                let after = KtfJvmSupport::current_thread_context(&core)?;
                if before != after {
                    return Err(WieError::FatalError("KTF JVM thread context changed while the task was suspended".into()));
                }

                contexts[index].store(before, Ordering::Relaxed);
                completed.fetch_add(1, Ordering::Relaxed);

                Ok(())
            });
        }

        while completed.load(Ordering::Relaxed) != 2 {
            system.tick()?;
        }

        let first = contexts[0].load(Ordering::Relaxed);
        let second = contexts[1].load(Ordering::Relaxed);
        assert_ne!(first, 0);
        assert_ne!(second, 0);
        assert_ne!(first, second);

        Ok(())
    }
}
