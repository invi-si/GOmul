use alloc::vec;
use jvm::{ClassInstanceRef, Jvm, Result};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use wie_jvm_support::{WieJavaClassProto, WieJvmContext};
pub struct ButtonComponent;
const COMPONENT: &str = "org/kwis/msp/lwc/Component";
const LABEL: &str = "org/kwis/msp/lwc/LabelComponent";
const NAME: &str = "org/kwis/msp/lwc/ButtonComponent";
impl ButtonComponent {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: NAME,
            parent_class: Some(COMPONENT),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("calcPreferredSize", "(I)V", Self::preferred, MethodAccessFlags::PROTECTED),
                JavaMethodProto::new("<init>", "()V", Self::init, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "<init>",
                    "(Ljava/lang/String;Lorg/kwis/msp/lcdui/Image;)V",
                    Self::init_content,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new(
                    "setActionListener",
                    "(Lorg/kwis/msp/lwc/ActionListener;Ljava/lang/Object;)V",
                    Self::listener,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("keyNotify", "(II)Z", Self::key, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("focusNotify", "(Z)V", Self::focus, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("paintContent", "(Lorg/kwis/msp/lcdui/Graphics;)V", Self::paint, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getString", "()Ljava/lang/String;", Self::get_string, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setString", "(Ljava/lang/String;)V", Self::set_string, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getImage", "()Lorg/kwis/msp/lcdui/Image;", Self::get_image, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setImage", "(Lorg/kwis/msp/lcdui/Image;)V", Self::set_image, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getFont", "()Lorg/kwis/msp/lcdui/Font;", Self::get_font, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setFont", "(Lorg/kwis/msp/lcdui/Font;)V", Self::set_font, MethodAccessFlags::PUBLIC),
            ],
            fields: vec![
                JavaFieldProto::new("label", "Lorg/kwis/msp/lwc/LabelComponent;", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("listener", "Lorg/kwis/msp/lwc/ActionListener;", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("argument", "Ljava/lang/Object;", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("pressed", "Z", FieldAccessFlags::PRIVATE),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }
    async fn init(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<()> {
        Self::init_content(jvm, ctx, this, None.into(), None.into()).await
    }
    async fn init_content(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        text: ClassInstanceRef<()>,
        image: ClassInstanceRef<()>,
    ) -> Result<()> {
        let _: () = jvm.invoke_special(&this, COMPONENT, "<init>", "()V", ()).await?;
        jvm.put_field(&mut this, "wieInput", "Z", true).await?;
        let label = jvm
            .new_class(LABEL, "(Ljava/lang/String;Lorg/kwis/msp/lcdui/Image;)V", (text, image))
            .await?;
        jvm.put_field(&mut this, "label", "Lorg/kwis/msp/lwc/LabelComponent;", label).await
    }
    async fn listener(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        listener: ClassInstanceRef<()>,
        argument: ClassInstanceRef<()>,
    ) -> Result<()> {
        jvm.put_field(&mut this, "listener", "Lorg/kwis/msp/lwc/ActionListener;", listener)
            .await?;
        jvm.put_field(&mut this, "argument", "Ljava/lang/Object;", argument).await
    }
    async fn focus(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, focused: bool) -> Result<()> {
        let _: () = jvm.invoke_special(&this, COMPONENT, "focusNotify", "(Z)V", (focused,)).await?;
        if !focused {
            jvm.put_field(&mut this, "pressed", "Z", false).await?;
        }
        Ok(())
    }
    async fn key(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, kind: i32, code: i32) -> Result<bool> {
        if code != crate::classes::net::wie::WIPIKeyCode::FIRE as i32 {
            return Ok(false);
        }
        if kind == 1 {
            jvm.put_field(&mut this, "pressed", "Z", true).await?;
        }
        if kind == 2 {
            let pressed: bool = jvm.get_field(&this, "pressed", "Z").await?;
            jvm.put_field(&mut this, "pressed", "Z", false).await?;
            if pressed {
                let listener: ClassInstanceRef<()> = jvm.get_field(&this, "listener", "Lorg/kwis/msp/lwc/ActionListener;").await?;
                let argument: ClassInstanceRef<()> = jvm.get_field(&this, "argument", "Ljava/lang/Object;").await?;
                if !listener.is_null() {
                    let _: () = jvm
                        .invoke_virtual(
                            &listener,
                            "org/kwis/msp/lwc/ActionListener",
                            "action",
                            "(Lorg/kwis/msp/lwc/Component;Ljava/lang/Object;)V",
                            (this.clone(), argument),
                        )
                        .await?;
                }
            }
        }
        let _: () = jvm.invoke_virtual(&this, COMPONENT, "repaint", "()V", ()).await?;
        Ok(true)
    }
    async fn preferred(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, width: i32) -> Result<()> {
        let label: ClassInstanceRef<()> = jvm.get_field(&this, "label", "Lorg/kwis/msp/lwc/LabelComponent;").await?;
        let _: () = jvm.invoke_virtual(&label, LABEL, "calcPreferredSize", "(I)V", (width,)).await?;
        for field in ["prefW", "prefH"] {
            let value: i32 = jvm.get_field(&label, field, "I").await?;
            jvm.put_field(&mut this, field, "I", value).await?;
        }
        Ok(())
    }
    async fn get_string(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<ClassInstanceRef<()>> {
        let label: ClassInstanceRef<()> = jvm.get_field(&this, "label", "Lorg/kwis/msp/lwc/LabelComponent;").await?;
        jvm.invoke_virtual(&label, LABEL, "getLabel", "()Ljava/lang/String;", ()).await
    }
    async fn set_string(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, value: ClassInstanceRef<()>) -> Result<()> {
        let label: ClassInstanceRef<()> = jvm.get_field(&this, "label", "Lorg/kwis/msp/lwc/LabelComponent;").await?;
        let _: () = jvm.invoke_virtual(&label, LABEL, "setLabel", "(Ljava/lang/String;)V", (value,)).await?;
        jvm.invoke_virtual(&this, COMPONENT, "repaint", "()V", ()).await
    }
    async fn get_image(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<ClassInstanceRef<()>> {
        let label: ClassInstanceRef<()> = jvm.get_field(&this, "label", "Lorg/kwis/msp/lwc/LabelComponent;").await?;
        jvm.invoke_virtual(&label, LABEL, "getImage", "()Lorg/kwis/msp/lcdui/Image;", ()).await
    }
    async fn set_image(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, value: ClassInstanceRef<()>) -> Result<()> {
        let label: ClassInstanceRef<()> = jvm.get_field(&this, "label", "Lorg/kwis/msp/lwc/LabelComponent;").await?;
        let _: () = jvm
            .invoke_virtual(&label, LABEL, "setImage", "(Lorg/kwis/msp/lcdui/Image;)V", (value,))
            .await?;
        jvm.invoke_virtual(&this, COMPONENT, "repaint", "()V", ()).await
    }
    async fn get_font(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<ClassInstanceRef<()>> {
        let label: ClassInstanceRef<()> = jvm.get_field(&this, "label", "Lorg/kwis/msp/lwc/LabelComponent;").await?;
        jvm.invoke_virtual(&label, LABEL, "getFont", "()Lorg/kwis/msp/lcdui/Font;", ()).await
    }
    async fn set_font(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, value: ClassInstanceRef<()>) -> Result<()> {
        let label: ClassInstanceRef<()> = jvm.get_field(&this, "label", "Lorg/kwis/msp/lwc/LabelComponent;").await?;
        let _: () = jvm
            .invoke_virtual(&label, LABEL, "setFont", "(Lorg/kwis/msp/lcdui/Font;)V", (value,))
            .await?;
        jvm.invoke_virtual(&this, COMPONENT, "repaint", "()V", ()).await
    }
    async fn paint(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, g: ClassInstanceRef<()>) -> Result<()> {
        let _: () = jvm
            .invoke_special(&this, COMPONENT, "paintContent", "(Lorg/kwis/msp/lcdui/Graphics;)V", (g.clone(),))
            .await?;
        let mut label: ClassInstanceRef<()> = jvm.get_field(&this, "label", "Lorg/kwis/msp/lwc/LabelComponent;").await?;
        for field in ["w", "h", "fg"] {
            let value: i32 = jvm.get_field(&this, field, "I").await?;
            jvm.put_field(&mut label, field, "I", value).await?;
        }
        jvm.invoke_virtual(&label, LABEL, "paintContent", "(Lorg/kwis/msp/lcdui/Graphics;)V", (g,))
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;
    use test_utils::run_jvm_test;
    async fn action<C: Send>(
        jvm: &Jvm,
        _: &mut C,
        mut this: ClassInstanceRef<()>,
        source: ClassInstanceRef<()>,
        argument: ClassInstanceRef<()>,
    ) -> Result<()> {
        let count: i32 = jvm.get_field(&this, "count", "I").await?;
        jvm.put_field(&mut this, "count", "I", count + 1).await?;
        jvm.put_field(&mut this, "source", "Ljava/lang/Object;", source).await?;
        jvm.put_field(&mut this, "argument", "Ljava/lang/Object;", argument).await
    }
    #[test]
    fn button_requires_paired_select_and_preserves_listener_arguments() -> wie_util::Result<()> {
        let listener = jvm_class_proto::JavaClassProto {
            name: "test/ButtonListener",
            parent_class: Some("java/lang/Object"),
            interfaces: vec!["org/kwis/msp/lwc/ActionListener"],
            methods: vec![JavaMethodProto::new(
                "action",
                "(Lorg/kwis/msp/lwc/Component;Ljava/lang/Object;)V",
                action,
                MethodAccessFlags::PUBLIC,
            )],
            fields: vec![
                JavaFieldProto::new("count", "I", FieldAccessFlags::PUBLIC),
                JavaFieldProto::new("source", "Ljava/lang/Object;", FieldAccessFlags::PUBLIC),
                JavaFieldProto::new("argument", "Ljava/lang/Object;", FieldAccessFlags::PUBLIC),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
        };
        run_jvm_test(
            Box::new([wie_midp::get_protos().into(), crate::get_protos().into(), vec![listener].into()]),
            |jvm| async move {
                let button = jvm.new_class(NAME, "()V", ()).await?;
                let first = jvm.instantiate_class("test/ButtonListener").await?;
                let second = jvm.instantiate_class("test/ButtonListener").await?;
                let argument = jvm.new_class("java/lang/Object", "()V", ()).await?;
                let fire = crate::classes::net::wie::WIPIKeyCode::FIRE as i32;
                let _: () = jvm
                    .invoke_virtual(
                        &button,
                        NAME,
                        "setActionListener",
                        "(Lorg/kwis/msp/lwc/ActionListener;Ljava/lang/Object;)V",
                        (first.clone(), argument.clone()),
                    )
                    .await?;
                for kind in [2, 3, 1, 3, 3] {
                    assert!(jvm.invoke_virtual::<_, bool>(&button, NAME, "keyNotify", "(II)Z", (kind, fire)).await?);
                }
                assert_eq!(jvm.get_field::<i32>(&first, "count", "I").await?, 0);
                let _: () = jvm
                    .invoke_virtual(
                        &button,
                        NAME,
                        "setActionListener",
                        "(Lorg/kwis/msp/lwc/ActionListener;Ljava/lang/Object;)V",
                        (second.clone(), argument.clone()),
                    )
                    .await?;
                for _ in 0..2 {
                    let _: bool = jvm.invoke_virtual(&button, NAME, "keyNotify", "(II)Z", (2, fire)).await?;
                }
                assert_eq!(jvm.get_field::<i32>(&first, "count", "I").await?, 0);
                assert_eq!(jvm.get_field::<i32>(&second, "count", "I").await?, 1);
                let source: ClassInstanceRef<()> = jvm.get_field(&second, "source", "Ljava/lang/Object;").await?;
                let arg: ClassInstanceRef<()> = jvm.get_field(&second, "argument", "Ljava/lang/Object;").await?;
                assert_eq!(source.identity(), button.identity());
                assert_eq!(arg.identity(), argument.identity());
                let _: bool = jvm.invoke_virtual(&button, NAME, "keyNotify", "(II)Z", (1, fire)).await?;
                let _: () = jvm.invoke_virtual(&button, NAME, "focusNotify", "(Z)V", (false,)).await?;
                let _: bool = jvm.invoke_virtual(&button, NAME, "keyNotify", "(II)Z", (2, fire)).await?;
                assert_eq!(jvm.get_field::<i32>(&second, "count", "I").await?, 1);
                assert!(!jvm.invoke_virtual::<_, bool>(&button, NAME, "keyNotify", "(II)Z", (1, 48)).await?);
                let _: () = jvm
                    .invoke_virtual(
                        &button,
                        NAME,
                        "setActionListener",
                        "(Lorg/kwis/msp/lwc/ActionListener;Ljava/lang/Object;)V",
                        (ClassInstanceRef::<()>::new(None), ClassInstanceRef::<()>::new(None)),
                    )
                    .await?;
                for kind in [1, 2] {
                    let _: bool = jvm.invoke_virtual(&button, NAME, "keyNotify", "(II)Z", (kind, fire)).await?;
                }
                let text: ClassInstanceRef<()> = jvm.invoke_virtual(&button, NAME, "getString", "()Ljava/lang/String;", ()).await?;
                assert!(text.is_null());
                Ok(())
            },
        )
    }
}
