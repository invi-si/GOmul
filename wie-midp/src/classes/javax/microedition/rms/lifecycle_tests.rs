use super::{RecordStore, install_packaged_records};
use alloc::{boxed::Box, collections::BTreeMap, vec, vec::Vec};
use jvm::{Array, ClassInstanceRef, JavaError, Jvm, Result as JvmResult, runtime::JavaLangString};
use rustjava_runtime::classes::java::lang::String;
use test_utils::{TestPlatform, run_jvm_test_with_system};

const CLASS: &str = "javax/microedition/rms/RecordStore";

async fn open(jvm: &Jvm, name: &str, create: bool) -> JvmResult<ClassInstanceRef<RecordStore>> {
    let name = JavaLangString::from_rust_string(jvm, name).await?;
    jvm.invoke_static(
        CLASS,
        "openRecordStore",
        "(Ljava/lang/String;Z)Ljavax/microedition/rms/RecordStore;",
        (name, create),
    )
    .await
}
async fn delete(jvm: &Jvm, name: &str) -> JvmResult<()> {
    let name = JavaLangString::from_rust_string(jvm, name).await?;
    jvm.invoke_static(CLASS, "deleteRecordStore", "(Ljava/lang/String;)V", (name,)).await
}
fn exception<T>(jvm: &Jvm, result: JvmResult<T>, class: &str) {
    let Err(JavaError::JavaException(error)) = result else {
        panic!("expected guest exception");
    };
    assert!(jvm.is_instance(&*error, class));
}
async fn list(jvm: &Jvm) -> JvmResult<Vec<alloc::string::String>> {
    let names: ClassInstanceRef<Array<String>> = jvm.invoke_static(CLASS, "listRecordStores", "()[Ljava/lang/String;", ()).await?;
    if names.is_null() {
        return Ok(Vec::new());
    }
    let size = jvm.array_length(&names).await?;
    let mut result = Vec::new();
    for name in jvm.load_array::<ClassInstanceRef<String>>(&names, 0, size).await? {
        result.push(JavaLangString::to_rust_string(jvm, &name).await?);
    }
    Ok(result)
}

#[test]
fn open_counts_closed_handles_and_deletion_follow_guest_lifecycle() -> wie_util::Result<()> {
    run_jvm_test_with_system(
        Box::new([crate::get_protos().into()]),
        Box::new(TestPlatform::new()),
        |jvm, system| async move {
            exception(
                &jvm,
                open(&jvm, "missing", false).await,
                "javax/microedition/rms/RecordStoreNotFoundException",
            );
            assert!(!system.platform().database_repository().exists("missing", system.pid()).await);
            let names: ClassInstanceRef<Array<String>> = jvm.invoke_static(CLASS, "listRecordStores", "()[Ljava/lang/String;", ()).await?;
            assert!(names.is_null());
            let a = open(&jvm, "Save", true).await?;
            let b = open(&jvm, "Save", false).await?;
            assert_eq!(a.identity(), b.identity());
            exception(&jvm, delete(&jvm, "Save").await, "javax/microedition/rms/RecordStoreException");
            assert_eq!(list(&jvm).await?, ["Save"]);
            let _: () = jvm.invoke_virtual(&a, CLASS, "closeRecordStore", "()V", ()).await?;
            let count: i32 = jvm.invoke_virtual(&b, CLASS, "getNumRecords", "()I", ()).await?;
            assert_eq!(count, 0);
            let _: () = jvm.invoke_virtual(&b, CLASS, "closeRecordStore", "()V", ()).await?;
            exception::<i32>(
                &jvm,
                jvm.invoke_virtual(&a, CLASS, "getNumRecords", "()I", ()).await,
                "javax/microedition/rms/RecordStoreNotOpenException",
            );
            exception::<()>(
                &jvm,
                jvm.invoke_virtual(&a, CLASS, "closeRecordStore", "()V", ()).await,
                "javax/microedition/rms/RecordStoreNotOpenException",
            );
            delete(&jvm, "Save").await?;
            assert!(list(&jvm).await?.is_empty());
            exception(&jvm, delete(&jvm, "Save").await, "javax/microedition/rms/RecordStoreNotFoundException");
            install_packaged_records(&system, "Save", 2, BTreeMap::from([(1, vec![99])]))
                .await
                .unwrap();
            exception(
                &jvm,
                open(&jvm, "Save", false).await,
                "javax/microedition/rms/RecordStoreNotFoundException",
            );
            let fresh = open(&jvm, "Save", true).await?;
            assert_ne!(fresh.identity(), a.identity());
            let next: i32 = jvm.invoke_virtual(&fresh, CLASS, "getNextRecordID", "()I", ()).await?;
            assert_eq!(next, 1);
            let count: i32 = jvm.invoke_virtual(&fresh, CLASS, "getNumRecords", "()I", ()).await?;
            assert_eq!(count, 0);
            exception::<i32>(
                &jvm,
                jvm.invoke_virtual(&a, CLASS, "getNextRecordID", "()I", ()).await,
                "javax/microedition/rms/RecordStoreNotOpenException",
            );
            Ok(())
        },
    )
}

#[test]
fn named_stores_are_distinct_and_closed_nodes_leave_the_guest_root() -> wie_util::Result<()> {
    run_jvm_test_with_system(
        Box::new([crate::get_protos().into()]),
        Box::new(TestPlatform::new()),
        |jvm, _| async move {
            for name in ["", "12345678901234567890123456789012345"] {
                exception(&jvm, open(&jvm, name, true).await, "java/lang/IllegalArgumentException");
            }
            let mut stores = Vec::new();
            let names = ["a/b", "__gomul_rms_612f62", "한글"];
            for (index, name) in names.iter().enumerate() {
                let store = open(&jvm, name, true).await?;
                let value: ClassInstanceRef<String> = jvm.invoke_virtual(&store, CLASS, "getName", "()Ljava/lang/String;", ()).await?;
                assert_eq!(JavaLangString::to_rust_string(&jvm, &value).await?, *name);
                let mut data = jvm.instantiate_array("B", 1).await?;
                jvm.store_array(&mut data, 0, [index as i8]).await?;
                let id: i32 = jvm.invoke_virtual(&store, CLASS, "addRecord", "([BII)I", (data, 0, 1)).await?;
                assert_eq!(id, 1);
                stores.push(store);
            }
            let mut expected = names.to_vec();
            expected.sort();
            assert_eq!(list(&jvm).await?, expected);
            for index in [1, 2, 0] {
                let _: () = jvm.invoke_virtual(&stores[index], CLASS, "closeRecordStore", "()V", ()).await?;
            }
            let root: ClassInstanceRef<RecordStore> = jvm.get_static_field(CLASS, "openStores", "Ljavax/microedition/rms/RecordStore;").await?;
            assert!(root.is_null());
            for (index, name) in names.iter().enumerate() {
                let reopened = open(&jvm, name, false).await?;
                let value: ClassInstanceRef<Array<i8>> = jvm.invoke_virtual(&reopened, CLASS, "getRecord", "(I)[B", (1,)).await?;
                assert_eq!(jvm.load_array::<i8>(&value, 0, 1).await?, [index as i8]);
            }
            Ok(())
        },
    )
}
