use alloc::vec;
use jvm::{Jvm, Result};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

pub struct CommandListener;
impl CommandListener {
    // ConstantValue attributes from the public AromaSoft WIPI 1.1.1 SDK.
    pub const FOCUS_CHANGE: i32 = 1;
    pub const SELECT: i32 = 2;
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "org/kwis/msp/lwc/CommandListener",
            parent_class: None,
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("<clinit>", "()V", Self::clinit, MethodAccessFlags::STATIC),
                JavaMethodProto::new_abstract(
                    "commandAction",
                    "(Lorg/kwis/msp/lwc/Command;ILjava/lang/Object;)V",
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::ABSTRACT,
                ),
            ],
            fields: ["FOCUS_CHANGE", "SELECT"]
                .into_iter()
                .map(|name| JavaFieldProto::new(name, "I", FieldAccessFlags::PUBLIC | FieldAccessFlags::STATIC | FieldAccessFlags::FINAL))
                .collect(),
            access_flags: ClassAccessFlags::PUBLIC | ClassAccessFlags::INTERFACE | ClassAccessFlags::ABSTRACT,
        }
    }
    async fn clinit(jvm: &Jvm, _: &mut WieJvmContext) -> Result<()> {
        for (field, value) in [("FOCUS_CHANGE", Self::FOCUS_CHANGE), ("SELECT", Self::SELECT)] {
            jvm.put_static_field("org/kwis/msp/lwc/CommandListener", field, "I", value).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use alloc::boxed::Box;
    use test_utils::run_jvm_test;
    #[test]
    fn sdk_listener_constants_are_visible_to_guest_static_field_reads() -> wie_util::Result<()> {
        run_jvm_test(Box::new([wie_midp::get_protos().into(), crate::get_protos().into()]), |jvm| async move {
            for (class, field, value) in [
                ("org/kwis/msp/lwc/CommandListener", "FOCUS_CHANGE", 1),
                ("org/kwis/msp/lwc/CommandListener", "SELECT", 2),
                ("org/kwis/msp/lcdui/ImageObserver", "FRAME_END", 0),
                ("org/kwis/msp/lcdui/ImageObserver", "IMAGE_END", 1),
                ("org/kwis/msp/lcdui/ImageObserver", "NOT_EXIST", -1),
                ("org/kwis/msp/lcdui/ImageObserver", "DECODE_ERROR", -2),
                ("org/kwis/msp/lcdui/ImageObserver", "OUT_OF_MEMORY", -3),
            ] {
                assert_eq!(jvm.get_static_field::<i32>(class, field, "I").await?, value);
            }
            Ok(())
        })
    }
}
