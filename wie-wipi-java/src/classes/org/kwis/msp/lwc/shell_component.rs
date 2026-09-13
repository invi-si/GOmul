use alloc::vec;

use jvm::{ClassInstanceRef, Jvm, Result as JvmResult};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

use crate::classes::org::kwis::msp::lwc::Component;

// class org.kwis.msp.lwc.ShellComponent
pub struct ShellComponent;

impl ShellComponent {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "org/kwis/msp/lwc/ShellComponent",
            parent_class: Some("org/kwis/msp/lwc/ContainerComponent"),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new(
                    "addComponent",
                    "(Lorg/kwis/msp/lwc/Component;)I",
                    Self::add_work,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new(
                    "addComponent",
                    "(ILorg/kwis/msp/lwc/Component;)V",
                    Self::insert_work,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("layout", "()V", Self::layout, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "getCommand",
                    "()Lorg/kwis/msp/lwc/Component;",
                    Self::get_command,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new(
                    "setCommand",
                    "(Lorg/kwis/msp/lwc/Component;Z)V",
                    Self::set_command,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("keyNotify", "(II)Z", Self::key_notify, MethodAccessFlags::PROTECTED),
                JavaMethodProto::new("setTitle", "(Lorg/kwis/msp/lwc/Component;)V", Self::set_title, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setTitle", "(Ljava/lang/String;)V", Self::set_title_string, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getTitle", "()Lorg/kwis/msp/lwc/Component;", Self::get_title, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "getWorkComponent",
                    "()Lorg/kwis/msp/lwc/Component;",
                    Self::get_work,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new(
                    "removeComponent",
                    "(Lorg/kwis/msp/lwc/Component;)V",
                    Self::remove_child,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("removeComponent", "(I)V", Self::remove_index, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("removeAllComponents", "()V", Self::remove_all, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("<init>", "()V", Self::init, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("<init>", "(IIII)V", Self::init_with_size, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "setWorkComponent",
                    "(Lorg/kwis/msp/lwc/Component;)V",
                    Self::set_work_component,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("show", "()V", Self::show, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("hide", "()V", Self::hide, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getCard", "()Lorg/kwis/msp/lcdui/Card;", Self::get_card, MethodAccessFlags::PUBLIC),
            ],
            fields: vec![
                JavaFieldProto::new("cd", "Lorg/kwis/msp/lcdui/Card;", FieldAccessFlags::PROTECTED),
                JavaFieldProto::new("cmpCommand", "Lorg/kwis/msp/lwc/Component;", FieldAccessFlags::PROTECTED),
                JavaFieldProto::new("wieGrabCommand", "Z", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("cmpTitle", "Lorg/kwis/msp/lwc/Component;", FieldAccessFlags::PROTECTED),
                JavaFieldProto::new("cmpWork", "Lorg/kwis/msp/lwc/Component;", FieldAccessFlags::PROTECTED),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }

    async fn init(jvm: &Jvm, context: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<()> {
        // Shells may be painted through a guest Card without calling show().
        // The no-argument SDK constructor already has display-sized geometry.
        let display: ClassInstanceRef<()> = jvm
            .invoke_static("org/kwis/msp/lcdui/Display", "getDefaultDisplay", "()Lorg/kwis/msp/lcdui/Display;", ())
            .await?;
        let width: i32 = jvm.invoke_virtual(&display, "org/kwis/msp/lcdui/Display", "getWidth", "()I", ()).await?;
        let height: i32 = jvm.invoke_virtual(&display, "org/kwis/msp/lcdui/Display", "getHeight", "()I", ()).await?;
        Self::init_with_size(jvm, context, this, 0, 0, width, height).await
    }

    async fn init_with_size(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> JvmResult<()> {
        tracing::debug!("stub org.kwis.msp.lwc.ShellComponent::<init>({this:?}, {x}, {y}, {width}, {height})");

        let _: () = jvm
            .invoke_special(&this, "org/kwis/msp/lwc/ContainerComponent", "<init>", "()V", ())
            .await?;

        jvm.put_field(&mut this, "x", "I", x).await?;
        jvm.put_field(&mut this, "y", "I", y).await?;
        jvm.put_field(&mut this, "w", "I", width).await?;
        jvm.put_field(&mut this, "h", "I", height).await?;
        Ok(())
    }

    async fn add_work(jvm: &Jvm, context: &mut WieJvmContext, this: ClassInstanceRef<Self>, child: ClassInstanceRef<Component>) -> JvmResult<i32> {
        let index: i32 = jvm.get_field(&this, "ncomp", "I").await?;
        Self::insert_work(jvm, context, this, index, child).await?;
        Ok(index)
    }

    async fn insert_work(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        index: i32,
        child: ClassInstanceRef<Component>,
    ) -> JvmResult<()> {
        // addComponent is also an SDK entry point for attaching the shell's work area.
        let _: () = jvm
            .invoke_special(
                &this,
                "org/kwis/msp/lwc/ContainerComponent",
                "addComponent",
                "(ILorg/kwis/msp/lwc/Component;)V",
                (index, child.clone()),
            )
            .await?;
        jvm.put_field(&mut this, "cmpWork", "Lorg/kwis/msp/lwc/Component;", child.clone()).await?;
        jvm.invoke_virtual(
            &this,
            "org/kwis/msp/lwc/ContainerComponent",
            "setFocus",
            "(Lorg/kwis/msp/lwc/Component;)V",
            (child,),
        )
        .await
    }

    async fn set_work_component(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        component: ClassInstanceRef<Component>,
    ) -> JvmResult<()> {
        let old: ClassInstanceRef<Component> = jvm.get_field(&this, "cmpWork", "Lorg/kwis/msp/lwc/Component;").await?;
        if old.instance.as_ref().map(|x| x.identity()) == component.instance.as_ref().map(|x| x.identity()) {
            return Ok(());
        }
        // Attach first so a rejected replacement leaves the current work area intact.
        if !component.is_null() {
            let _: i32 = jvm
                .invoke_special(
                    &this,
                    "org/kwis/msp/lwc/ContainerComponent",
                    "addComponent",
                    "(Lorg/kwis/msp/lwc/Component;)I",
                    (component.clone(),),
                )
                .await?;
        }
        if !old.is_null() {
            let _: () = jvm
                .invoke_virtual(
                    &this,
                    "org/kwis/msp/lwc/ContainerComponent",
                    "removeComponent",
                    "(Lorg/kwis/msp/lwc/Component;)V",
                    (old,),
                )
                .await?;
        }
        jvm.put_field(&mut this, "cmpWork", "Lorg/kwis/msp/lwc/Component;", component.clone())
            .await?;
        jvm.invoke_virtual(
            &this,
            "org/kwis/msp/lwc/ContainerComponent",
            "setFocus",
            "(Lorg/kwis/msp/lwc/Component;)V",
            (component,),
        )
        .await
    }
    async fn get_title(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<ClassInstanceRef<Component>> {
        jvm.get_field(&this, "cmpTitle", "Lorg/kwis/msp/lwc/Component;").await
    }
    async fn get_work(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<ClassInstanceRef<Component>> {
        jvm.get_field(&this, "cmpWork", "Lorg/kwis/msp/lwc/Component;").await
    }
    async fn set_title_string(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>, text: ClassInstanceRef<()>) -> JvmResult<()> {
        let title = if text.is_null() {
            None.into()
        } else {
            jvm.new_class("org/kwis/msp/lwc/LabelComponent", "(Ljava/lang/String;)V", (text,))
                .await?
                .into()
        };
        Self::set_title(jvm, ctx, this, title).await
    }
    async fn set_title(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>, title: ClassInstanceRef<Component>) -> JvmResult<()> {
        Self::replace_bar(jvm, ctx, this, "cmpTitle", title).await
    }
    async fn get_command(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<ClassInstanceRef<Component>> {
        jvm.get_field(&this, "cmpCommand", "Lorg/kwis/msp/lwc/Component;").await
    }
    async fn set_command(
        jvm: &Jvm,
        ctx: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        command: ClassInstanceRef<Component>,
        grab: bool,
    ) -> JvmResult<()> {
        Self::replace_bar(jvm, ctx, this.clone(), "cmpCommand", command).await?;
        jvm.put_field(&mut this, "wieGrabCommand", "Z", grab).await
    }
    async fn key_notify(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, kind: i32, code: i32) -> JvmResult<bool> {
        let command: ClassInstanceRef<Component> = jvm.get_field(&this, "cmpCommand", "Lorg/kwis/msp/lwc/Component;").await?;
        if !command.is_null() && jvm.get_field::<bool>(&this, "wieGrabCommand", "Z").await? {
            let handled: bool = jvm
                .invoke_virtual(&command, "org/kwis/msp/lwc/Component", "keyNotify", "(II)Z", (kind, code))
                .await?;
            if handled {
                return Ok(true);
            }
            // A grabbed command that also owns focus must not receive an event twice.
            let focus: ClassInstanceRef<Component> = jvm.get_field(&this, "cmpFocus", "Lorg/kwis/msp/lwc/Component;").await?;
            if !focus.is_null() && focus.identity() == command.identity() {
                return Ok(false);
            }
        }
        jvm.invoke_special(&this, "org/kwis/msp/lwc/ContainerComponent", "keyNotify", "(II)Z", (kind, code))
            .await
    }
    async fn replace_bar(
        jvm: &Jvm,
        ctx: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        field: &str,
        title: ClassInstanceRef<Component>,
    ) -> JvmResult<()> {
        let old: ClassInstanceRef<Component> = jvm.get_field(&this, field, "Lorg/kwis/msp/lwc/Component;").await?;
        if old.instance.as_ref().map(|x| x.identity()) == title.instance.as_ref().map(|x| x.identity()) {
            return Ok(());
        }
        if !title.is_null() {
            let _: i32 = jvm
                .invoke_special(
                    &this,
                    "org/kwis/msp/lwc/ContainerComponent",
                    "addComponent",
                    "(Lorg/kwis/msp/lwc/Component;)I",
                    (title.clone(),),
                )
                .await?;
        }
        if !old.is_null() {
            Self::remove_child(jvm, ctx, this.clone(), old).await?;
        }
        jvm.put_field(&mut this, field, "Lorg/kwis/msp/lwc/Component;", title).await?;
        jvm.invoke_virtual(&this, "org/kwis/msp/lwc/Component", "invalidate", "()V", ()).await
    }
    async fn clear_removed_slot(jvm: &Jvm, mut this: ClassInstanceRef<Self>, removed: &ClassInstanceRef<Component>) -> JvmResult<()> {
        for field in ["cmpTitle", "cmpWork", "cmpCommand"] {
            let child: ClassInstanceRef<Component> = jvm.get_field(&this, field, "Lorg/kwis/msp/lwc/Component;").await?;
            if !child.is_null() && !removed.is_null() && child.identity() == removed.identity() {
                jvm.put_field(&mut this, field, "Lorg/kwis/msp/lwc/Component;", ClassInstanceRef::<Component>::new(None))
                    .await?;
            }
        }
        Ok(())
    }
    async fn remove_child(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, child: ClassInstanceRef<Component>) -> JvmResult<()> {
        let _: () = jvm
            .invoke_special(
                &this,
                "org/kwis/msp/lwc/ContainerComponent",
                "removeComponent",
                "(Lorg/kwis/msp/lwc/Component;)V",
                (child.clone(),),
            )
            .await?;
        Self::clear_removed_slot(jvm, this, &child).await
    }
    async fn remove_index(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, index: i32) -> JvmResult<()> {
        let child: ClassInstanceRef<Component> = jvm
            .invoke_special(
                &this,
                "org/kwis/msp/lwc/ContainerComponent",
                "getComponent",
                "(I)Lorg/kwis/msp/lwc/Component;",
                (index,),
            )
            .await?;
        let _: () = jvm
            .invoke_special(&this, "org/kwis/msp/lwc/ContainerComponent", "removeComponent", "(I)V", (index,))
            .await?;
        Self::clear_removed_slot(jvm, this, &child).await
    }
    async fn remove_all(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>) -> JvmResult<()> {
        let _: () = jvm
            .invoke_special(&this, "org/kwis/msp/lwc/ContainerComponent", "removeAllComponents", "()V", ())
            .await?;
        for field in ["cmpTitle", "cmpWork", "cmpCommand"] {
            jvm.put_field(&mut this, field, "Lorg/kwis/msp/lwc/Component;", ClassInstanceRef::<Component>::new(None))
                .await?;
        }
        Ok(())
    }
    async fn layout(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<()> {
        let title: ClassInstanceRef<Component> = jvm.get_field(&this, "cmpTitle", "Lorg/kwis/msp/lwc/Component;").await?;
        let work: ClassInstanceRef<Component> = jvm.get_field(&this, "cmpWork", "Lorg/kwis/msp/lwc/Component;").await?;
        let command: ClassInstanceRef<Component> = jvm.get_field(&this, "cmpCommand", "Lorg/kwis/msp/lwc/Component;").await?;
        let width: i32 = jvm.get_field(&this, "w", "I").await?;
        let height: i32 = jvm.get_field(&this, "h", "I").await?;
        let title_height = if title.is_null() {
            0
        } else {
            jvm.invoke_virtual::<_, i32>(&title, "org/kwis/msp/lwc/Component", "getPreferredHeight", "(I)I", (width,))
                .await?
                .clamp(0, height.max(0))
        };
        let remaining = height.saturating_sub(title_height).max(0);
        let command_height = if command.is_null() {
            0
        } else {
            jvm.invoke_virtual::<_, i32>(&command, "org/kwis/msp/lwc/Component", "getPreferredHeight", "(I)I", (width,))
                .await?
                .clamp(0, remaining)
        };
        for (mut child, y, h) in [
            (title, 0, title_height),
            (work, title_height, remaining - command_height),
            (command, height.max(0) - command_height, command_height),
        ] {
            if child.is_null() {
                continue;
            }
            for (field, value) in [("x", 0), ("y", y), ("w", width), ("h", h)] {
                jvm.put_field(&mut child, field, "I", value).await?;
            }
            let _: () = jvm.invoke_virtual(&child, "org/kwis/msp/lwc/Component", "layout", "()V", ()).await?;
        }
        Ok(())
    }
    async fn show(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>) -> JvmResult<()> {
        let mut card: ClassInstanceRef<()> = jvm.get_field(&this, "cd", "Lorg/kwis/msp/lcdui/Card;").await?;
        if card.is_null() {
            card = jvm
                .new_class("net/wie/LwcCard", "(Lorg/kwis/msp/lwc/ShellComponent;)V", (this.clone(),))
                .await?
                .into();
            jvm.put_field(&mut this, "cd", "Lorg/kwis/msp/lcdui/Card;", card.clone()).await?;
        }
        let _: () = jvm.invoke_virtual(&this, "org/kwis/msp/lwc/Component", "layout", "()V", ()).await?;
        let shown: bool = jvm.invoke_virtual(&card, "org/kwis/msp/lcdui/Card", "isShown", "()Z", ()).await?;
        if !shown {
            let display: ClassInstanceRef<()> = jvm
                .invoke_virtual(&card, "org/kwis/msp/lcdui/Card", "getDisplay", "()Lorg/kwis/msp/lcdui/Display;", ())
                .await?;
            let _: () = jvm
                .invoke_virtual(
                    &display,
                    "org/kwis/msp/lcdui/Display",
                    "pushCard",
                    "(Lorg/kwis/msp/lcdui/Card;)V",
                    (card,),
                )
                .await?;
        }
        Ok(())
    }
    async fn hide(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<()> {
        let card: ClassInstanceRef<()> = jvm.get_field(&this, "cd", "Lorg/kwis/msp/lcdui/Card;").await?;
        if !card.is_null() {
            let display: ClassInstanceRef<()> = jvm
                .invoke_virtual(&card, "org/kwis/msp/lcdui/Card", "getDisplay", "()Lorg/kwis/msp/lcdui/Display;", ())
                .await?;
            let _: bool = jvm
                .invoke_virtual(
                    &display,
                    "org/kwis/msp/lcdui/Display",
                    "removeCard",
                    "(Lorg/kwis/msp/lcdui/Card;)Z",
                    (card,),
                )
                .await?;
        }
        Ok(())
    }
    async fn get_card(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<ClassInstanceRef<()>> {
        jvm.get_field(&this, "cd", "Lorg/kwis/msp/lcdui/Card;").await
    }
}

/// Guest-owned bridge between an LWC shell and the existing Card event loop.
pub struct LwcCard;
impl LwcCard {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "net/wie/LwcCard",
            parent_class: Some("org/kwis/msp/lcdui/Card"),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("<init>", "(Lorg/kwis/msp/lwc/ShellComponent;)V", Self::init, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("paint", "(Lorg/kwis/msp/lcdui/Graphics;)V", Self::paint, MethodAccessFlags::PROTECTED),
                JavaMethodProto::new("keyNotify", "(II)Z", Self::key, MethodAccessFlags::PROTECTED),
            ],
            fields: vec![JavaFieldProto::new(
                "owner",
                "Lorg/kwis/msp/lwc/ShellComponent;",
                FieldAccessFlags::PRIVATE,
            )],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }
    async fn init(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, mut owner: ClassInstanceRef<ShellComponent>) -> JvmResult<()> {
        let mut w: i32 = jvm.get_field(&owner, "w", "I").await?;
        let mut h: i32 = jvm.get_field(&owner, "h", "I").await?;
        if w == 0 || h == 0 {
            let display: ClassInstanceRef<()> = jvm
                .invoke_static("org/kwis/msp/lcdui/Display", "getDefaultDisplay", "()Lorg/kwis/msp/lcdui/Display;", ())
                .await?;
            if w == 0 {
                w = jvm.invoke_virtual(&display, "org/kwis/msp/lcdui/Display", "getWidth", "()I", ()).await?;
            }
            if h == 0 {
                h = jvm.invoke_virtual(&display, "org/kwis/msp/lcdui/Display", "getHeight", "()I", ()).await?;
            }
            jvm.put_field(&mut owner, "w", "I", w).await?;
            jvm.put_field(&mut owner, "h", "I", h).await?;
        }
        let x: i32 = jvm.get_field(&owner, "x", "I").await?;
        let y: i32 = jvm.get_field(&owner, "y", "I").await?;
        let _: () = jvm
            .invoke_special(&this, "org/kwis/msp/lcdui/Card", "<init>", "(IIII)V", (x, y, w, h))
            .await?;
        jvm.put_field(&mut this, "owner", "Lorg/kwis/msp/lwc/ShellComponent;", owner).await
    }
    async fn paint(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, g: ClassInstanceRef<()>) -> JvmResult<()> {
        let owner: ClassInstanceRef<ShellComponent> = jvm.get_field(&this, "owner", "Lorg/kwis/msp/lwc/ShellComponent;").await?;
        jvm.invoke_virtual(
            &owner,
            "org/kwis/msp/lwc/ContainerComponent",
            "paint",
            "(Lorg/kwis/msp/lcdui/Graphics;)V",
            (g,),
        )
        .await
    }
    async fn key(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, kind: i32, code: i32) -> JvmResult<bool> {
        let owner: ClassInstanceRef<ShellComponent> = jvm.get_field(&this, "owner", "Lorg/kwis/msp/lwc/ShellComponent;").await?;
        // Components return handled=true; Card returns propagate=true.
        let handled: bool = jvm
            .invoke_virtual(&owner, "org/kwis/msp/lwc/Component", "keyNotify", "(II)Z", (kind, code))
            .await?;
        Ok(!handled)
    }
}

#[cfg(test)]
mod bridge_tests {
    use super::*;
    use alloc::boxed::Box;
    use test_utils::run_jvm_test;
    async fn key(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<()>, _: i32, _: i32) -> JvmResult<bool> {
        jvm.get_field(&this, "handled", "Z").await
    }
    #[test]
    fn card_propagation_is_the_inverse_of_component_handling() -> wie_util::Result<()> {
        let owner = WieJavaClassProto {
            name: "test/KeyShell",
            parent_class: Some("org/kwis/msp/lwc/ShellComponent"),
            interfaces: vec![],
            methods: vec![JavaMethodProto::new("keyNotify", "(II)Z", key, MethodAccessFlags::PUBLIC)],
            fields: vec![JavaFieldProto::new("handled", "Z", FieldAccessFlags::PUBLIC)],
            access_flags: ClassAccessFlags::PUBLIC,
        };
        run_jvm_test(
            Box::new([wie_midp::get_protos().into(), crate::get_protos().into(), vec![owner].into()]),
            |jvm| async move {
                let mut owner = jvm.instantiate_class("test/KeyShell").await?;
                let mut card = jvm.instantiate_class("net/wie/LwcCard").await?;
                jvm.put_field(&mut card, "owner", "Lorg/kwis/msp/lwc/ShellComponent;", owner.clone())
                    .await?;
                for handled in [false, true] {
                    jvm.put_field(&mut owner, "handled", "Z", handled).await?;
                    let propagate: bool = jvm
                        .invoke_virtual(&card, "org/kwis/msp/lcdui/Card", "keyNotify", "(II)Z", (1, -3))
                        .await?;
                    assert_eq!(propagate, !handled);
                }
                Ok(())
            },
        )
    }
}

#[cfg(test)]
mod geometry_tests {
    use super::*;
    use alloc::boxed::Box;
    use test_utils::run_jvm_test;

    async fn content<C: Send>(_: &Jvm, _: &mut C, _: ClassInstanceRef<()>, _: ClassInstanceRef<()>) -> JvmResult<()> {
        // Guest containers often paint their own backdrop without invoking super.
        Ok(())
    }

    #[test]
    fn default_shell_and_guest_paint_override_layout_before_clipping_without_show() -> wie_util::Result<()> {
        let guest = jvm_class_proto::JavaClassProto {
            name: "test/PaintedShell",
            parent_class: Some("org/kwis/msp/lwc/ShellComponent"),
            interfaces: vec![],
            methods: vec![JavaMethodProto::new(
                "paintContent",
                "(Lorg/kwis/msp/lcdui/Graphics;)V",
                content,
                MethodAccessFlags::PUBLIC,
            )],
            fields: vec![],
            access_flags: ClassAccessFlags::PUBLIC,
        };
        run_jvm_test(
            Box::new([wie_midp::get_protos().into(), super::test_support::protos(), vec![guest].into()]),
            |jvm| async move {
                super::test_support::init(&jvm).await?;
                let shell = jvm.instantiate_class("test/PaintedShell").await?;
                let _: () = jvm.invoke_special(&shell, "org/kwis/msp/lwc/ShellComponent", "<init>", "()V", ()).await?;
                let display: ClassInstanceRef<()> = jvm
                    .invoke_static("org/kwis/msp/lcdui/Display", "getDefaultDisplay", "()Lorg/kwis/msp/lcdui/Display;", ())
                    .await?;
                let w: i32 = jvm.invoke_virtual(&display, "org/kwis/msp/lcdui/Display", "getWidth", "()I", ()).await?;
                let h: i32 = jvm.invoke_virtual(&display, "org/kwis/msp/lcdui/Display", "getHeight", "()I", ()).await?;
                assert!(w > 0 && h > 0);
                assert_eq!(jvm.get_field::<i32>(&shell, "w", "I").await?, w);
                assert_eq!(jvm.get_field::<i32>(&shell, "h", "I").await?, h);
                let card: ClassInstanceRef<()> = jvm.get_field(&shell, "cd", "Lorg/kwis/msp/lcdui/Card;").await?;
                assert!(card.is_null());
                let child = jvm.new_class("org/kwis/msp/lwc/ButtonComponent", "()V", ()).await?;
                let _: () = jvm
                    .invoke_virtual(&child, "org/kwis/msp/lwc/Component", "setBackground", "(I)V", (0x123456,))
                    .await?;
                let index: i32 = jvm
                    .invoke_virtual(
                        &shell,
                        "org/kwis/msp/lwc/ShellComponent",
                        "addComponent",
                        "(Lorg/kwis/msp/lwc/Component;)I",
                        (child.clone(),),
                    )
                    .await?;
                assert_eq!(index, 0);
                let image: ClassInstanceRef<()> = jvm
                    .invoke_static("org/kwis/msp/lcdui/Image", "createImage", "(II)Lorg/kwis/msp/lcdui/Image;", (w, h))
                    .await?;
                let g: ClassInstanceRef<()> = jvm
                    .invoke_virtual(&image, "org/kwis/msp/lcdui/Image", "getGraphics", "()Lorg/kwis/msp/lcdui/Graphics;", ())
                    .await?;
                let _: () = jvm
                    .invoke_virtual(
                        &shell,
                        "org/kwis/msp/lwc/ContainerComponent",
                        "paint",
                        "(Lorg/kwis/msp/lcdui/Graphics;)V",
                        (g.clone(),),
                    )
                    .await?;
                assert_eq!(jvm.get_field::<i32>(&child, "w", "I").await?, w);
                assert_eq!(jvm.get_field::<i32>(&child, "h", "I").await?, h);
                assert_eq!(
                    jvm.invoke_virtual::<_, i32>(&g, "org/kwis/msp/lcdui/Graphics", "getPixel", "(II)I", (w - 1, h - 1))
                        .await?,
                    0x123456
                );
                assert!(
                    jvm.invoke_virtual::<_, bool>(&child, "org/kwis/msp/lwc/Component", "canHandleInput", "()Z", ())
                        .await?
                );
                let sized = jvm.new_class("org/kwis/msp/lwc/ShellComponent", "(IIII)V", (3, 5, 40, 60)).await?;
                for (field, expected) in [("x", 3), ("y", 5), ("w", 40), ("h", 60)] {
                    assert_eq!(jvm.get_field::<i32>(&sized, field, "I").await?, expected);
                }
                Ok(())
            },
        )
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    pub fn protos() -> alloc::boxed::Box<[WieJavaClassProto]> {
        let mut protos = crate::get_protos().into_iter().collect::<alloc::vec::Vec<_>>();
        protos.push(WieJavaClassProto {
            name: "test/LwcJlet",
            parent_class: Some("org/kwis/msp/lcdui/Jlet"),
            interfaces: vec![],
            methods: vec![],
            fields: vec![],
            access_flags: ClassAccessFlags::PUBLIC,
        });
        protos.into_boxed_slice()
    }
    pub async fn init(jvm: &Jvm) -> JvmResult<()> {
        let _midlet = jvm.new_class("net/wie/WIPIMIDlet", "()V", ()).await?;
        let jlet = jvm.instantiate_class("test/LwcJlet").await?;
        jvm.invoke_special(&jlet, "org/kwis/msp/lcdui/Jlet", "<init>", "()V", ()).await
    }
}

#[cfg(test)]
mod title_tests {
    use super::*;
    use alloc::boxed::Box;
    use test_utils::run_jvm_test;
    use wie_util::Result;

    #[test]
    fn title_layout_replacement_and_removal_preserve_work_focus() -> Result<()> {
        run_jvm_test(Box::new([wie_midp::get_protos().into(), test_support::protos()]), |jvm| async move {
            test_support::init(&jvm).await?;
            let shell_name = "org/kwis/msp/lwc/ShellComponent";
            let component = "org/kwis/msp/lwc/LabelComponent";
            let container = "org/kwis/msp/lwc/ContainerComponent";
            let shell = jvm.new_class(shell_name, "(IIII)V", (3, 4, 100, 80)).await?;
            let work = jvm.new_class("org/kwis/msp/lwc/FormComponent", "()V", ()).await?;
            let _: () = jvm
                .invoke_virtual(&shell, shell_name, "setWorkComponent", "(Lorg/kwis/msp/lwc/Component;)V", (work.clone(),))
                .await?;
            let text = jvm::runtime::JavaLangString::from_rust_string(&jvm, "제목").await?;
            let title = jvm.new_class(component, "(Ljava/lang/String;)V", (text,)).await?;
            let title_height: i32 = jvm.invoke_virtual(&title, component, "getPreferredHeight", "(I)I", (100,)).await?;
            assert!(title_height > 0 && title_height < 80);
            let _: () = jvm
                .invoke_virtual(&shell, shell_name, "setTitle", "(Lorg/kwis/msp/lwc/Component;)V", (title.clone(),))
                .await?;
            let _: () = jvm.invoke_virtual(&shell, shell_name, "layout", "()V", ()).await?;
            assert_eq!(jvm.get_field::<i32>(&work, "y", "I").await?, title_height);
            assert_eq!(jvm.get_field::<i32>(&work, "h", "I").await?, 80 - title_height);
            assert_eq!(jvm.get_field::<i32>(&title, "w", "I").await?, 100);
            let focus: ClassInstanceRef<()> = jvm.get_field(&shell, "cmpFocus", "Lorg/kwis/msp/lwc/Component;").await?;
            assert_eq!(focus.identity(), work.identity());
            // Failed reparenting must leave the existing title intact.
            let other = jvm.new_class(shell_name, "()V", ()).await?;
            let replacement = jvm.new_class(component, "()V", ()).await?;
            let _: i32 = jvm
                .invoke_virtual(
                    &other,
                    container,
                    "addComponent",
                    "(Lorg/kwis/msp/lwc/Component;)I",
                    (replacement.clone(),),
                )
                .await?;
            assert!(
                jvm.invoke_virtual::<_, ()>(&shell, shell_name, "setTitle", "(Lorg/kwis/msp/lwc/Component;)V", (replacement,))
                    .await
                    .is_err()
            );
            let current: ClassInstanceRef<()> = jvm
                .invoke_virtual(&shell, shell_name, "getTitle", "()Lorg/kwis/msp/lwc/Component;", ())
                .await?;
            assert_eq!(current.identity(), title.identity());
            let _: () = jvm.invoke_virtual(&shell, container, "removeComponent", "(I)V", (1,)).await?;
            let current: ClassInstanceRef<()> = jvm
                .invoke_virtual(&shell, shell_name, "getTitle", "()Lorg/kwis/msp/lwc/Component;", ())
                .await?;
            assert!(current.is_null());
            let _: () = jvm.invoke_virtual(&shell, shell_name, "layout", "()V", ()).await?;
            assert_eq!(jvm.get_field::<i32>(&work, "y", "I").await?, 0);
            assert_eq!(jvm.get_field::<i32>(&work, "h", "I").await?, 80);
            let text = jvm::runtime::JavaLangString::from_rust_string(&jvm, "제목").await?;
            let _: () = jvm
                .invoke_virtual(&shell, shell_name, "setTitle", "(Ljava/lang/String;)V", (text,))
                .await?;
            let _: () = jvm.invoke_virtual(&shell, container, "removeAllComponents", "()V", ()).await?;
            for method in ["getTitle", "getWorkComponent"] {
                let current: ClassInstanceRef<()> = jvm
                    .invoke_virtual(&shell, shell_name, method, "()Lorg/kwis/msp/lwc/Component;", ())
                    .await?;
                assert!(current.is_null());
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod command_area_tests {
    use super::*;
    use alloc::boxed::Box;
    use test_utils::run_jvm_test;
    async fn key(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<()>, _: i32, code: i32) -> JvmResult<bool> {
        let count: i32 = jvm.get_field(&this, "calls", "I").await?;
        jvm.put_field(&mut this, "calls", "I", count + 1).await?;
        jvm.put_field(&mut this, "lastKey", "I", code).await?;
        jvm.get_field(&this, "consume", "Z").await
    }
    #[test]
    fn command_layout_and_grab_priority_preserve_focus_and_avoid_duplicate_delivery() -> wie_util::Result<()> {
        let mut protos = test_support::protos().into_vec();
        protos.push(WieJavaClassProto {
            name: "test/KeyArea",
            parent_class: Some("org/kwis/msp/lwc/Component"),
            interfaces: vec![],
            methods: vec![JavaMethodProto::new("keyNotify", "(II)Z", key, MethodAccessFlags::PUBLIC)],
            fields: vec![
                JavaFieldProto::new("calls", "I", FieldAccessFlags::PUBLIC),
                JavaFieldProto::new("consume", "Z", FieldAccessFlags::PUBLIC),
                JavaFieldProto::new("lastKey", "I", FieldAccessFlags::PUBLIC),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
        });
        run_jvm_test(Box::new([wie_midp::get_protos().into(), protos.into_boxed_slice()]), |jvm| async move {
            test_support::init(&jvm).await?;
            let shell_name = "org/kwis/msp/lwc/ShellComponent";
            let component = "org/kwis/msp/lwc/Component";
            let container = "org/kwis/msp/lwc/ContainerComponent";
            let shell = jvm.new_class(shell_name, "(IIII)V", (0, 0, 100, 80)).await?;
            let work = jvm.instantiate_class("test/KeyArea").await?;
            let mut command = jvm.instantiate_class("test/KeyArea").await?;
            let mut title = jvm.instantiate_class("test/KeyArea").await?;
            for child in [&work, &command, &title] {
                let _: () = jvm.invoke_special(child, component, "<init>", "()V", ()).await?;
            }
            jvm.put_field(&mut command, "h", "I", 18).await?;
            jvm.put_field(&mut title, "h", "I", 10).await?;
            let _: () = jvm
                .invoke_virtual(&shell, shell_name, "setWorkComponent", "(Lorg/kwis/msp/lwc/Component;)V", (work.clone(),))
                .await?;
            let _: () = jvm
                .invoke_virtual(&shell, shell_name, "setTitle", "(Lorg/kwis/msp/lwc/Component;)V", (title.clone(),))
                .await?;
            let _: () = jvm
                .invoke_virtual(
                    &shell,
                    shell_name,
                    "setCommand",
                    "(Lorg/kwis/msp/lwc/Component;Z)V",
                    (command.clone(), true),
                )
                .await?;
            let _: () = jvm.invoke_virtual(&shell, shell_name, "layout", "()V", ()).await?;
            for (child, y, h) in [(&title, 0, 10), (&work, 10, 52), (&command, 62, 18)] {
                assert_eq!(jvm.get_field::<i32>(child, "y", "I").await?, y);
                assert_eq!(jvm.get_field::<i32>(child, "h", "I").await?, h);
                assert_eq!(jvm.get_field::<i32>(child, "w", "I").await?, 100);
            }
            let focus: ClassInstanceRef<()> = jvm.get_field(&shell, "cmpFocus", "Lorg/kwis/msp/lwc/Component;").await?;
            assert_eq!(focus.identity(), work.identity());
            jvm.put_field(&mut command, "consume", "Z", true).await?;
            assert!(jvm.invoke_virtual::<_, bool>(&shell, shell_name, "keyNotify", "(II)Z", (1, 49)).await?);
            assert_eq!(jvm.get_field::<i32>(&work, "calls", "I").await?, 0);
            assert_eq!(jvm.get_field::<i32>(&command, "lastKey", "I").await?, 49);
            jvm.put_field(&mut command, "consume", "Z", false).await?;
            assert!(!jvm.invoke_virtual::<_, bool>(&shell, shell_name, "keyNotify", "(II)Z", (2, 49)).await?);
            assert_eq!(jvm.get_field::<i32>(&work, "calls", "I").await?, 1);
            // Updating the same bar changes grab policy without reattaching it.
            let _: () = jvm
                .invoke_virtual(
                    &shell,
                    shell_name,
                    "setCommand",
                    "(Lorg/kwis/msp/lwc/Component;Z)V",
                    (command.clone(), false),
                )
                .await?;
            let _: bool = jvm.invoke_virtual(&shell, shell_name, "keyNotify", "(II)Z", (1, 50)).await?;
            assert_eq!(jvm.get_field::<i32>(&command, "calls", "I").await?, 2);
            assert_eq!(jvm.get_field::<i32>(&work, "calls", "I").await?, 2);
            let _: () = jvm
                .invoke_virtual(&shell, container, "setFocus", "(Lorg/kwis/msp/lwc/Component;)V", (command.clone(),))
                .await?;
            let _: () = jvm
                .invoke_virtual(
                    &shell,
                    shell_name,
                    "setCommand",
                    "(Lorg/kwis/msp/lwc/Component;Z)V",
                    (command.clone(), true),
                )
                .await?;
            let _: bool = jvm.invoke_virtual(&shell, shell_name, "keyNotify", "(II)Z", (1, 51)).await?;
            assert_eq!(jvm.get_field::<i32>(&command, "calls", "I").await?, 3);
            let _: () = jvm
                .invoke_virtual(
                    &shell,
                    container,
                    "removeComponent",
                    "(Lorg/kwis/msp/lwc/Component;)V",
                    (command.clone(),),
                )
                .await?;
            let removed: ClassInstanceRef<()> = jvm
                .invoke_virtual(&shell, shell_name, "getCommand", "()Lorg/kwis/msp/lwc/Component;", ())
                .await?;
            assert!(removed.is_null());
            let _: bool = jvm.invoke_virtual(&shell, shell_name, "keyNotify", "(II)Z", (1, 52)).await?;
            assert_eq!(jvm.get_field::<i32>(&command, "calls", "I").await?, 3);
            let _: () = jvm.invoke_virtual(&shell, shell_name, "layout", "()V", ()).await?;
            assert_eq!(jvm.get_field::<i32>(&work, "h", "I").await?, 70);
            Ok(())
        })
    }
}
