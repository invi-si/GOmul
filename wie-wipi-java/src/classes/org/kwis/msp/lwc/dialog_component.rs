use alloc::vec;

use jvm::{ClassInstanceRef, Jvm, Result as JvmResult, runtime::JavaLangString};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use rustjava_runtime::classes::java::lang::String;
use wie_backend::Event;
use wie_jvm_support::{JvmSupport, WieJavaClassProto, WieJvmContext};

use super::Component;
use crate::classes::net::wie::WIPIKeyCode;

const NAME: &str = "org/kwis/msp/lwc/DialogComponent";
const SHELL: &str = "org/kwis/msp/lwc/ShellComponent";
const COMPONENT: &str = "org/kwis/msp/lwc/Component";
const GRAPHICS: &str = "org/kwis/msp/lcdui/Graphics";

pub struct DialogComponent;
impl DialogComponent {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "org/kwis/msp/lwc/DialogComponent",
            parent_class: Some("org/kwis/msp/lwc/ShellComponent"),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("show", "()V", Self::show, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("hide", "()V", Self::hide, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("keyNotify", "(II)Z", Self::key, MethodAccessFlags::PROTECTED),
                JavaMethodProto::new("layout", "()V", Self::layout, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("paint", "(Lorg/kwis/msp/lcdui/Graphics;)V", Self::paint, MethodAccessFlags::PROTECTED),
                JavaMethodProto::new("getActionState", "()I", Self::action, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getTimeout", "()I", Self::timeout, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setTimeout", "(I)V", Self::set_timeout, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setType", "(I)V", Self::set_type, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("wieExpire", "(I)V", Self::expire, MethodAccessFlags::PRIVATE),
                JavaMethodProto::new(
                    "<init>",
                    "(Lorg/kwis/msp/lwc/Component;Ljava/lang/String;I)V",
                    Self::init,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new(
                    "setButtonString",
                    "(ILjava/lang/String;)V",
                    Self::set_button_string,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("doModal", "()I", Self::do_modal, MethodAccessFlags::PUBLIC),
            ],
            fields: vec![
                JavaFieldProto::new("actionState", "I", FieldAccessFlags::PROTECTED),
                JavaFieldProto::new("timeout", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("generation", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("pressedKey", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("active", "Z", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("okLabel", "Ljava/lang/String;", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("cancelLabel", "Ljava/lang/String;", FieldAccessFlags::PRIVATE),
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
        let _: () = jvm
            .invoke_special(
                &this,
                "org/kwis/msp/lwc/ShellComponent",
                "setWorkComponent",
                "(Lorg/kwis/msp/lwc/Component;)V",
                (component.clone(),),
            )
            .await?;
        jvm.put_field(&mut this, "workComponent", "Lorg/kwis/msp/lwc/Component;", component)
            .await?;
        jvm.put_field(&mut this, "title", "Ljava/lang/String;", title).await?;
        jvm.put_field(&mut this, "timeout", "I", if kind == 0 { 3000 } else { -1 }).await?;
        jvm.put_field(&mut this, "bg", "I", 0xffffff).await?;
        jvm.put_field(&mut this, "actionState", "I", -2).await?;
        jvm.put_field(&mut this, "type", "I", kind).await
    }
    async fn set_button_string(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        button_type: i32,
        label: ClassInstanceRef<String>,
    ) -> JvmResult<()> {
        let field = match button_type {
            20 => "okLabel",
            21 => "cancelLabel",
            _ => return Err(jvm.exception("java/lang/IllegalArgumentException", "Invalid dialog button type").await),
        };
        let kind: i32 = jvm.get_field(&this, "type", "I").await?;
        if kind != 0 {
            jvm.put_field(&mut this, field, "Ljava/lang/String;", label).await?;
        }
        Ok(())
    }
    async fn action(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        jvm.get_field(&this, "actionState", "I").await
    }
    async fn timeout(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        jvm.get_field(&this, "timeout", "I").await
    }
    async fn set_type(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, kind: i32) -> JvmResult<()> {
        if !(0..=2).contains(&kind) {
            return Err(jvm.exception("java/lang/IllegalArgumentException", "Invalid dialog type").await);
        }
        jvm.put_field(&mut this, "type", "I", kind).await
    }
    async fn set_timeout(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, timeout: i32) -> JvmResult<()> {
        if timeout < -1 {
            return Err(jvm.exception("java/lang/IllegalArgumentException", "Invalid dialog timeout").await);
        }
        if timeout == -1 && jvm.get_field::<i32>(&this, "type", "I").await? == 0 {
            jvm.put_field(&mut this, "type", "I", 1).await?;
        }
        jvm.put_field(&mut this, "timeout", "I", timeout).await
    }
    async fn show(jvm: &Jvm, context: &mut WieJvmContext, mut this: ClassInstanceRef<Self>) -> JvmResult<()> {
        if jvm.get_field::<bool>(&this, "active", "Z").await? {
            return Ok(());
        }
        let generation = jvm.get_field::<i32>(&this, "generation", "I").await?.wrapping_add(1);
        jvm.put_field(&mut this, "generation", "I", generation).await?;
        jvm.put_field(&mut this, "actionState", "I", -2).await?;
        jvm.put_field(&mut this, "pressedKey", "I", 0).await?;
        Self::layout(jvm, context, this.clone()).await?;
        let _: () = jvm.invoke_special(&this, SHELL, "show", "()V", ()).await?;
        jvm.put_field(&mut this, "active", "Z", true).await?;
        let timeout: i32 = jvm.get_field(&this, "timeout", "I").await?;
        if timeout >= 0 {
            let due = context.system().platform().now() + timeout as u64;
            let Some(owner) = jvm.new_global_ref(&this) else {
                return Err(jvm.exception("java/lang/NullPointerException", "dialog").await);
            };
            let jvm = jvm.clone();
            context.system().event_queue().push(Event::timer(due, move || async move {
                let result: JvmResult<()> = jvm.invoke_virtual(&*owner, NAME, "wieExpire", "(I)V", (generation,)).await;
                match result {
                    Ok(()) => Ok(()),
                    Err(error) => Err(JvmSupport::to_wie_err(&jvm, error).await),
                }
            }));
        }
        Ok(())
    }
    async fn hide(jvm: &Jvm, context: &mut WieJvmContext, mut this: ClassInstanceRef<Self>) -> JvmResult<()> {
        jvm.put_field(&mut this, "active", "Z", false).await?;
        if jvm.get_field::<i32>(&this, "actionState", "I").await? == -2 {
            jvm.put_field(&mut this, "actionState", "I", 10).await?;
        }
        let _: () = jvm.invoke_special(&this, SHELL, "hide", "()V", ()).await?;
        context.system().event_queue().push(Event::Redraw);
        Ok(())
    }
    async fn expire(jvm: &Jvm, context: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, generation: i32) -> JvmResult<()> {
        if jvm.get_field::<i32>(&this, "generation", "I").await? != generation || !jvm.get_field::<bool>(&this, "active", "Z").await? {
            return Ok(());
        }
        jvm.put_field(&mut this, "actionState", "I", 10).await?;
        Self::hide(jvm, context, this).await
    }
    async fn do_modal(jvm: &Jvm, context: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        if jvm.get_field::<bool>(&this, "active", "Z").await? {
            return Err(jvm.exception("java/lang/IllegalStateException", "Dialog is already active").await);
        }
        Self::show(jvm, context, this.clone()).await?;
        let queue: ClassInstanceRef<()> = jvm
            .invoke_static("net/wie/EventQueue", "getEventQueue", "()Lnet/wie/EventQueue;", ())
            .await?;
        let event = jvm.instantiate_array("I", 4).await?;
        let result = async {
            while jvm.get_field::<i32>(&this, "actionState", "I").await? == -2 {
                let _: () = jvm
                    .invoke_virtual(&queue, "net/wie/EventQueue", "getNextEvent", "([I)V", (event.clone(),))
                    .await?;
                let _: () = jvm
                    .invoke_virtual(&queue, "net/wie/EventQueue", "dispatchEvent", "([I)V", (event.clone(),))
                    .await?;
            }
            jvm.get_field::<i32>(&this, "actionState", "I").await
        }
        .await;
        Self::hide(jvm, context, this).await?;
        result
    }
    async fn key(jvm: &Jvm, context: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, kind: i32, code: i32) -> JvmResult<bool> {
        let dialog_type: i32 = jvm.get_field(&this, "type", "I").await?;
        let soft = code == WIPIKeyCode::LEFT_SOFT_KEY as i32 || code == WIPIKeyCode::RIGHT_SOFT_KEY as i32;
        if !soft
            && jvm
                .invoke_special::<_, bool>(&this, "org/kwis/msp/lwc/ContainerComponent", "keyNotify", "(II)Z", (kind, code))
                .await?
        {
            return Ok(true);
        }
        let action = if dialog_type >= 1 && (code == WIPIKeyCode::FIRE as i32 || code == WIPIKeyCode::LEFT_SOFT_KEY as i32) {
            11
        } else if dialog_type == 2 && code == WIPIKeyCode::RIGHT_SOFT_KEY as i32 {
            12
        } else {
            0
        };
        if action != 0 {
            if kind == 1 {
                jvm.put_field(&mut this, "pressedKey", "I", code).await?;
            }
            if kind == 2 && jvm.get_field::<i32>(&this, "pressedKey", "I").await? == code {
                jvm.put_field(&mut this, "pressedKey", "I", 0).await?;
                jvm.put_field(&mut this, "actionState", "I", action).await?;
                Self::hide(jvm, context, this).await?;
            }
        }
        Ok(true)
    }
    async fn font_height(jvm: &Jvm) -> JvmResult<i32> {
        let font: ClassInstanceRef<()> = jvm
            .invoke_static("org/kwis/msp/lcdui/Font", "getDefaultFont", "()Lorg/kwis/msp/lcdui/Font;", ())
            .await?;
        jvm.invoke_virtual(&font, "org/kwis/msp/lcdui/Font", "getHeight", "()I", ()).await
    }
    async fn layout(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>) -> JvmResult<()> {
        let display: ClassInstanceRef<()> = jvm
            .invoke_static("org/kwis/msp/lcdui/Display", "getDefaultDisplay", "()Lorg/kwis/msp/lcdui/Display;", ())
            .await?;
        let sw: i32 = jvm.invoke_virtual(&display, "org/kwis/msp/lcdui/Display", "getWidth", "()I", ()).await?;
        let sh: i32 = jvm.invoke_virtual(&display, "org/kwis/msp/lcdui/Display", "getHeight", "()I", ()).await?;
        let fh = Self::font_height(jvm).await?;
        let title: ClassInstanceRef<()> = jvm.get_field(&this, "title", "Ljava/lang/String;").await?;
        let top = if title.is_null() { 4 } else { fh + 8 };
        let bottom = if jvm.get_field::<i32>(&this, "type", "I").await? == 0 {
            4
        } else {
            fh + 8
        };
        let mut work: ClassInstanceRef<()> = jvm.get_field(&this, "cmpWork", "Lorg/kwis/msp/lwc/Component;").await?;
        let height = if work.is_null() {
            fh
        } else {
            jvm.invoke_virtual::<_, i32>(&work, COMPONENT, "getPreferredHeight", "(I)I", ((sw - 8).max(0),))
                .await?
                .max(fh)
        };
        let h = (height + top + bottom).min(sh);
        let w = sw;
        for (field, value) in [("x", 0), ("y", (sh - h) / 2), ("w", w), ("h", h)] {
            jvm.put_field(&mut this, field, "I", value).await?;
        }
        let mut card: ClassInstanceRef<()> = jvm.get_field(&this, "cd", "Lorg/kwis/msp/lcdui/Card;").await?;
        if !card.is_null() {
            for (field, value) in [("x", 0), ("y", (sh - h) / 2), ("w", w), ("h", h)] {
                jvm.put_field(&mut card, field, "I", value).await?;
            }
        }
        if !work.is_null() {
            for (field, value) in [("x", 4), ("y", top), ("w", (w - 8).max(0)), ("h", (h - top - bottom).max(0))] {
                jvm.put_field(&mut work, field, "I", value).await?;
            }
            let _: () = jvm.invoke_virtual(&work, COMPONENT, "layout", "()V", ()).await?;
        }
        Ok(())
    }
    async fn paint(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, g: ClassInstanceRef<()>) -> JvmResult<()> {
        let _: () = jvm
            .invoke_special(
                &this,
                "org/kwis/msp/lwc/ContainerComponent",
                "paint",
                "(Lorg/kwis/msp/lcdui/Graphics;)V",
                (g.clone(),),
            )
            .await?;
        let _: () = jvm.invoke_virtual(&g, GRAPHICS, "setColor", "(I)V", (0,)).await?;
        let w: i32 = jvm.get_field(&this, "w", "I").await?;
        let h: i32 = jvm.get_field(&this, "h", "I").await?;
        let fh = Self::font_height(jvm).await?;
        let title: ClassInstanceRef<()> = jvm.get_field(&this, "title", "Ljava/lang/String;").await?;
        if !title.is_null() {
            let _: () = jvm
                .invoke_virtual(&g, GRAPHICS, "drawString", "(Ljava/lang/String;III)V", (title, 4, 4, 0))
                .await?;
        }
        let kind: i32 = jvm.get_field(&this, "type", "I").await?;
        for (field, fallback, x, visible) in [("okLabel", "확인", 4, kind >= 1), ("cancelLabel", "취소", w / 2, kind == 2)] {
            if visible {
                let mut label: ClassInstanceRef<()> = jvm.get_field(&this, field, "Ljava/lang/String;").await?;
                if label.is_null() {
                    label = JavaLangString::from_rust_string(jvm, fallback).await?.into();
                }
                let _: () = jvm
                    .invoke_virtual(&g, GRAPHICS, "drawString", "(Ljava/lang/String;III)V", (label, x, h - fh - 4, 0))
                    .await?;
            }
        }
        jvm.invoke_virtual(&g, GRAPHICS, "drawRect", "(IIII)V", (0, 0, w - 1, h - 1)).await
    }
}

#[cfg(test)]
mod tests {
    use crate::classes::org::kwis::msp::lwc::Component;
    use alloc::boxed::Box;
    use jvm::{ClassInstanceRef, runtime::JavaLangString};
    use test_utils::run_jvm_test;
    use wie_util::Result;
    #[test]
    fn button_labels_are_independent_and_validate_button_constants() -> Result<()> {
        run_jvm_test(
            Box::new([
                wie_midp::get_protos().into(),
                crate::classes::org::kwis::msp::lwc::shell_component::test_support::protos(),
            ]),
            |jvm| async move {
                crate::classes::org::kwis::msp::lwc::shell_component::test_support::init(&jvm).await?;
                let name = "org/kwis/msp/lwc/DialogComponent";
                let signature = "(Lorg/kwis/msp/lwc/Component;Ljava/lang/String;I)V";
                let title = JavaLangString::from_rust_string(&jvm, "Dialog").await?;
                let label = JavaLangString::from_rust_string(&jvm, "확인").await?;
                for kind in 0..=2 {
                    let dialog = jvm
                        .new_class(name, signature, (ClassInstanceRef::<Component>::new(None), title.clone(), kind))
                        .await?;
                    for (button, field) in [(20, "okLabel"), (21, "cancelLabel")] {
                        jvm.invoke_virtual::<_, ()>(&dialog, name, "setButtonString", "(ILjava/lang/String;)V", (button, label.clone()))
                            .await?;
                        let stored: ClassInstanceRef<JavaLangString> = jvm.get_field(&dialog, field, "Ljava/lang/String;").await?;
                        if kind == 0 {
                            assert!(stored.is_null());
                        } else {
                            assert_eq!(stored.identity(), label.identity());
                        }
                    }
                    for bad in [-1, 0, 19, 22] {
                        assert!(
                            jvm.invoke_virtual::<_, ()>(&dialog, name, "setButtonString", "(ILjava/lang/String;)V", (bad, label.clone()))
                                .await
                                .is_err()
                        );
                    }
                    let other = jvm
                        .new_class(name, signature, (ClassInstanceRef::<Component>::new(None), title.clone(), kind))
                        .await?;
                    let stored: ClassInstanceRef<JavaLangString> = jvm.get_field(&other, "okLabel", "Ljava/lang/String;").await?;
                    assert!(stored.is_null());
                }
                Ok(())
            },
        )
    }

    #[test]
    fn dialog_linkage_retains_arguments() -> Result<()> {
        run_jvm_test(
            Box::new([
                wie_midp::get_protos().into(),
                crate::classes::org::kwis::msp::lwc::shell_component::test_support::protos(),
            ]),
            |jvm| async move {
                crate::classes::org::kwis::msp::lwc::shell_component::test_support::init(&jvm).await?;
                let form = jvm.new_class("org/kwis/msp/lwc/FormComponent", "()V", ()).await?;
                let vertical: bool = jvm.get_field(&form, "vertical", "Z").await?;
                assert!(vertical);
                let title = JavaLangString::from_rust_string(&jvm, "이름 입력").await?;
                let name = "org/kwis/msp/lwc/DialogComponent";
                let signature = "(Lorg/kwis/msp/lwc/Component;Ljava/lang/String;I)V";
                let dialog = jvm.new_class(name, signature, (form.clone(), title.clone(), 2)).await?;
                let work: ClassInstanceRef<Component> = jvm.get_field(&dialog, "workComponent", "Lorg/kwis/msp/lwc/Component;").await?;
                assert_eq!(work.identity(), form.identity());
                let child: ClassInstanceRef<Component> = jvm
                    .invoke_virtual(
                        &dialog,
                        "org/kwis/msp/lwc/ContainerComponent",
                        "getComponent",
                        "(I)Lorg/kwis/msp/lwc/Component;",
                        (0,),
                    )
                    .await?;
                assert_eq!(child.identity(), form.identity());
                let kind: i32 = jvm.get_field(&dialog, "type", "I").await?;
                assert_eq!(kind, 2);
                assert_eq!(jvm.invoke_virtual::<_, i32>(&dialog, name, "getActionState", "()I", ()).await?, -2);
                assert!(
                    jvm.new_class(name, signature, (ClassInstanceRef::<Component>::new(None), title, 3))
                        .await
                        .is_err()
                );
                Ok(())
            },
        )
    }
}

#[cfg(test)]
mod modal_tests {
    use super::*;
    use alloc::boxed::Box;
    use test_utils::{TestPlatform, run_jvm_test_with_system};
    use wie_backend::KeyCode;
    #[test]
    fn modal_dispatches_real_input_and_timeout_without_fabricating_selection() -> wie_util::Result<()> {
        let jlet_proto = WieJavaClassProto {
            name: "test/ModalJlet",
            parent_class: Some("org/kwis/msp/lcdui/Jlet"),
            interfaces: vec![],
            methods: vec![],
            fields: vec![],
            access_flags: ClassAccessFlags::PUBLIC,
        };
        run_jvm_test_with_system(
            Box::new([wie_midp::get_protos().into(), crate::get_protos().into(), vec![jlet_proto].into()]),
            Box::new(TestPlatform::new()),
            |jvm, system| async move {
                let _midlet = jvm.new_class("net/wie/WIPIMIDlet", "()V", ()).await?;
                let jlet = jvm.instantiate_class("test/ModalJlet").await?;
                let _: () = jvm.invoke_special(&jlet, "org/kwis/msp/lcdui/Jlet", "<init>", "()V", ()).await?;
                let display: ClassInstanceRef<()> = jvm
                    .invoke_static("org/kwis/msp/lcdui/Display", "getDefaultDisplay", "()Lorg/kwis/msp/lcdui/Display;", ())
                    .await?;
                for (key, expected) in [(KeyCode::LEFT_SOFT_KEY, 11), (KeyCode::RIGHT_SOFT_KEY, 12)] {
                    let dialog = jvm
                        .new_class(
                            NAME,
                            "(Lorg/kwis/msp/lwc/Component;Ljava/lang/String;I)V",
                            (ClassInstanceRef::<()>::new(None), ClassInstanceRef::<()>::new(None), 2),
                        )
                        .await?;
                    system.event_queue().push(Event::Keydown(key));
                    system.event_queue().push(Event::Keyup(key));
                    assert_eq!(jvm.invoke_virtual::<_, i32>(&dialog, NAME, "doModal", "()I", ()).await?, expected);
                    assert_eq!(
                        jvm.invoke_virtual::<_, i32>(&display, "org/kwis/msp/lcdui/Display", "countCard", "()I", ())
                            .await?,
                        0
                    );
                }
                let dialog = jvm
                    .new_class(
                        NAME,
                        "(Lorg/kwis/msp/lwc/Component;Ljava/lang/String;I)V",
                        (ClassInstanceRef::<()>::new(None), ClassInstanceRef::<()>::new(None), 0),
                    )
                    .await?;
                let _: () = jvm.invoke_virtual(&dialog, NAME, "setTimeout", "(I)V", (0,)).await?;
                assert_eq!(jvm.invoke_virtual::<_, i32>(&dialog, NAME, "doModal", "()I", ()).await?, 10);
                let _: () = jvm.invoke_virtual(&dialog, NAME, "setTimeout", "(I)V", (-1,)).await?;
                assert_eq!(jvm.get_field::<i32>(&dialog, "type", "I").await?, 1);
                let _: () = jvm.invoke_virtual(&dialog, NAME, "show", "()V", ()).await?;
                let generation: i32 = jvm.get_field(&dialog, "generation", "I").await?;
                let _: () = jvm.invoke_virtual(&dialog, NAME, "wieExpire", "(I)V", (generation - 1,)).await?;
                assert_eq!(jvm.invoke_virtual::<_, i32>(&dialog, NAME, "getActionState", "()I", ()).await?, -2);
                let _: () = jvm.invoke_virtual(&dialog, NAME, "hide", "()V", ()).await?;
                assert_eq!(
                    jvm.invoke_virtual::<_, i32>(&display, "org/kwis/msp/lcdui/Display", "countCard", "()I", ())
                        .await?,
                    0
                );
                Ok(())
            },
        )
    }
}
