use super::Component;
use alloc::vec;
use jvm::{ClassInstanceRef, Jvm, Result as JvmResult};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use rustjava_runtime::classes::java::lang::String;
use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

pub struct DialogComponent;
impl DialogComponent {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "org/kwis/msp/lwc/DialogComponent",
            parent_class: Some("org/kwis/msp/lwc/ShellComponent"),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new(
                    "<init>",
                    "(Lorg/kwis/msp/lwc/Component;Ljava/lang/String;I)V",
                    Self::init,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("doModal", "()I", Self::do_modal, MethodAccessFlags::PUBLIC),
            ],
            fields: vec![
                JavaFieldProto::new("workComponent", "Lorg/kwis/msp/lwc/Component;", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("title", "Ljava/lang/String;", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("type", "I", FieldAccessFlags::PRIVATE),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }
    async fn init(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        component: ClassInstanceRef<Component>,
        title: ClassInstanceRef<String>,
        kind: i32,
    ) -> JvmResult<()> {
        if !(0..=2).contains(&kind) {
            return Err(jvm.exception("java/lang/IllegalArgumentException", "Invalid dialog type").await);
        }
        let _: () = jvm.invoke_special(&this, "org/kwis/msp/lwc/ShellComponent", "<init>", "()V", ()).await?;
        jvm.put_field(&mut this, "workComponent", "Lorg/kwis/msp/lwc/Component;", component)
            .await?;
        jvm.put_field(&mut this, "title", "Ljava/lang/String;", title).await?;
        jvm.put_field(&mut this, "type", "I", kind).await
    }
    async fn do_modal(jvm: &Jvm, _: &mut WieJvmContext, _: ClassInstanceRef<Self>) -> JvmResult<i32> {
        // Never manufacture an OK/cancel response for an unimplemented user interaction.
        Err(jvm
            .exception(
                "java/lang/UnsupportedOperationException",
                "LWC modal dialog presentation is not implemented",
            )
            .await)
    }
}

#[cfg(test)]
mod tests {
    use crate::{classes::org::kwis::msp::lwc::Component, get_protos};
    use alloc::boxed::Box;
    use jvm::{ClassInstanceRef, runtime::JavaLangString};
    use test_utils::run_jvm_test;
    use wie_util::Result;
    #[test]
    fn dialog_linkage_retains_arguments_and_rejects_fake_modal_results() -> Result<()> {
        run_jvm_test(Box::new([wie_midp::get_protos().into(), get_protos().into()]), |jvm| async move {
            let form = jvm.new_class("org/kwis/msp/lwc/FormComponent", "()V", ()).await?;
            let vertical: bool = jvm.get_field(&form, "vertical", "Z").await?;
            assert!(vertical);
            let title = JavaLangString::from_rust_string(&jvm, "이름 입력").await?;
            let name = "org/kwis/msp/lwc/DialogComponent";
            let signature = "(Lorg/kwis/msp/lwc/Component;Ljava/lang/String;I)V";
            let dialog = jvm.new_class(name, signature, (form.clone(), title.clone(), 2)).await?;
            let work: ClassInstanceRef<Component> = jvm.get_field(&dialog, "workComponent", "Lorg/kwis/msp/lwc/Component;").await?;
            assert_eq!(work.identity(), form.identity());
            let kind: i32 = jvm.get_field(&dialog, "type", "I").await?;
            assert_eq!(kind, 2);
            assert!(jvm.invoke_virtual::<_, i32>(&dialog, name, "doModal", "()I", ()).await.is_err());
            assert!(
                jvm.new_class(name, signature, (ClassInstanceRef::<Component>::new(None), title, 3))
                    .await
                    .is_err()
            );
            Ok(())
        })
    }
}
