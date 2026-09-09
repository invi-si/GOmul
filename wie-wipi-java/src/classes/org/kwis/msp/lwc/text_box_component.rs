use alloc::vec;

use jvm::{ClassInstanceRef, Jvm, Result as JvmResult};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use rustjava_runtime::classes::java::lang::String;

use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

// class org.kwis.msp.lwc.TextBoxComponent
pub struct TextBoxComponent;

impl TextBoxComponent {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "org/kwis/msp/lwc/TextBoxComponent",
            parent_class: Some("org/kwis/msp/lwc/TextComponent"),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("<init>", "(Ljava/lang/String;I)V", Self::init, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("<init>", "(Ljava/lang/String;II)V", Self::init_with_height, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getHeight", "()I", Self::get_height, MethodAccessFlags::PUBLIC),
            ],
            fields: vec![JavaFieldProto::new("boxHeight", "I", FieldAccessFlags::PRIVATE)],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }

    async fn init(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        mut this: ClassInstanceRef<TextBoxComponent>,
        data: ClassInstanceRef<String>,
        constraint: i32,
    ) -> JvmResult<()> {
        if !(0..=5).contains(&constraint) {
            return Err(jvm.exception("java/lang/IllegalArgumentException", "Invalid text constraint").await);
        }

        let _: () = jvm.invoke_special(&this, "org/kwis/msp/lwc/TextComponent", "<init>", "()V", ()).await?;

        jvm.put_field(&mut this, "constraint", "I", constraint).await?;
        jvm.invoke_virtual(&this, "org/kwis/msp/lwc/TextComponent", "setString", "(Ljava/lang/String;)V", (data,))
            .await
    }
    async fn init_with_height(
        jvm: &Jvm,
        context: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        data: ClassInstanceRef<String>,
        constraint: i32,
        height: i32,
    ) -> JvmResult<()> {
        Self::init(jvm, context, this.clone(), data, constraint).await?;
        jvm.put_field(&mut this, "boxHeight", "I", height).await
    }

    async fn get_height(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        jvm.get_field(&this, "boxHeight", "I").await
    }
}

#[cfg(test)]
mod tests {
    use crate::get_protos;
    use alloc::boxed::Box;
    use jvm::{ClassInstanceRef, runtime::JavaLangString};
    use rustjava_runtime::classes::java::lang::String;
    use test_utils::run_jvm_test;
    use wie_util::Result;

    #[test]
    fn constructor_overloads_preserve_text_and_explicit_height() -> Result<()> {
        run_jvm_test(Box::new([wie_midp::get_protos().into(), get_protos().into()]), |jvm| async move {
            let name = "org/kwis/msp/lwc/TextBoxComponent";
            let text = JavaLangString::from_rust_string(&jvm, "붕어빵 가게").await?;
            let a = jvm.new_class(name, "(Ljava/lang/String;II)V", (text, 0, 80)).await?;
            let b = jvm
                .new_class(name, "(Ljava/lang/String;I)V", (ClassInstanceRef::<String>::new(None), 1))
                .await?;
            let actual: ClassInstanceRef<String> = jvm.invoke_virtual(&a, name, "getString", "()Ljava/lang/String;", ()).await?;
            assert_eq!(JavaLangString::to_rust_string(&jvm, &actual).await?, "붕어빵 가게");
            let height: i32 = jvm.invoke_virtual(&a, name, "getHeight", "()I", ()).await?;
            assert_eq!(height, 80);
            let constraint: i32 = jvm.get_field(&b, "constraint", "I").await?;
            assert_eq!(constraint, 1);
            let replacement = JavaLangString::from_rust_string(&jvm, "새 이름").await?;
            let _: () = jvm.invoke_virtual(&a, name, "setString", "(Ljava/lang/String;)V", (replacement,)).await?;
            let actual: ClassInstanceRef<String> = jvm.invoke_virtual(&a, name, "getString", "()Ljava/lang/String;", ()).await?;
            assert_eq!(JavaLangString::to_rust_string(&jvm, &actual).await?, "새 이름");
            let other: ClassInstanceRef<String> = jvm.invoke_virtual(&b, name, "getString", "()Ljava/lang/String;", ()).await?;
            assert_eq!(JavaLangString::to_rust_string(&jvm, &other).await?, "");
            assert!(
                jvm.new_class(name, "(Ljava/lang/String;II)V", (ClassInstanceRef::<String>::new(None), 99, 80))
                    .await
                    .is_err()
            );
            Ok(())
        })
    }
}
