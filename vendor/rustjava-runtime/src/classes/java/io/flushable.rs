use alloc::vec;

use jvm_class_proto::JavaMethodProto;
use jvm_types::{ClassAccessFlags, MethodAccessFlags};

use crate::RuntimeClassProto;

// public interface java.io.Flushable
pub struct Flushable;

impl Flushable {
    pub fn as_proto() -> RuntimeClassProto {
        RuntimeClassProto {
            name: "java/io/Flushable",
            parent_class: None,
            interfaces: vec![],
            methods: vec![JavaMethodProto::new_abstract(
                "flush",
                "()V",
                MethodAccessFlags::PUBLIC | MethodAccessFlags::ABSTRACT,
            )],
            fields: vec![],
            access_flags: ClassAccessFlags::PUBLIC | ClassAccessFlags::INTERFACE | ClassAccessFlags::ABSTRACT,
        }
    }
}
