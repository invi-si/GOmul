use alloc::vec;

use jvm::{ClassInstanceRef, Jvm, Result};
use jvm_class_proto::JavaMethodProto;
use jvm_types::{ClassAccessFlags, MethodAccessFlags};
use rustjava_runtime::classes::java::lang::String;

use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

// class org.kwis.msp.media.MediaUnsupportedException
pub struct MediaUnsupportedException;

impl MediaUnsupportedException {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "org/kwis/msp/media/MediaUnsupportedException",
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
        tracing::debug!("org.kwis.msp.media.MediaUnsupportedException::<init>({this:?})");

        let _: () = jvm.invoke_special(&this, "java/lang/RuntimeException", "<init>", "()V", ()).await?;

        Ok(())
    }

    async fn init_with_message(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, message: ClassInstanceRef<String>) -> Result<()> {
        tracing::debug!("org.kwis.msp.media.MediaUnsupportedException::<init>({this:?}, {message:?})");

        let _: () = jvm
            .invoke_special(&this, "java/lang/RuntimeException", "<init>", "(Ljava/lang/String;)V", (message,))
            .await?;

        Ok(())
    }
}
