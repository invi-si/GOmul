use alloc::vec;

use jvm::{Jvm, Result};
use jvm_class_proto::JavaMethodProto;
use jvm_types::{ClassAccessFlags, MethodAccessFlags};
use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

/// WIPI LED capability on a host exposing no emulated indicator LEDs.
pub struct LED;

impl LED {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "org/kwis/msp/handset/LED",
            parent_class: Some("java/lang/Object"),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("getCount", "()I", Self::get, MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC),
                JavaMethodProto::new("get", "()I", Self::get, MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC),
                JavaMethodProto::new("set", "(I)V", Self::set, MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC),
            ],
            fields: vec![],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }

    async fn get(_: &Jvm, _: &mut WieJvmContext) -> Result<i32> {
        Ok(0)
    }

    async fn set(_: &Jvm, _: &mut WieJvmContext, _mask: i32) -> Result<()> {
        // With zero LEDs there are no supported bits to turn on.
        Ok(())
    }
}
