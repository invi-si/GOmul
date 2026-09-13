use alloc::vec;

use jvm::{ClassInstanceRef, Jvm, Result};
use jvm_class_proto::JavaMethodProto;
use jvm_types::{ClassAccessFlags, MethodAccessFlags};
use rustjava_runtime::classes::java::lang::String;

use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

// class java.io.UnavailableException
pub struct UnavailableException;

impl UnavailableException {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "java/io/UnavailableException",
            parent_class: Some("java/lang/RuntimeException"),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("<init>", "()V", Self::init, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("<init>", "(Ljava/lang/String;)V", Self::init_with_message, MethodAccessFlags::PUBLIC),
            ],
            fields: vec![],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }

    async fn init(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<()> {
        tracing::debug!("java.io.UnavailableException::<init>({this:?})");

        let _: () = jvm.invoke_special(&this, "java/lang/RuntimeException", "<init>", "()V", ()).await?;

        Ok(())
    }

    async fn init_with_message(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, message: ClassInstanceRef<String>) -> Result<()> {
        tracing::debug!("java.io.UnavailableException::<init>({this:?}, {message:?})");

        let _: () = jvm
            .invoke_special(&this, "java/lang/RuntimeException", "<init>", "(Ljava/lang/String;)V", (message,))
            .await?;

        Ok(())
    }
}
