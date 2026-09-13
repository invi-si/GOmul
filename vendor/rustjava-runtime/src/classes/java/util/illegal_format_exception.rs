use alloc::vec;

use jvm::{ClassInstanceRef, Jvm, Result};
use jvm_class_proto::JavaMethodProto;
use jvm_types::{ClassAccessFlags, MethodAccessFlags};

use crate::{RuntimeClassProto, RuntimeContext};

// public class java.util.IllegalFormatException
pub struct IllegalFormatException;

impl IllegalFormatException {
    pub fn as_proto() -> RuntimeClassProto {
        RuntimeClassProto {
            name: "java/util/IllegalFormatException",
            parent_class: Some("java/lang/IllegalArgumentException"),
            interfaces: vec![],
            methods: vec![JavaMethodProto::new("<init>", "()V", Self::init, MethodAccessFlags::empty())],
            fields: vec![],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }

    async fn init(jvm: &Jvm, _: &mut RuntimeContext, this: ClassInstanceRef<Self>) -> Result<()> {
        jvm.invoke_special(&this, "java/lang/IllegalArgumentException", "<init>", "()V", ()).await
    }
}
