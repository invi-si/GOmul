use alloc::vec;

use jvm_class_proto::JavaMethodProto;
use jvm_types::{ClassAccessFlags, MethodAccessFlags};

use crate::RuntimeClassProto;

// interface java.lang.Runnable
pub struct Runnable;

impl Runnable {
    pub fn as_proto() -> RuntimeClassProto {
        RuntimeClassProto {
            name: "java/lang/Runnable",
            parent_class: None,
            interfaces: vec![],
            methods: vec![JavaMethodProto::new_abstract(
                "run",
                "()V",
                MethodAccessFlags::PUBLIC | MethodAccessFlags::ABSTRACT,
            )],
            fields: vec![],
            access_flags: ClassAccessFlags::INTERFACE,
        }
    }
}
