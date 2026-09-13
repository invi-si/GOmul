use alloc::vec;

use jvm::{Jvm, Result};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};

use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

// interface org.kwis.msp.lcdui.ImageObserver
pub struct ImageObserver;

impl ImageObserver {
    pub const FRAME_END: i32 = 0;
    pub const IMAGE_END: i32 = 1;
    pub const NOT_EXIST: i32 = -1;
    pub const DECODE_ERROR: i32 = -2;
    pub const OUT_OF_MEMORY: i32 = -3;
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "org/kwis/msp/lcdui/ImageObserver",
            parent_class: None,
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("<clinit>", "()V", Self::clinit, MethodAccessFlags::STATIC),
                JavaMethodProto::new_abstract(
                    "notify",
                    "(Lorg/kwis/msp/lcdui/Image;I)V",
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::ABSTRACT,
                ),
            ],
            fields: ["FRAME_END", "IMAGE_END", "NOT_EXIST", "DECODE_ERROR", "OUT_OF_MEMORY"]
                .into_iter()
                .map(|name| JavaFieldProto::new(name, "I", FieldAccessFlags::PUBLIC | FieldAccessFlags::STATIC | FieldAccessFlags::FINAL))
                .collect(),
            access_flags: ClassAccessFlags::PUBLIC | ClassAccessFlags::INTERFACE | ClassAccessFlags::ABSTRACT,
        }
    }
    async fn clinit(jvm: &Jvm, _: &mut WieJvmContext) -> Result<()> {
        for (name, value) in [
            ("FRAME_END", Self::FRAME_END),
            ("IMAGE_END", Self::IMAGE_END),
            ("NOT_EXIST", Self::NOT_EXIST),
            ("DECODE_ERROR", Self::DECODE_ERROR),
            ("OUT_OF_MEMORY", Self::OUT_OF_MEMORY),
        ] {
            jvm.put_static_field("org/kwis/msp/lcdui/ImageObserver", name, "I", value).await?;
        }
        Ok(())
    }
}
