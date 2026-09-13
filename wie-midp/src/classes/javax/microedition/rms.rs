mod invalid_record_id_exception;
#[cfg(test)]
mod lifecycle_tests;
mod record_data;
mod record_store;
mod record_store_exception;
mod record_store_not_found_exception;
mod record_store_not_open_exception;
mod store_name;

pub use self::{
    invalid_record_id_exception::InvalidRecordIDException, record_store::RecordStore, record_store_exception::RecordStoreException,
    record_store_not_found_exception::RecordStoreNotFoundException, record_store_not_open_exception::RecordStoreNotOpenException,
};

/// Seed original handset data before starting the guest. Existing stores,
/// including empty ones, belong to the user and must not be replaced.
pub async fn install_packaged_records(
    system: &wie_backend::System,
    name: &str,
    next: u32,
    records: alloc::collections::BTreeMap<u32, alloc::vec::Vec<u8>>,
) -> wie_util::Result<()> {
    let name = store_name::storage_name(name);
    if system.platform().database_repository().exists(&name, system.pid()).await {
        return Ok(());
    }
    let backend = system.platform().database_repository().open(&name, system.pid()).await;
    if !record_data::RecordData::install(backend, next, records).await {
        // This call created the directory; remove a failed initial installation
        // so a later boot can retry. Guest execution has not started yet.
        system.platform().database_repository().delete(&name, system.pid()).await;
        return Err(wie_util::WieError::FatalError("Could not install packaged RMS data".into()));
    }
    Ok(())
}

#[cfg(test)]
mod import_tests {
    use super::*;
    use alloc::{boxed::Box, collections::BTreeMap, vec};
    use jvm::{Array, ClassInstanceRef, runtime::JavaLangString};
    use test_utils::{TestPlatform, run_jvm_test_with_system};

    #[test]
    fn imported_records_are_guest_visible_and_never_replace_existing_data() -> wie_util::Result<()> {
        run_jvm_test_with_system(
            Box::new([crate::get_protos().into()]),
            Box::new(TestPlatform::new()),
            |jvm, system| async move {
                const CLASS: &str = "javax/microedition/rms/RecordStore";
                install_packaged_records(&system, "Save", 8, BTreeMap::from([(1, vec![1, 2]), (7, vec![7])]))
                    .await
                    .unwrap();
                let name = JavaLangString::from_rust_string(&jvm, "Save").await?;
                let store: ClassInstanceRef<RecordStore> = jvm
                    .invoke_static(
                        CLASS,
                        "openRecordStore",
                        "(Ljava/lang/String;Z)Ljavax/microedition/rms/RecordStore;",
                        (name, false),
                    )
                    .await?;
                let bytes: ClassInstanceRef<Array<i8>> = jvm.invoke_virtual(&store, CLASS, "getRecord", "(I)[B", (1,)).await?;
                assert_eq!(jvm.load_array::<i8>(&bytes, 0, 2).await?, [1, 2]);
                let next: i32 = jvm.invoke_virtual(&store, CLASS, "getNextRecordID", "()I", ()).await?;
                assert_eq!(next, 8);
                for id in [1, 7] {
                    let _: () = jvm.invoke_virtual(&store, CLASS, "deleteRecord", "(I)V", (id,)).await?;
                }
                install_packaged_records(&system, "Save", 2, BTreeMap::from([(1, vec![99])]))
                    .await
                    .unwrap();
                let count: i32 = jvm.invoke_virtual(&store, CLASS, "getNumRecords", "()I", ()).await?;
                assert_eq!(count, 0);
                let empty: ClassInstanceRef<Array<i8>> = None.into();
                let id: i32 = jvm.invoke_virtual(&store, CLASS, "addRecord", "([BII)I", (empty, 0, 0)).await?;
                assert_eq!(id, 8);
                let mut legacy = system.platform().database_repository().open("Legacy", system.pid()).await;
                legacy.set(1, &[42]).await;
                install_packaged_records(&system, "Legacy", 2, BTreeMap::from([(1, vec![99])]))
                    .await
                    .unwrap();
                assert_eq!(legacy.get(1).await, Some(vec![42]));
                assert_eq!(legacy.get(0).await, None);
                Ok(())
            },
        )
    }
}
