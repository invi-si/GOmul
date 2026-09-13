use alloc::vec;
use jvm::{ClassInstanceRef, Jvm, Result};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

pub struct GForm;
const NAME: &str = "com/ktf/kfc/GForm";
const SHELL: &str = "org/kwis/msp/lwc/ShellComponent";
impl GForm {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: NAME,
            parent_class: Some(SHELL),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("<init>", "(I)V", Self::init_id, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("<init>", "(IIII)V", Self::init_bounds, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setFormID", "(I)V", Self::set_id, MethodAccessFlags::PUBLIC | MethodAccessFlags::FINAL),
                JavaMethodProto::new("getFormID", "()I", Self::get_id, MethodAccessFlags::PUBLIC | MethodAccessFlags::FINAL),
                JavaMethodProto::new(
                    "getAnnunciatorHeight",
                    "()I",
                    Self::annunciator_height,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::FINAL,
                ),
                JavaMethodProto::new("makeContents", "()V", Self::extension_hook, MethodAccessFlags::PUBLIC),
            ],
            fields: vec![JavaFieldProto::new("wieFormID", "I", FieldAccessFlags::PRIVATE)],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }
    async fn init_id(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>, id: i32) -> Result<()> {
        let _: () = jvm.invoke_special(&this, SHELL, "<init>", "()V", ()).await?;
        Self::set_id(jvm, ctx, this, id).await
    }
    async fn init_bounds(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, x: i32, y: i32, width: i32, height: i32) -> Result<()> {
        jvm.invoke_special(&this, SHELL, "<init>", "(IIII)V", (x, y, width, height)).await
    }
    async fn set_id(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, id: i32) -> Result<()> {
        jvm.put_field_in_class(&mut this, NAME, "wieFormID", "I", id).await
    }
    async fn get_id(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<i32> {
        jvm.get_field_in_class(&this, NAME, "wieFormID", "I").await
    }
    async fn annunciator_height(jvm: &Jvm, _: &mut WieJvmContext, _: ClassInstanceRef<Self>) -> Result<i32> {
        // Read the same device-profile strip geometry exposed to other WIPI
        // callers. A form's own dimensions must not change the device strip.
        let strip = jvm.new_class("org/kwis/msp/lwc/AnnunciatorComponent", "(Z)V", (false,)).await?;
        jvm.invoke_virtual(&strip, "org/kwis/msp/lwc/Component", "getHeight", "()I", ()).await
    }
    async fn extension_hook(_: &Jvm, _: &mut WieJvmContext, _: ClassInstanceRef<Self>) -> Result<()> {
        // Documented extension point; a guest subclass supplies its contents.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;
    use test_utils::run_jvm_test;
    #[test]
    fn form_ids_and_geometry_are_independent_and_contents_dispatches_to_guest() -> wie_util::Result<()> {
        let jlet = WieJavaClassProto {
            name: "test/FormJlet",
            parent_class: Some("org/kwis/msp/lcdui/Jlet"),
            interfaces: vec![],
            methods: vec![],
            fields: vec![],
            access_flags: ClassAccessFlags::PUBLIC,
        };
        let subclass = WieJavaClassProto {
            name: "test/CustomForm",
            parent_class: Some(NAME),
            interfaces: vec![],
            methods: vec![JavaMethodProto::new("makeContents", "()V", contents, MethodAccessFlags::PUBLIC)],
            fields: vec![JavaFieldProto::new("calls", "I", FieldAccessFlags::PUBLIC)],
            access_flags: ClassAccessFlags::PUBLIC,
        };
        run_jvm_test(
            Box::new([
                wie_midp::get_protos().into(),
                wie_wipi_java::get_protos().into(),
                vec![GForm::as_proto(), jlet, subclass].into_boxed_slice(),
            ]),
            |jvm| async move {
                let _ = jvm.new_class("net/wie/WIPIMIDlet", "()V", ()).await?;
                let jlet = jvm.instantiate_class("test/FormJlet").await?;
                let _: () = jvm.invoke_special(&jlet, "org/kwis/msp/lcdui/Jlet", "<init>", "()V", ()).await?;
                let a = jvm.new_class(NAME, "(I)V", (42,)).await?;
                let b = jvm.new_class(NAME, "(IIII)V", (3, 7, 80, 90)).await?;
                assert_eq!(jvm.invoke_virtual::<_, i32>(&a, NAME, "getFormID", "()I", ()).await?, 42);
                let _: () = jvm.invoke_virtual(&a, NAME, "setFormID", "(I)V", (-17,)).await?;
                assert_eq!(jvm.invoke_virtual::<_, i32>(&a, NAME, "getFormID", "()I", ()).await?, -17);
                assert_eq!(jvm.invoke_virtual::<_, i32>(&b, NAME, "getFormID", "()I", ()).await?, 0);
                for (method, expected) in [("getX", 3), ("getY", 7), ("getWidth", 80), ("getHeight", 90)] {
                    assert_eq!(jvm.invoke_virtual::<_, i32>(&b, NAME, method, "()I", ()).await?, expected);
                }
                let strip = jvm.new_class("org/kwis/msp/lwc/AnnunciatorComponent", "(Z)V", (false,)).await?;
                let expected: i32 = jvm.invoke_virtual(&strip, "org/kwis/msp/lwc/Component", "getHeight", "()I", ()).await?;
                assert!(expected > 0);
                assert_eq!(jvm.invoke_virtual::<_, i32>(&b, NAME, "getAnnunciatorHeight", "()I", ()).await?, expected);
                let custom = jvm.instantiate_class("test/CustomForm").await?;
                let _: () = jvm.invoke_special(&custom, NAME, "<init>", "(I)V", (9,)).await?;
                assert_eq!(jvm.get_field::<i32>(&custom, "calls", "I").await?, 0);
                let _: () = jvm.invoke_virtual(&custom, NAME, "makeContents", "()V", ()).await?;
                assert_eq!(jvm.get_field::<i32>(&custom, "calls", "I").await?, 1);
                Ok(())
            },
        )
    }
    async fn contents(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<()>) -> Result<()> {
        let calls: i32 = jvm.get_field(&this, "calls", "I").await?;
        jvm.put_field(&mut this, "calls", "I", calls + 1).await
    }
}
