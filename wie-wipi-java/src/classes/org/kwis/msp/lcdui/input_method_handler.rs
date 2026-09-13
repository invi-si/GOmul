use alloc::vec;

use jvm::{ClassInstanceRef, Jvm, Result as JvmResult};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};

use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

// class org.kwis.msp.lcdui.InputMethodHandler
pub struct InputMethodHandler;

impl InputMethodHandler {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "org/kwis/msp/lcdui/InputMethodHandler",
            parent_class: Some("java/lang/Object"),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("<init>", "(I)V", Self::init, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setCurrentMode", "(I)Z", Self::set_current_mode, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "setInputMethodListener",
                    "(Lorg/kwis/msp/lcdui/InputMethodListener;)V",
                    Self::set_listener,
                    MethodAccessFlags::PUBLIC,
                ),
            ],
            fields: vec![JavaFieldProto::new(
                "wieListener",
                "Lorg/kwis/msp/lcdui/InputMethodListener;",
                FieldAccessFlags::PRIVATE,
            )],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }

    async fn init(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, constraint: i32) -> JvmResult<()> {
        tracing::debug!("stub org.kwis.msp.lcdui.InputMethodHandler::<init>({this:?}, {constraint})");

        let _: () = jvm.invoke_special(&this, "java/lang/Object", "<init>", "()V", ()).await?;

        Ok(())
    }

    async fn set_listener(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, listener: ClassInstanceRef<()>) -> JvmResult<()> {
        // WIPI permits null to unregister the previous listener.
        jvm.put_field(&mut this, "wieListener", "Lorg/kwis/msp/lcdui/InputMethodListener;", listener)
            .await
    }

    async fn set_current_mode(_: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, mode: i32) -> JvmResult<bool> {
        tracing::warn!("stub org.kwis.msp.lcdui.InputMethodHandler::setCurrentMode({this:?}, {mode})");

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;
    use test_utils::run_jvm_test;
    #[test]
    fn null_byte_string_raises_guest_exception() -> wie_util::Result<()> {
        run_jvm_test(Box::new([wie_midp::get_protos().into(), crate::get_protos().into()]), |jvm| async move {
            let null: ClassInstanceRef<jvm::Array<i8>> = None.into();
            let result = jvm.new_class("java/lang/String", "([B)V", (null,)).await;
            let Err(jvm::JavaError::JavaException(exception)) = result else {
                panic!("Expected NullPointerException");
            };
            assert!(jvm.is_instance(&*exception, "java/lang/NullPointerException"));
            Ok(())
        })
    }
    #[test]
    fn listener_registration_replacement_and_removal() -> wie_util::Result<()> {
        run_jvm_test(Box::new([wie_midp::get_protos().into(), crate::get_protos().into()]), |jvm| async move {
            let handler = jvm.new_class("org/kwis/msp/lcdui/InputMethodHandler", "(I)V", (0,)).await?;
            // The VM reference storage must retain the exact instance, including replacement/null.
            let first: ClassInstanceRef<()> = jvm.new_class("java/lang/Object", "()V", ()).await?.into();
            let second: ClassInstanceRef<()> = jvm.new_class("java/lang/Object", "()V", ()).await?.into();
            for listener in [first, second, ClassInstanceRef::new(None)] {
                let _: () = jvm
                    .invoke_virtual(
                        &handler,
                        "org/kwis/msp/lcdui/InputMethodHandler",
                        "setInputMethodListener",
                        "(Lorg/kwis/msp/lcdui/InputMethodListener;)V",
                        (listener.clone(),),
                    )
                    .await?;
                let stored: ClassInstanceRef<()> = jvm.get_field(&handler, "wieListener", "Lorg/kwis/msp/lcdui/InputMethodListener;").await?;
                assert_eq!(stored.is_null(), listener.is_null());
                if !stored.is_null() {
                    assert_eq!(stored.identity(), listener.identity());
                }
            }
            Ok(())
        })
    }
}
