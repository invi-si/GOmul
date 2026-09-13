use alloc::vec;

use jvm::{ClassInstanceRef, Jvm, Result};
use jvm_class_proto::JavaMethodProto;
use jvm_types::{ClassAccessFlags, MethodAccessFlags};

use crate::{RuntimeClassProto, RuntimeContext};

// public class java.util.FormatterClosedException
pub struct FormatterClosedException;

impl FormatterClosedException {
    pub fn as_proto() -> RuntimeClassProto {
        RuntimeClassProto {
            name: "java/util/FormatterClosedException",
            parent_class: Some("java/lang/IllegalStateException"),
            interfaces: vec![],
            methods: vec![JavaMethodProto::new("<init>", "()V", Self::init, MethodAccessFlags::PUBLIC)],
            fields: vec![],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }

    async fn init(jvm: &Jvm, _: &mut RuntimeContext, this: ClassInstanceRef<Self>) -> Result<()> {
        jvm.invoke_special(&this, "java/lang/IllegalStateException", "<init>", "()V", ()).await
    }
}
