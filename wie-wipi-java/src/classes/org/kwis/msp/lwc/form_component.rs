use alloc::vec;
use jvm::{ClassInstanceRef, Jvm, Result as JvmResult};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

pub struct FormComponent;
impl FormComponent {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "org/kwis/msp/lwc/FormComponent",
            parent_class: Some("org/kwis/msp/lwc/ContainerComponent"),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("<init>", "()V", Self::init, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("<init>", "(Z)V", Self::init_orientation, MethodAccessFlags::PUBLIC),
            ],
            fields: vec![JavaFieldProto::new("vertical", "Z", FieldAccessFlags::PRIVATE)],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }
    async fn init(jvm: &Jvm, context: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<()> {
        Self::init_orientation(jvm, context, this, true).await
    }
    async fn init_orientation(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, vertical: bool) -> JvmResult<()> {
        let _: () = jvm
            .invoke_special(&this, "org/kwis/msp/lwc/ContainerComponent", "<init>", "()V", ())
            .await?;
        jvm.put_field(&mut this, "vertical", "Z", vertical).await
    }
}
