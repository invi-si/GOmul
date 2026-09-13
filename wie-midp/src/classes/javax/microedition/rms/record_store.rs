use alloc::{borrow::ToOwned, boxed::Box, vec, vec::Vec};

use bytemuck::cast_vec;

use jvm::{Array, ClassInstanceRef, Jvm, Result as JvmResult, runtime::JavaLangString};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use rustjava_runtime::classes::java::lang::String;

use wie_backend::Database;
use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

use super::record_data::RecordData;

// class javax.microedition.rms.RecordStore
pub struct RecordStore;

impl RecordStore {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "javax/microedition/rms/RecordStore",
            parent_class: Some("java/lang/Object"),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new(
                    "wieClose",
                    "(Ljavax/microedition/rms/RecordStore;)V",
                    Self::close_locked,
                    MethodAccessFlags::PRIVATE | MethodAccessFlags::STATIC | MethodAccessFlags::SYNCHRONIZED,
                ),
                JavaMethodProto::new(
                    "getName",
                    "()Ljava/lang/String;",
                    Self::get_name,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::SYNCHRONIZED,
                ),
                // A guest Class monitor serializes mutations across independently
                // opened handles. No host-side open-store registry is needed.
                JavaMethodProto::new(
                    "wieWrite",
                    "(Ljavax/microedition/rms/RecordStore;II[BII)I",
                    Self::write_locked,
                    MethodAccessFlags::PRIVATE | MethodAccessFlags::STATIC | MethodAccessFlags::SYNCHRONIZED,
                ),
                JavaMethodProto::new("<init>", "(Ljava/lang/String;)V", Self::init, MethodAccessFlags::PRIVATE),
                JavaMethodProto::new(
                    "addRecord",
                    "([BII)I",
                    Self::add_record,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::SYNCHRONIZED,
                ),
                JavaMethodProto::new(
                    "deleteRecord",
                    "(I)V",
                    Self::delete_record,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::SYNCHRONIZED,
                ),
                JavaMethodProto::new(
                    "getSizeAvailable",
                    "()I",
                    Self::get_size_available,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::SYNCHRONIZED,
                ),
                JavaMethodProto::new(
                    "getNextRecordID",
                    "()I",
                    Self::get_next_record_id,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::SYNCHRONIZED,
                ),
                JavaMethodProto::new(
                    "getRecord",
                    "(I)[B",
                    Self::get_record,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::SYNCHRONIZED,
                ),
                JavaMethodProto::new(
                    "getRecord",
                    "(I[BI)I",
                    Self::get_record_array,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::SYNCHRONIZED,
                ),
                JavaMethodProto::new(
                    "getRecordSize",
                    "(I)I",
                    Self::get_record_size,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::SYNCHRONIZED,
                ),
                JavaMethodProto::new(
                    "setRecord",
                    "(I[BII)V",
                    Self::set_record,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::SYNCHRONIZED,
                ),
                JavaMethodProto::new(
                    "getNumRecords",
                    "()I",
                    Self::get_num_records,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::SYNCHRONIZED,
                ),
                JavaMethodProto::new(
                    "closeRecordStore",
                    "()V",
                    Self::close_record_store,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::SYNCHRONIZED,
                ),
                JavaMethodProto::new(
                    "openRecordStore",
                    "(Ljava/lang/String;Z)Ljavax/microedition/rms/RecordStore;",
                    Self::open_record_store,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC | MethodAccessFlags::SYNCHRONIZED,
                ),
                JavaMethodProto::new(
                    "deleteRecordStore",
                    "(Ljava/lang/String;)V",
                    Self::delete_record_store,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC | MethodAccessFlags::SYNCHRONIZED,
                ),
                JavaMethodProto::new(
                    "listRecordStores",
                    "()[Ljava/lang/String;",
                    Self::list_record_stores,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC | MethodAccessFlags::SYNCHRONIZED,
                ),
            ],
            fields: vec![
                JavaFieldProto::new("dbName", "Ljava/lang/String;", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("openCount", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("nextOpen", "Ljavax/microedition/rms/RecordStore;", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new(
                    "openStores",
                    "Ljavax/microedition/rms/RecordStore;",
                    FieldAccessFlags::PRIVATE | FieldAccessFlags::STATIC,
                ),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }

    async fn init(jvm: &Jvm, _context: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, db_name: ClassInstanceRef<String>) -> JvmResult<()> {
        tracing::debug!("javax.microedition.rms.RecordStore::<init>({this:?}, {db_name:?})");

        let _: () = jvm.invoke_special(&this, "java/lang/Object", "<init>", "()V", ()).await?;

        jvm.put_field(&mut this, "dbName", "Ljava/lang/String;", db_name).await?;

        Ok(())
    }

    async fn add_record(
        jvm: &Jvm,
        _context: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        data: ClassInstanceRef<Array<i8>>,
        offset: i32,
        length: i32,
    ) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.rms.RecordStore::addRecord({this:?}, {data:?}, {offset}, {length})");

        jvm.invoke_static(
            "javax/microedition/rms/RecordStore",
            "wieWrite",
            "(Ljavax/microedition/rms/RecordStore;II[BII)I",
            (this, 0, 0, data, offset, length),
        )
        .await
    }

    async fn delete_record(jvm: &Jvm, _context: &mut WieJvmContext, this: ClassInstanceRef<Self>, record_id: i32) -> JvmResult<()> {
        let empty: ClassInstanceRef<Array<i8>> = None.into();
        let _: i32 = jvm
            .invoke_static(
                "javax/microedition/rms/RecordStore",
                "wieWrite",
                "(Ljavax/microedition/rms/RecordStore;II[BII)I",
                (this, 1, record_id, empty, 0, 0),
            )
            .await?;
        Ok(())
    }

    async fn get_size_available(jvm: &Jvm, _context: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::warn!("stub javax.microedition.rms.RecordStore::getSizeAvailable({this:?})");

        Self::ensure_open(jvm, &this).await?;
        Ok(1000000)
    }

    async fn get_next_record_id(jvm: &Jvm, context: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.rms.RecordStore::getNextRecordID({this:?})");

        let database = Self::get_database(jvm, context, &this).await?;

        let next_id = database.next_id().await;

        Ok(next_id as _)
    }

    async fn get_record(
        jvm: &Jvm,
        context: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        record_id: i32,
    ) -> JvmResult<ClassInstanceRef<Array<i8>>> {
        tracing::debug!("javax.microedition.rms.RecordStore::getRecord({this:?}, {record_id})");

        let database = Self::get_database(jvm, context, &this).await?;

        let result = database.get(record_id as _).await;
        if result.is_none() {
            return Err(jvm.exception("javax/microedition/rms/InvalidRecordIDException", "Record not found").await);
        }

        let data = result.unwrap();

        let mut array = jvm.instantiate_array("B", data.len() as _).await?;
        jvm.store_array(&mut array, 0, cast_vec::<u8, i8>(data)).await?;

        Ok(array.into())
    }

    async fn get_record_array(
        jvm: &Jvm,
        context: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        record_id: i32,
        mut buffer: ClassInstanceRef<Array<i8>>,
        offset: i32,
    ) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.rms.RecordStore::getRecord({this:?}, {record_id}, {buffer:?}, {offset})");

        let database = Self::get_database(jvm, context, &this).await?;

        let result = database.get(record_id as _).await;
        if result.is_none() {
            return Err(jvm.exception("javax/microedition/rms/InvalidRecordIDException", "Record not found").await);
        }

        let data = result.unwrap();
        let data_length = data.len();
        jvm.store_array(&mut buffer, offset as _, cast_vec::<u8, i8>(data)).await?;

        Ok(data_length as _)
    }

    async fn get_record_size(jvm: &Jvm, context: &mut WieJvmContext, this: ClassInstanceRef<Self>, record_id: i32) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.rms.RecordStore::getRecordSize({this:?}, {record_id})");

        let database = Self::get_database(jvm, context, &this).await?;

        let result = database.get(record_id as _).await;
        if result.is_none() {
            return Err(jvm.exception("javax/microedition/rms/InvalidRecordIDException", "Record not found").await);
        }

        let data = result.unwrap();

        Ok(data.len() as _)
    }

    async fn set_record(
        jvm: &Jvm,
        _context: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        record_id: i32,
        data: ClassInstanceRef<Array<i8>>,
        offset: i32,
        length: i32,
    ) -> JvmResult<()> {
        tracing::debug!("javax.microedition.rms.RecordStore::setRecord({this:?}, {record_id}, {data:?}, {offset}, {length})");

        let _: i32 = jvm
            .invoke_static(
                "javax/microedition/rms/RecordStore",
                "wieWrite",
                "(Ljavax/microedition/rms/RecordStore;II[BII)I",
                (this, 2, record_id, data, offset, length),
            )
            .await?;
        Ok(())
    }

    async fn write_locked(
        jvm: &Jvm,
        context: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        operation: i32,
        record_id: i32,
        data: ClassInstanceRef<Array<i8>>,
        offset: i32,
        length: i32,
    ) -> JvmResult<i32> {
        let mut database = Self::get_database(jvm, context, &this).await?;
        if operation != 0 && (record_id <= 0 || database.get(record_id as _).await.is_none()) {
            return Err(jvm.exception("javax/microedition/rms/InvalidRecordIDException", "Record not found").await);
        }
        if operation == 0 {
            let bytes = Self::record_bytes(jvm, data, offset, length).await?;
            let id = database.add(&cast_vec(bytes)).await;
            if id != 0 {
                return Ok(id as i32);
            }
        } else if operation == 1 {
            if database.delete(record_id as _).await {
                return Ok(0);
            }
        } else {
            let bytes = Self::record_bytes(jvm, data, offset, length).await?;
            if database.set(record_id as _, &cast_vec(bytes)).await {
                return Ok(0);
            }
        }
        Err(jvm.exception("javax/microedition/rms/RecordStoreException", "Record write failed").await)
    }

    // MIDP permits a null buffer only for an empty record. Validate signed
    // bounds before converting to the JVM array API's unsigned indices.
    async fn record_bytes(jvm: &Jvm, data: ClassInstanceRef<Array<i8>>, offset: i32, length: i32) -> JvmResult<Vec<i8>> {
        if data.is_null() && length == 0 {
            return Ok(Vec::new());
        }
        if data.is_null() {
            return Err(jvm.exception("java/lang/NullPointerException", "Record data is null").await);
        }
        let size = jvm.array_length(&data).await? as usize;
        if offset < 0 || length < 0 || (offset as usize).checked_add(length as usize).is_none_or(|end| end > size) {
            return Err(jvm
                .exception("java/lang/ArrayIndexOutOfBoundsException", "Invalid record data range")
                .await);
        }
        jvm.load_array(&data, offset as _, length as _).await
    }

    async fn get_num_records(jvm: &Jvm, context: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.rms.RecordStore::getNumRecords({this:?})");

        let database = Self::get_database(jvm, context, &this).await?;

        let count = database.get_record_ids().await.len();

        Ok(count as _)
    }

    async fn close_record_store(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<()> {
        jvm.invoke_static(
            "javax/microedition/rms/RecordStore",
            "wieClose",
            "(Ljavax/microedition/rms/RecordStore;)V",
            (this,),
        )
        .await
    }

    async fn close_locked(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>) -> JvmResult<()> {
        let count = Self::ensure_open(jvm, &this).await?;
        jvm.put_field(&mut this, "openCount", "I", count - 1).await?;
        if count > 1 {
            return Ok(());
        }
        let mut previous: ClassInstanceRef<Self> = None.into();
        let mut cursor = Self::head(jvm).await?;
        while !cursor.is_null() {
            let next: ClassInstanceRef<Self> = jvm.get_field(&cursor, "nextOpen", "Ljavax/microedition/rms/RecordStore;").await?;
            if cursor.identity() == this.identity() {
                if previous.is_null() {
                    jvm.put_static_field(
                        "javax/microedition/rms/RecordStore",
                        "openStores",
                        "Ljavax/microedition/rms/RecordStore;",
                        next,
                    )
                    .await?;
                } else {
                    jvm.put_field(&mut previous, "nextOpen", "Ljavax/microedition/rms/RecordStore;", next)
                        .await?;
                }
                let empty: ClassInstanceRef<Self> = None.into();
                jvm.put_field(&mut this, "nextOpen", "Ljavax/microedition/rms/RecordStore;", empty)
                    .await?;
                break;
            }
            previous = cursor;
            cursor = next;
        }
        Ok(())
    }

    async fn head(jvm: &Jvm) -> JvmResult<ClassInstanceRef<Self>> {
        jvm.get_static_field("javax/microedition/rms/RecordStore", "openStores", "Ljavax/microedition/rms/RecordStore;")
            .await
    }

    async fn find_open(jvm: &Jvm, name: &str) -> JvmResult<ClassInstanceRef<Self>> {
        let mut cursor = Self::head(jvm).await?;
        while !cursor.is_null() {
            let stored = jvm.get_field(&cursor, "dbName", "Ljava/lang/String;").await?;
            if JavaLangString::to_rust_string(jvm, &stored).await? == name {
                return Ok(cursor);
            }
            cursor = jvm.get_field(&cursor, "nextOpen", "Ljavax/microedition/rms/RecordStore;").await?;
        }
        Ok(None.into())
    }

    async fn checked_name(jvm: &Jvm, name: &ClassInstanceRef<String>) -> JvmResult<alloc::string::String> {
        let name = JavaLangString::to_rust_string(jvm, name).await?;
        if !(1..=32).contains(&name.encode_utf16().count()) {
            return Err(jvm
                .exception("java/lang/IllegalArgumentException", "Record store name must contain 1 to 32 characters")
                .await);
        }
        Ok(name)
    }

    async fn ensure_open(jvm: &Jvm, this: &ClassInstanceRef<Self>) -> JvmResult<i32> {
        let count: i32 = jvm.get_field(this, "openCount", "I").await?;
        if count <= 0 {
            return Err(jvm
                .exception("javax/microedition/rms/RecordStoreNotOpenException", "Record store is closed")
                .await);
        }
        Ok(count)
    }

    async fn get_name(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<ClassInstanceRef<String>> {
        Self::ensure_open(jvm, &this).await?;
        jvm.get_field(&this, "dbName", "Ljava/lang/String;").await
    }

    async fn open_record_store(
        jvm: &Jvm,
        context: &mut WieJvmContext,
        name: ClassInstanceRef<String>,
        create: bool,
    ) -> JvmResult<ClassInstanceRef<Self>> {
        let guest_name = Self::checked_name(jvm, &name).await?;
        let mut existing = Self::find_open(jvm, &guest_name).await?;
        if !existing.is_null() {
            let count = Self::ensure_open(jvm, &existing).await?;
            let Some(count) = count.checked_add(1) else {
                return Err(jvm.exception("javax/microedition/rms/RecordStoreException", "Too many opens").await);
            };
            jvm.put_field(&mut existing, "openCount", "I", count).await?;
            return Ok(existing);
        }
        let key = super::store_name::storage_name(&guest_name);
        let system = context.system();
        let exists = system.platform().database_repository().exists(&key, system.pid()).await;
        if !exists && !create {
            return Err(jvm
                .exception("javax/microedition/rms/RecordStoreNotFoundException", "Record store not found")
                .await);
        }
        let mut backend = system.platform().database_repository().open(&key, system.pid()).await;
        let deleted = RecordData::is_deleted(&mut *backend).await;
        if deleted && !create {
            return Err(jvm
                .exception("javax/microedition/rms/RecordStoreNotFoundException", "Record store was deleted")
                .await);
        }
        if !exists || deleted {
            if !RecordData::install(backend, 1, alloc::collections::BTreeMap::new()).await {
                if !exists {
                    system.platform().database_repository().delete(&key, system.pid()).await;
                }
                return Err(jvm
                    .exception("javax/microedition/rms/RecordStoreException", "Could not create record store")
                    .await);
            }
        } else if let Err(error) = RecordData::load(backend).await {
            return Err(jvm.exception("javax/microedition/rms/RecordStoreException", error).await);
        }
        let mut store: ClassInstanceRef<Self> = jvm
            .new_class("javax/microedition/rms/RecordStore", "(Ljava/lang/String;)V", (name,))
            .await?
            .into();
        jvm.put_field(&mut store, "openCount", "I", 1).await?;
        let head = Self::head(jvm).await?;
        jvm.put_field(&mut store, "nextOpen", "Ljavax/microedition/rms/RecordStore;", head)
            .await?;
        jvm.put_static_field(
            "javax/microedition/rms/RecordStore",
            "openStores",
            "Ljavax/microedition/rms/RecordStore;",
            store.clone(),
        )
        .await?;
        Ok(store)
    }

    async fn delete_record_store(jvm: &Jvm, context: &mut WieJvmContext, name: ClassInstanceRef<String>) -> JvmResult<()> {
        let guest_name = Self::checked_name(jvm, &name).await?;
        if !Self::find_open(jvm, &guest_name).await?.is_null() {
            return Err(jvm
                .exception("javax/microedition/rms/RecordStoreException", "Cannot delete an open record store")
                .await);
        }
        let key = super::store_name::storage_name(&guest_name);
        let system = context.system();
        if !system.platform().database_repository().exists(&key, system.pid()).await {
            return Err(jvm
                .exception("javax/microedition/rms/RecordStoreNotFoundException", "Record store not found")
                .await);
        }
        let mut backend = system.platform().database_repository().open(&key, system.pid()).await;
        if RecordData::is_deleted(&mut *backend).await {
            return Err(jvm
                .exception("javax/microedition/rms/RecordStoreNotFoundException", "Record store was deleted")
                .await);
        }
        if !RecordData::mark_deleted(&mut *backend).await {
            return Err(jvm
                .exception("javax/microedition/rms/RecordStoreException", "Could not delete record store")
                .await);
        }
        Ok(())
    }

    async fn list_record_stores(jvm: &Jvm, context: &mut WieJvmContext) -> JvmResult<ClassInstanceRef<Array<String>>> {
        let system = context.system();
        let entries = system.platform().database_repository().list_directory(".", system.pid()).await;
        let Some(entries) = entries else {
            return Err(jvm
                .exception("javax/microedition/rms/RecordStoreException", "Could not list record stores")
                .await);
        };
        let mut names = Vec::new();
        for key in entries {
            let Some(name) = super::store_name::guest_name(&key) else {
                continue;
            };
            let mut backend = system.platform().database_repository().open(&key, system.pid()).await;
            if !RecordData::is_deleted(&mut *backend).await {
                names.push(name);
            }
        }
        if names.is_empty() {
            return Ok(None.into());
        }
        names.sort();
        let mut array = jvm.instantiate_array("Ljava/lang/String;", names.len()).await?;
        for (index, name) in names.iter().enumerate() {
            let name = JavaLangString::from_rust_string(jvm, name).await?;
            jvm.store_array(&mut array, index, [name]).await?;
        }
        Ok(array.into())
    }

    async fn get_database(jvm: &Jvm, context: &mut WieJvmContext, this: &ClassInstanceRef<Self>) -> JvmResult<Box<dyn Database>> {
        Self::ensure_open(jvm, this).await?;
        let db_name = jvm.get_field(this, "dbName", "Ljava/lang/String;").await?;
        let db_name_str = super::store_name::storage_name(&JavaLangString::to_rust_string(jvm, &db_name).await?);

        let system = context.system();
        let pid = system.pid().to_owned();

        let backend = system.platform().database_repository().open(&db_name_str, &pid).await;
        match RecordData::load(backend).await {
            Ok(records) => Ok(Box::new(records)),
            Err(error) => Err(jvm.exception("javax/microedition/rms/RecordStoreException", error).await),
        }
    }
}

#[cfg(test)]
mod test {
    use alloc::boxed::Box;

    use jvm::{Array, ClassInstanceRef, JavaError, Result as JvmResult, runtime::JavaLangString};
    use rustjava_runtime::classes::java::lang::String;
    use test_utils::run_jvm_test;
    use wie_util::Result;

    use crate::get_protos;

    use super::RecordStore;

    #[test]
    fn multiple_handles_share_monotonic_ids_after_deleting_all_records() -> Result<()> {
        run_jvm_test(Box::new([get_protos().into()]), |jvm| async move {
            const CLASS: &str = "javax/microedition/rms/RecordStore";
            let name = JavaLangString::from_rust_string(&jvm, "sequence").await?;
            let first: ClassInstanceRef<RecordStore> = jvm
                .invoke_static(
                    CLASS,
                    "openRecordStore",
                    "(Ljava/lang/String;Z)Ljavax/microedition/rms/RecordStore;",
                    (name.clone(), true),
                )
                .await?;
            let second: ClassInstanceRef<RecordStore> = jvm
                .invoke_static(
                    CLASS,
                    "openRecordStore",
                    "(Ljava/lang/String;Z)Ljavax/microedition/rms/RecordStore;",
                    (name.clone(), true),
                )
                .await?;
            let empty: ClassInstanceRef<Array<i8>> = None.into();
            for (handle, expected) in [(&first, 1), (&second, 2), (&first, 3)] {
                let id: i32 = jvm.invoke_virtual(handle, CLASS, "addRecord", "([BII)I", (empty.clone(), 0, 0)).await?;
                assert_eq!(id, expected);
            }
            for id in [3, 1, 2] {
                let _: () = jvm.invoke_virtual(&second, CLASS, "deleteRecord", "(I)V", (id,)).await?;
            }
            let _: () = jvm.invoke_virtual(&first, CLASS, "closeRecordStore", "()V", ()).await?;
            let _: () = jvm.invoke_virtual(&second, CLASS, "closeRecordStore", "()V", ()).await?;
            let reopened: ClassInstanceRef<RecordStore> = jvm
                .invoke_static(
                    CLASS,
                    "openRecordStore",
                    "(Ljava/lang/String;Z)Ljavax/microedition/rms/RecordStore;",
                    (name, false),
                )
                .await?;
            let next: i32 = jvm.invoke_virtual(&reopened, CLASS, "getNextRecordID", "()I", ()).await?;
            assert_eq!(next, 4);
            let count: i32 = jvm.invoke_virtual(&reopened, CLASS, "getNumRecords", "()I", ()).await?;
            assert_eq!(count, 0);
            let id: i32 = jvm.invoke_virtual(&reopened, CLASS, "addRecord", "([BII)I", (empty, 0, 0)).await?;
            assert_eq!(id, 4);
            Ok(())
        })
    }

    #[test]
    fn record_mutations_wait_for_the_shared_guest_monitor() -> Result<()> {
        use alloc::sync::Arc;
        use core::sync::atomic::{AtomicBool, Ordering};
        use test_utils::{TestPlatform, run_jvm_test_with_system};
        run_jvm_test_with_system(Box::new([get_protos().into()]), Box::new(TestPlatform::new()), |jvm, system| async move {
            const CLASS: &str = "javax/microedition/rms/RecordStore";
            let name = JavaLangString::from_rust_string(&jvm, "concurrent").await?;
            let store: ClassInstanceRef<RecordStore> = jvm
                .invoke_static(
                    CLASS,
                    "openRecordStore",
                    "(Ljava/lang/String;Z)Ljavax/microedition/rms/RecordStore;",
                    (name, true),
                )
                .await?;
            let monitor = jvm.resolve_class(CLASS).await?.java_class();
            jvm.monitor_enter(&monitor).await?;
            let started = Arc::new(AtomicBool::new(false));
            let completed = Arc::new(AtomicBool::new(false));
            let entered = started.clone();
            let done = completed.clone();
            let other_jvm = jvm.clone();
            let other_store = store.clone();
            let (tx, rx) = futures::channel::oneshot::channel();
            system.spawn(async move || {
                other_jvm.attach_thread(None).await.unwrap();
                entered.store(true, Ordering::SeqCst);
                let empty: ClassInstanceRef<Array<i8>> = None.into();
                let result: JvmResult<i32> = other_jvm.invoke_virtual(&other_store, CLASS, "addRecord", "([BII)I", (empty, 0, 0)).await;
                done.store(true, Ordering::SeqCst);
                other_jvm.detach_thread().unwrap();
                tx.send(result).unwrap();
                Ok(())
            });
            while !started.load(Ordering::SeqCst) {
                system.yield_now().await;
            }
            system.yield_now().await;
            assert!(!completed.load(Ordering::SeqCst));
            jvm.monitor_exit(&monitor).await?;
            assert_eq!(rx.await.unwrap()?, 1);
            let empty: ClassInstanceRef<Array<i8>> = None.into();
            let next: i32 = jvm.invoke_virtual(&store, CLASS, "addRecord", "([BII)I", (empty, 0, 0)).await?;
            assert_eq!(next, 2);
            Ok(())
        })
    }

    #[test]
    fn empty_records_and_failed_updates_preserve_store_contents() -> Result<()> {
        run_jvm_test(Box::new([get_protos().into()]), |jvm| async move {
            const CLASS: &str = "javax/microedition/rms/RecordStore";
            let name = JavaLangString::from_rust_string(&jvm, "record-contract").await?;
            let store: ClassInstanceRef<RecordStore> = jvm
                .invoke_static(
                    CLASS,
                    "openRecordStore",
                    "(Ljava/lang/String;Z)Ljavax/microedition/rms/RecordStore;",
                    (name, true),
                )
                .await?;
            let empty: ClassInstanceRef<Array<i8>> = None.into();
            let id: i32 = jvm.invoke_virtual(&store, CLASS, "addRecord", "([BII)I", (empty.clone(), 0, 0)).await?;
            let size: i32 = jvm.invoke_virtual(&store, CLASS, "getRecordSize", "(I)I", (id,)).await?;
            assert_eq!(size, 0);
            let mut bytes = jvm.instantiate_array("B", 3).await?;
            jvm.store_array(&mut bytes, 0, [4i8, 5, 6]).await?;
            let _: () = jvm
                .invoke_virtual(&store, CLASS, "setRecord", "(I[BII)V", (id, bytes.clone(), 1, 2))
                .await?;
            for bad_id in [0, -1, 99] {
                let result: JvmResult<()> = jvm
                    .invoke_virtual(&store, CLASS, "setRecord", "(I[BII)V", (bad_id, bytes.clone(), 0, 1))
                    .await;
                let Err(JavaError::JavaException(e)) = result else {
                    panic!("invalid update succeeded")
                };
                assert!(jvm.is_instance(&*e, "javax/microedition/rms/InvalidRecordIDException"));
            }
            for (offset, length) in [(-1, 1), (0, -1), (2, 2), (i32::MAX, i32::MAX)] {
                let result: JvmResult<()> = jvm
                    .invoke_virtual(&store, CLASS, "setRecord", "(I[BII)V", (id, bytes.clone(), offset, length))
                    .await;
                let Err(JavaError::JavaException(e)) = result else {
                    panic!("invalid bounds succeeded")
                };
                assert!(jvm.is_instance(&*e, "java/lang/ArrayIndexOutOfBoundsException"));
            }
            let saved: ClassInstanceRef<Array<i8>> = jvm.invoke_virtual(&store, CLASS, "getRecord", "(I)[B", (id,)).await?;
            assert_eq!(jvm.load_array::<i8>(&saved, 0, 2).await?, [5, 6]);
            let count: i32 = jvm.invoke_virtual(&store, CLASS, "getNumRecords", "()I", ()).await?;
            assert_eq!(count, 1);
            let _: () = jvm
                .invoke_virtual(&store, CLASS, "setRecord", "(I[BII)V", (id, empty.clone(), 0, 0))
                .await?;
            let size: i32 = jvm.invoke_virtual(&store, CLASS, "getRecordSize", "(I)I", (id,)).await?;
            assert_eq!(size, 0);
            let null_add: JvmResult<i32> = jvm.invoke_virtual(&store, CLASS, "addRecord", "([BII)I", (empty, 0, 1)).await;
            let Err(JavaError::JavaException(e)) = null_add else {
                panic!("nonempty null buffer accepted")
            };
            assert!(jvm.is_instance(&*e, "java/lang/NullPointerException"));
            Ok(())
        })
    }

    #[test]
    fn delete_record_removes_record_and_rejects_unknown_id() -> Result<()> {
        run_jvm_test(Box::new([get_protos().into()]), |jvm| async move {
            let name: ClassInstanceRef<String> = JavaLangString::from_rust_string(&jvm, "delete-record").await?.into();
            let store: ClassInstanceRef<RecordStore> = jvm
                .invoke_static(
                    "javax/microedition/rms/RecordStore",
                    "openRecordStore",
                    "(Ljava/lang/String;Z)Ljavax/microedition/rms/RecordStore;",
                    (name, true),
                )
                .await?;

            let mut data = jvm.instantiate_array("B", 2).await?;
            jvm.store_array(&mut data, 0, [1i8, 2]).await?;
            let record_id: i32 = jvm
                .invoke_virtual(&store, "javax/microedition/rms/RecordStore", "addRecord", "([BII)I", (data, 0, 2))
                .await?;
            assert_eq!(record_id, 1);

            let count: i32 = jvm
                .invoke_virtual(&store, "javax/microedition/rms/RecordStore", "getNumRecords", "()I", ())
                .await?;
            assert_eq!(count, 1);

            let _: () = jvm
                .invoke_virtual(&store, "javax/microedition/rms/RecordStore", "deleteRecord", "(I)V", (record_id,))
                .await?;
            let count: i32 = jvm
                .invoke_virtual(&store, "javax/microedition/rms/RecordStore", "getNumRecords", "()I", ())
                .await?;
            assert_eq!(count, 0);

            let deleted: JvmResult<ClassInstanceRef<Array<i8>>> = jvm
                .invoke_virtual(&store, "javax/microedition/rms/RecordStore", "getRecord", "(I)[B", (record_id,))
                .await;
            let Err(JavaError::JavaException(exception)) = deleted else {
                panic!("deleted record lookup succeeded");
            };
            assert!(jvm.is_instance(&*exception, "javax/microedition/rms/InvalidRecordIDException"));

            let unknown: JvmResult<()> = jvm
                .invoke_virtual(&store, "javax/microedition/rms/RecordStore", "deleteRecord", "(I)V", (99,))
                .await;
            let Err(JavaError::JavaException(exception)) = unknown else {
                panic!("unknown record deletion succeeded");
            };
            assert!(jvm.is_instance(&*exception, "javax/microedition/rms/InvalidRecordIDException"));

            Ok(())
        })
    }
}
