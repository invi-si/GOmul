use alloc::vec;
use jvm::{ClassInstanceRef, Jvm, Result};
use jvm_class_proto::JavaMethodProto;
use jvm_types::{ClassAccessFlags, MethodAccessFlags};
use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

/// KTF OEM extension discovery. Unsupported services return null per the SDK.
pub struct OEMDevice;
impl OEMDevice {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "wec/OEMDevice",
            parent_class: Some("java/lang/Object"),
            interfaces: vec![],
            fields: vec![],
            methods: vec![
                JavaMethodProto::new(
                    "getAddressBook",
                    "()Lwec/AddressBook;",
                    Self::unavailable,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC,
                ),
                JavaMethodProto::new(
                    "getSYSTheme",
                    "()Lwec/SYSTheme;",
                    Self::unavailable,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC,
                ),
            ],
            access_flags: ClassAccessFlags::PUBLIC | ClassAccessFlags::FINAL,
        }
    }
    async fn unavailable(_: &Jvm, _: &mut WieJvmContext) -> Result<ClassInstanceRef<()>> {
        Ok(None.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;
    use test_utils::run_jvm_test;

    #[test]
    fn unsupported_oem_services_return_null() -> wie_util::Result<()> {
        run_jvm_test(Box::new([vec![OEMDevice::as_proto()].into()]), |jvm| async move {
            for (name, descriptor) in [("getAddressBook", "()Lwec/AddressBook;"), ("getSYSTheme", "()Lwec/SYSTheme;")] {
                let result: ClassInstanceRef<()> = jvm.invoke_static("wec/OEMDevice", name, descriptor, ()).await?;
                assert!(result.is_null());
            }
            Ok(())
        })
    }
}
