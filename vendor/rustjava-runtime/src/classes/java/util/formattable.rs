use alloc::vec;

use jvm_class_proto::JavaMethodProto;
use jvm_types::{ClassAccessFlags, MethodAccessFlags};

use crate::RuntimeClassProto;

// public interface java.util.Formattable
pub struct Formattable;

impl Formattable {
    pub fn as_proto() -> RuntimeClassProto {
        RuntimeClassProto {
            name: "java/util/Formattable",
            parent_class: None,
            interfaces: vec![],
            methods: vec![JavaMethodProto::new_abstract(
                "formatTo",
                "(Ljava/util/Formatter;III)V",
                MethodAccessFlags::PUBLIC | MethodAccessFlags::ABSTRACT,
            )],
            fields: vec![],
            access_flags: ClassAccessFlags::PUBLIC | ClassAccessFlags::INTERFACE | ClassAccessFlags::ABSTRACT,
        }
    }
}
