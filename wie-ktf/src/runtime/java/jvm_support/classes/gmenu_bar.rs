use alloc::vec;
use jvm::{Array, ClassInstanceRef, Jvm, Result};
use jvm_class_proto::JavaMethodProto;
use jvm_types::{ClassAccessFlags, MethodAccessFlags};
use wie_jvm_support::{WieJavaClassProto, WieJvmContext};
use wie_wipi_java::classes::net::wie::WIPIKeyCode;

pub struct GMenuBar;
const NAME: &str = "com/ktf/kfc/GMenuBar";
const BAR: &str = "org/kwis/msp/lwc/CommandBarComponent";
const CMD: &str = "Lorg/kwis/msp/lwc/Command;";
const CMDS: &str = "[Lorg/kwis/msp/lwc/Command;";
type Commands = ClassInstanceRef<Array<ClassInstanceRef<()>>>;
impl GMenuBar {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: NAME,
            parent_class: Some(BAR),
            interfaces: vec![],
            fields: vec![],
            methods: vec![
                JavaMethodProto::new("addCommand", "(Lorg/kwis/msp/lwc/Command;)I", Self::add, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "removeCommand",
                    "(Lorg/kwis/msp/lwc/Command;)V",
                    Self::remove_object,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("removeAll", "()V", Self::remove_all, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("<init>", "()V", Self::init, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setItem", "(ILorg/kwis/msp/lwc/Command;)V", Self::set_item, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setItem", "(ILjava/lang/String;)V", Self::set_string, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("itemAt", "(I)Lorg/kwis/msp/lwc/Command;", Self::item, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("removeCommand", "(I)V", Self::remove, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("removeAllCommand", "()V", Self::remove_all, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "getCommandListener",
                    "()Lorg/kwis/msp/lwc/CommandListener;",
                    Self::listener,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("keyNotify", "(II)Z", Self::key, MethodAccessFlags::PROTECTED),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }
    async fn init(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<()> {
        let _: () = jvm.invoke_special(&this, BAR, "<init>", "()V", ()).await?;
        Self::remove_all(jvm, ctx, this).await
    }
    async fn add(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>, command: ClassInstanceRef<()>) -> Result<i32> {
        if command.is_null() {
            return Ok(-1);
        }
        for position in 0..3 {
            if Self::item(jvm, ctx, this.clone(), position).await?.is_null() {
                Self::set_item(jvm, ctx, this, position, command).await?;
                return Ok(position);
            }
        }
        Ok(-1)
    }
    async fn remove_object(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>, command: ClassInstanceRef<()>) -> Result<()> {
        if command.is_null() {
            return Ok(());
        }
        for position in 0..3 {
            let item = Self::item(jvm, ctx, this.clone(), position).await?;
            if !item.is_null() && item.identity() == command.identity() {
                return Self::remove(jvm, ctx, this, position).await;
            }
        }
        Ok(())
    }
    async fn remove_all(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>) -> Result<()> {
        let slots = jvm.instantiate_array(CMD, 3).await?;
        jvm.put_field_in_class(&mut this, BAR, "wieCommands", CMDS, slots).await?;
        jvm.put_field_in_class(&mut this, BAR, "wieSelected", "I", -1).await?;
        Self::changed(jvm, this).await
    }
    async fn changed(jvm: &Jvm, mut this: ClassInstanceRef<Self>) -> Result<()> {
        jvm.put_field_in_class(&mut this, BAR, "wiePressedCommand", CMD, ClassInstanceRef::<()>::from(None))
            .await?;
        jvm.put_field_in_class(&mut this, BAR, "wieSuppressTyped", "Z", false).await?;
        let _: () = jvm.invoke_virtual(&this, "org/kwis/msp/lwc/Component", "invalidate", "()V", ()).await?;
        jvm.invoke_virtual(&this, "org/kwis/msp/lwc/Component", "repaint", "()V", ()).await
    }
    async fn set_item(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, position: i32, command: ClassInstanceRef<()>) -> Result<()> {
        if !(0..3).contains(&position) {
            return Err(jvm.exception("java/lang/IllegalArgumentException", "menu position").await);
        }
        let mut slots: Commands = jvm.get_field_in_class(&this, BAR, "wieCommands", CMDS).await?;
        jvm.store_array(&mut slots, position as usize, vec![command]).await?;
        Self::changed(jvm, this).await
    }
    async fn set_string(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>, position: i32, text: ClassInstanceRef<()>) -> Result<()> {
        let command = if text.is_null() {
            None.into()
        } else {
            jvm.new_class(
                "org/kwis/msp/lwc/Command",
                "(Ljava/lang/String;Ljava/lang/Object;)V",
                (text, ClassInstanceRef::<()>::from(None)),
            )
            .await?
            .into()
        };
        Self::set_item(jvm, ctx, this, position, command).await
    }
    async fn item(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, position: i32) -> Result<ClassInstanceRef<()>> {
        jvm.invoke_special(&this, BAR, "getCommand", "(I)Lorg/kwis/msp/lwc/Command;", (position,))
            .await
    }
    async fn remove(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>, position: i32) -> Result<()> {
        Self::set_item(jvm, ctx, this, position, None.into()).await
    }
    async fn listener(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<ClassInstanceRef<()>> {
        jvm.get_field_in_class(&this, BAR, "wieCommandListener", "Lorg/kwis/msp/lwc/CommandListener;")
            .await
    }
    async fn key(jvm: &Jvm, ctx: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, kind: i32, code: i32) -> Result<bool> {
        // Inferred keypad profile: the observed 0/mode, 1/confirm, 2/cancel
        // slots correspond to left soft key, centre OK, right soft key.
        let position = match code {
            x if x == WIPIKeyCode::LEFT_SOFT_KEY as i32 => 0,
            x if x == WIPIKeyCode::FIRE as i32 => 1,
            x if x == WIPIKeyCode::RIGHT_SOFT_KEY as i32 => 2,
            _ => return Ok(false),
        };
        if Self::item(jvm, ctx, this.clone(), position).await?.is_null() {
            return Ok(false);
        }
        // A physical soft-key activation is SELECT, not a focus-navigation event.
        jvm.put_field_in_class(&mut this, BAR, "wieSelected", "I", position).await?;
        jvm.invoke_special(&this, BAR, "keyNotify", "(II)Z", (kind, WIPIKeyCode::FIRE as i32))
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;
    use jvm::runtime::JavaLangString;
    use jvm_class_proto::JavaFieldProto;
    use jvm_types::FieldAccessFlags;
    async fn selected(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        mut this: ClassInstanceRef<()>,
        command: ClassInstanceRef<()>,
        kind: i32,
        object: ClassInstanceRef<()>,
    ) -> Result<()> {
        assert_eq!(kind, 2);
        let count: i32 = jvm.get_field(&this, "count", "I").await?;
        jvm.put_field(&mut this, "count", "I", count + 1).await?;
        jvm.put_field(&mut this, "last", CMD, command).await?;
        assert_eq!(this.identity(), object.identity());
        Ok(())
    }
    #[test]
    fn positional_commands_keep_gaps_and_emit_one_selection_per_soft_key() -> wie_util::Result<()> {
        let listener = WieJavaClassProto {
            name: "test/MenuListener",
            parent_class: Some("java/lang/Object"),
            interfaces: vec!["org/kwis/msp/lwc/CommandListener"],
            methods: vec![JavaMethodProto::new(
                "commandAction",
                "(Lorg/kwis/msp/lwc/Command;ILjava/lang/Object;)V",
                selected,
                MethodAccessFlags::PUBLIC,
            )],
            fields: vec![
                JavaFieldProto::new("count", "I", FieldAccessFlags::PUBLIC),
                JavaFieldProto::new("last", CMD, FieldAccessFlags::PUBLIC),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
        };
        test_utils::run_jvm_test(
            Box::new([
                wie_midp::get_protos().into(),
                wie_wipi_java::get_protos().into(),
                Box::new([GMenuBar::as_proto(), listener]),
            ]),
            |jvm| async move {
                let bar = jvm.new_class(NAME, "()V", ()).await?;
                let listener = jvm.instantiate_class("test/MenuListener").await?;
                let _: () = jvm.invoke_special(&listener, "java/lang/Object", "<init>", "()V", ()).await?;
                let _: () = jvm
                    .invoke_virtual(
                        &bar,
                        BAR,
                        "setCommandListener",
                        "(Lorg/kwis/msp/lwc/CommandListener;Ljava/lang/Object;)V",
                        (listener.clone(), listener.clone()),
                    )
                    .await?;
                for (position, text) in [(0, "mode"), (1, "confirm"), (2, "cancel")] {
                    let text = JavaLangString::from_rust_string(&jvm, text).await?;
                    let _: () = jvm
                        .invoke_virtual(&bar, NAME, "setItem", "(ILjava/lang/String;)V", (position, text))
                        .await?;
                }
                let right: ClassInstanceRef<()> = jvm.invoke_virtual(&bar, NAME, "itemAt", "(I)Lorg/kwis/msp/lwc/Command;", (2,)).await?;
                for code in [WIPIKeyCode::LEFT_SOFT_KEY, WIPIKeyCode::FIRE, WIPIKeyCode::RIGHT_SOFT_KEY] {
                    for kind in [1, 3, 2, 4] {
                        assert!(
                            jvm.invoke_virtual::<_, bool>(&bar, NAME, "keyNotify", "(II)Z", (kind, code as i32))
                                .await?
                        );
                    }
                }
                assert_eq!(jvm.get_field::<i32>(&listener, "count", "I").await?, 3);
                let last: ClassInstanceRef<()> = jvm.get_field(&listener, "last", CMD).await?;
                assert_eq!(right.identity(), last.identity());
                let _: () = jvm.invoke_virtual(&bar, NAME, "removeCommand", "(I)V", (0,)).await?;
                let still_right: ClassInstanceRef<()> = jvm.invoke_virtual(&bar, NAME, "itemAt", "(I)Lorg/kwis/msp/lwc/Command;", (2,)).await?;
                assert_eq!(right.identity(), still_right.identity());
                assert!(
                    !jvm.invoke_virtual::<_, bool>(&bar, NAME, "keyNotify", "(II)Z", (1, WIPIKeyCode::LEFT_SOFT_KEY as i32))
                        .await?
                );
                let image: ClassInstanceRef<()> = jvm
                    .invoke_static("org/kwis/msp/lcdui/Image", "createImage", "(II)Lorg/kwis/msp/lcdui/Image;", (90, 24))
                    .await?;
                let g: ClassInstanceRef<()> = jvm
                    .invoke_virtual(&image, "org/kwis/msp/lcdui/Image", "getGraphics", "()Lorg/kwis/msp/lcdui/Graphics;", ())
                    .await?;
                let _: () = jvm
                    .invoke_virtual(&bar, "org/kwis/msp/lwc/Component", "configure", "(IIIII)V", (0, 0, 90, 24, 15))
                    .await?;
                let _: () = jvm
                    .invoke_virtual(&bar, BAR, "paintContent", "(Lorg/kwis/msp/lcdui/Graphics;)V", (g,))
                    .await?;
                let _: () = jvm
                    .invoke_virtual(&bar, NAME, "keyNotify", "(II)Z", (1, WIPIKeyCode::RIGHT_SOFT_KEY as i32))
                    .await
                    .map(|_: bool| ())?;
                let _: () = jvm.invoke_virtual(&bar, NAME, "removeAllCommand", "()V", ()).await?;
                assert!(
                    !jvm.invoke_virtual::<_, bool>(&bar, NAME, "keyNotify", "(II)Z", (2, WIPIKeyCode::RIGHT_SOFT_KEY as i32))
                        .await?
                );
                assert_eq!(jvm.get_field::<i32>(&listener, "count", "I").await?, 3);
                Ok(())
            },
        )
    }
}
