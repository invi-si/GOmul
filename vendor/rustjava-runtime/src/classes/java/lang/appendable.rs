use alloc::vec;

use jvm_class_proto::JavaMethodProto;
use jvm_types::{ClassAccessFlags, MethodAccessFlags};

use crate::RuntimeClassProto;

// public interface java.lang.Appendable
pub struct Appendable;

impl Appendable {
    pub fn as_proto() -> RuntimeClassProto {
        RuntimeClassProto {
            name: "java/lang/Appendable",
            parent_class: None,
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new_abstract(
                    "append",
                    "(Ljava/lang/CharSequence;)Ljava/lang/Appendable;",
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::ABSTRACT,
                ),
                JavaMethodProto::new_abstract(
                    "append",
                    "(Ljava/lang/CharSequence;II)Ljava/lang/Appendable;",
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::ABSTRACT,
                ),
                JavaMethodProto::new_abstract(
                    "append",
                    "(C)Ljava/lang/Appendable;",
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::ABSTRACT,
                ),
            ],
            fields: vec![],
            access_flags: ClassAccessFlags::PUBLIC | ClassAccessFlags::INTERFACE | ClassAccessFlags::ABSTRACT,
        }
    }
}
