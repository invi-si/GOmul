use alloc::vec;

use jvm::{Array, ClassInstanceRef, Jvm, Result as JvmResult};
use jvm_class_proto::JavaMethodProto;
use jvm_types::{ClassAccessFlags, MethodAccessFlags};
use rustjava_runtime::classes::java::lang::String;

use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

// class org.kwis.msp.lwc.TextFieldComponent
pub struct TextFieldComponent;

impl TextFieldComponent {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "org/kwis/msp/lwc/TextFieldComponent",
            parent_class: Some("org/kwis/msp/lwc/TextComponent"),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("<init>", "(Ljava/lang/String;I)V", Self::init, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("insert", "([CIII)V", Self::insert, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setString", "(Ljava/lang/String;)V", Self::set_string, MethodAccessFlags::PUBLIC),
            ],
            fields: vec![],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }

    async fn init(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        mut this: ClassInstanceRef<TextFieldComponent>,
        data: ClassInstanceRef<String>,
        constraint: i32,
    ) -> JvmResult<()> {
        if !(0..=5).contains(&constraint) {
            return Err(jvm.exception("java/lang/IllegalArgumentException", "Invalid text constraint").await);
        }

        let _: () = jvm.invoke_special(&this, "org/kwis/msp/lwc/TextComponent", "<init>", "()V", ()).await?;

        jvm.put_field(&mut this, "constraint", "I", constraint).await?;
        jvm.invoke_special(&this, "org/kwis/msp/lwc/TextComponent", "setString", "(Ljava/lang/String;)V", (data,))
            .await
    }

    async fn insert(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        this: ClassInstanceRef<TextFieldComponent>,
        data: ClassInstanceRef<Array<u16>>,
        offset: i32,
        length: i32,
        index: i32,
    ) -> JvmResult<()> {
        jvm.invoke_special(
            &this,
            "org/kwis/msp/lwc/TextComponent",
            "insert",
            "([CIII)V",
            (data, offset, length, index),
        )
        .await
    }

    async fn set_string(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        this: ClassInstanceRef<TextFieldComponent>,
        data: ClassInstanceRef<String>,
    ) -> JvmResult<()> {
        jvm.invoke_special(&this, "org/kwis/msp/lwc/TextComponent", "setString", "(Ljava/lang/String;)V", (data,))
            .await
    }
}
