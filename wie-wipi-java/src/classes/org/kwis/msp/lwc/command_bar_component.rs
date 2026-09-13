use crate::classes::{net::wie::WIPIKeyCode, org::kwis::msp::lwc::CommandListener};
use alloc::{vec, vec::Vec};
use jvm::{Array, ClassInstanceRef, Jvm, Result};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

pub struct CommandBarComponent;
const NAME: &str = "org/kwis/msp/lwc/CommandBarComponent";
const COMPONENT: &str = "org/kwis/msp/lwc/Component";
const COMMAND: &str = "org/kwis/msp/lwc/Command";
const CMD: &str = "Lorg/kwis/msp/lwc/Command;";
const CMDS: &str = "[Lorg/kwis/msp/lwc/Command;";
const LISTENER: &str = "org/kwis/msp/lwc/CommandListener";
const FONT: &str = "org/kwis/msp/lcdui/Font";
const GRAPHICS: &str = "org/kwis/msp/lcdui/Graphics";
type Commands = ClassInstanceRef<Array<ClassInstanceRef<()>>>;
impl CommandBarComponent {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: NAME,
            parent_class: Some(COMPONENT),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("<init>", "()V", Self::init, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("addCommand", "(Lorg/kwis/msp/lwc/Command;)I", Self::add, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("removeCommand", "(Lorg/kwis/msp/lwc/Command;)V", Self::remove, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("removeAll", "()V", Self::remove_all, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getSize", "()I", Self::size, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getCommand", "(I)Lorg/kwis/msp/lwc/Command;", Self::command, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getActiveIndex", "()I", Self::active, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setActiveIndex", "(I)V", Self::set_active, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "setCommandListener",
                    "(Lorg/kwis/msp/lwc/CommandListener;Ljava/lang/Object;)V",
                    Self::listener,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("getPreferredWidth", "()I", Self::width, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getPreferredHeight", "()I", Self::height, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getPreferredHeight", "(I)I", Self::height_for_width, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("keyNotify", "(II)Z", Self::key, MethodAccessFlags::PROTECTED),
                JavaMethodProto::new("focusNotify", "(Z)V", Self::focus, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("paintContent", "(Lorg/kwis/msp/lcdui/Graphics;)V", Self::paint, MethodAccessFlags::PUBLIC),
            ],
            fields: vec![
                JavaFieldProto::new("wieCommands", CMDS, FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("wieSelected", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("wieCommandListener", "Lorg/kwis/msp/lwc/CommandListener;", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("wieListenerObject", "Ljava/lang/Object;", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("wieCommandFont", "Lorg/kwis/msp/lcdui/Font;", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("wiePressedCommand", CMD, FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("wieSuppressTyped", "Z", FieldAccessFlags::PRIVATE),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }
    async fn init(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>) -> Result<()> {
        let _: () = jvm.invoke_special(&this, COMPONENT, "<init>", "()V", ()).await?;
        jvm.put_field(&mut this, "wieInput", "Z", true).await?;
        let commands = jvm.instantiate_array(CMD, 0).await?;
        jvm.put_field(&mut this, "wieCommands", CMDS, commands).await?;
        jvm.put_field(&mut this, "wieSelected", "I", -1).await?;
        let font: ClassInstanceRef<()> = jvm.invoke_static(FONT, "getDefaultFont", "()Lorg/kwis/msp/lcdui/Font;", ()).await?;
        jvm.put_field(&mut this, "wieCommandFont", "Lorg/kwis/msp/lcdui/Font;", font).await
    }
    async fn entries(jvm: &Jvm, this: &ClassInstanceRef<Self>) -> Result<Vec<ClassInstanceRef<()>>> {
        let array: Commands = jvm.get_field(this, "wieCommands", CMDS).await?;
        jvm.load_array(&array, 0, jvm.array_length(&array).await?).await
    }
    async fn store(jvm: &Jvm, mut this: ClassInstanceRef<Self>, entries: Vec<ClassInstanceRef<()>>) -> Result<()> {
        let mut array = jvm.instantiate_array(CMD, entries.len()).await?;
        jvm.store_array(&mut array, 0, entries).await?;
        jvm.put_field(&mut this, "wieCommands", CMDS, array).await?;
        let _: () = jvm.invoke_virtual(&this, COMPONENT, "invalidate", "()V", ()).await?;
        jvm.invoke_virtual(&this, COMPONENT, "repaint", "()V", ()).await
    }
    async fn size(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<i32> {
        let array: Commands = jvm.get_field(&this, "wieCommands", CMDS).await?;
        Ok(jvm.array_length(&array).await? as i32)
    }
    async fn command(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>, index: i32) -> Result<ClassInstanceRef<()>> {
        if index < 0 || index >= Self::size(jvm, ctx, this.clone()).await? {
            return Ok(None.into());
        }
        let array: Commands = jvm.get_field(&this, "wieCommands", CMDS).await?;
        Ok(jvm.load_array(&array, index as usize, 1).await?.remove(0))
    }
    async fn add(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, command: ClassInstanceRef<()>) -> Result<i32> {
        if command.is_null() {
            return Ok(-1);
        }
        let mut entries = Self::entries(jvm, &this).await?;
        let index = entries.len() as i32;
        entries.push(command);
        Self::store(jvm, this, entries).await?;
        Ok(index)
    }
    async fn remove(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, command: ClassInstanceRef<()>) -> Result<()> {
        if command.is_null() {
            return Ok(());
        }
        let mut entries = Self::entries(jvm, &this).await?;
        if let Some(index) = entries.iter().position(|c| !c.is_null() && c.identity() == command.identity()) {
            entries.remove(index);
            let selected: i32 = jvm.get_field(&this, "wieSelected", "I").await?;
            let next = if selected == index as i32 {
                -1
            } else if selected > index as i32 {
                selected - 1
            } else {
                selected
            };
            jvm.put_field(&mut this, "wieSelected", "I", next).await?;
            jvm.put_field(&mut this, "wiePressedCommand", CMD, ClassInstanceRef::<()>::new(None))
                .await?;
            Self::store(jvm, this, entries).await?;
        }
        Ok(())
    }
    async fn remove_all(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>) -> Result<()> {
        jvm.put_field(&mut this, "wieSelected", "I", -1).await?;
        jvm.put_field(&mut this, "wiePressedCommand", CMD, ClassInstanceRef::<()>::new(None))
            .await?;
        Self::store(jvm, this, vec![]).await
    }
    async fn active(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<i32> {
        jvm.get_field(&this, "wieSelected", "I").await
    }
    async fn set_active(jvm: &Jvm, ctx: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, index: i32) -> Result<()> {
        let index = if index >= 0 && index < Self::size(jvm, ctx, this.clone()).await? {
            index
        } else {
            -1
        };
        if Self::active(jvm, ctx, this.clone()).await? == index {
            return Ok(());
        }
        jvm.put_field(&mut this, "wieSelected", "I", index).await?;
        jvm.put_field(&mut this, "wiePressedCommand", CMD, ClassInstanceRef::<()>::new(None))
            .await?;
        let _: () = jvm.invoke_virtual(&this, COMPONENT, "repaint", "()V", ()).await?;
        if index >= 0 {
            let command = Self::command(jvm, ctx, this.clone(), index).await?;
            Self::notify(jvm, this, command, CommandListener::FOCUS_CHANGE).await?;
        }
        Ok(())
    }
    async fn listener(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        listener: ClassInstanceRef<()>,
        object: ClassInstanceRef<()>,
    ) -> Result<()> {
        jvm.put_field(&mut this, "wieCommandListener", "Lorg/kwis/msp/lwc/CommandListener;", listener)
            .await?;
        jvm.put_field(&mut this, "wieListenerObject", "Ljava/lang/Object;", object).await
    }
    async fn notify(jvm: &Jvm, this: ClassInstanceRef<Self>, command: ClassInstanceRef<()>, kind: i32) -> Result<()> {
        let listener: ClassInstanceRef<()> = jvm.get_field(&this, "wieCommandListener", "Lorg/kwis/msp/lwc/CommandListener;").await?;
        if !listener.is_null() && !command.is_null() {
            let object: ClassInstanceRef<()> = jvm.get_field(&this, "wieListenerObject", "Ljava/lang/Object;").await?;
            let _: () = jvm
                .invoke_virtual(
                    &listener,
                    LISTENER,
                    "commandAction",
                    "(Lorg/kwis/msp/lwc/Command;ILjava/lang/Object;)V",
                    (command, kind, object),
                )
                .await?;
        }
        Ok(())
    }
    async fn key(jvm: &Jvm, ctx: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, kind: i32, code: i32) -> Result<bool> {
        let size = Self::size(jvm, ctx, this.clone()).await?;
        if size == 0 {
            return Ok(false);
        }
        let selected = Self::active(jvm, ctx, this.clone()).await?;
        if code == WIPIKeyCode::LEFT as i32 || code == WIPIKeyCode::RIGHT as i32 {
            if kind == 1 || kind == 3 {
                let next = if selected < 0 {
                    0
                } else if code == WIPIKeyCode::LEFT as i32 {
                    (selected - 1).max(0)
                } else {
                    (selected + 1).min(size - 1)
                };
                Self::set_active(jvm, ctx, this, next).await?;
            }
            return Ok(true);
        }
        if code != WIPIKeyCode::FIRE as i32 || selected < 0 {
            return Ok(false);
        }
        let command = Self::command(jvm, ctx, this.clone(), selected).await?;
        match kind {
            1 => {
                jvm.put_field(&mut this, "wiePressedCommand", CMD, command).await?;
                jvm.put_field(&mut this, "wieSuppressTyped", "Z", false).await?;
            }
            2 => {
                let held: ClassInstanceRef<()> = jvm.get_field(&this, "wiePressedCommand", CMD).await?;
                jvm.put_field(&mut this, "wiePressedCommand", CMD, ClassInstanceRef::<()>::new(None))
                    .await?;
                if !held.is_null() && held.identity() == command.identity() {
                    jvm.put_field(&mut this, "wieSuppressTyped", "Z", true).await?;
                    Self::notify(jvm, this, command, CommandListener::SELECT).await?;
                }
            }
            4 => {
                let suppressed: bool = jvm.get_field(&this, "wieSuppressTyped", "Z").await?;
                jvm.put_field(&mut this, "wieSuppressTyped", "Z", false).await?;
                if !suppressed {
                    Self::notify(jvm, this, command, CommandListener::SELECT).await?;
                }
            }
            _ => {}
        }
        Ok(true)
    }
    async fn focus(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, focused: bool) -> Result<()> {
        if !focused {
            jvm.put_field(&mut this, "wiePressedCommand", CMD, ClassInstanceRef::<()>::new(None))
                .await?;
            jvm.put_field(&mut this, "wieSuppressTyped", "Z", false).await?;
        }
        jvm.invoke_special(&this, COMPONENT, "focusNotify", "(Z)V", (focused,)).await
    }
    async fn height(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<i32> {
        Self::height_for_width(jvm, ctx, this, -1).await
    }
    async fn height_for_width(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, _: i32) -> Result<i32> {
        let font: ClassInstanceRef<()> = jvm.get_field(&this, "wieCommandFont", "Lorg/kwis/msp/lcdui/Font;").await?;
        let height: i32 = jvm.invoke_virtual(&font, FONT, "getHeight", "()I", ()).await?;
        Ok(height.max(20) + 4)
    }
    async fn width(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<i32> {
        let font: ClassInstanceRef<()> = jvm.get_field(&this, "wieCommandFont", "Lorg/kwis/msp/lcdui/Font;").await?;
        let mut width = 0i32;
        for command in Self::entries(jvm, &this).await? {
            if command.is_null() {
                continue;
            }
            let text: ClassInstanceRef<()> = jvm.invoke_virtual(&command, COMMAND, "getString", "()Ljava/lang/String;", ()).await?;
            let text_width = if text.is_null() {
                0
            } else {
                jvm.invoke_virtual::<_, i32>(&font, FONT, "stringWidth", "(Ljava/lang/String;)I", (text,))
                    .await?
            };
            width = width.saturating_add(text_width.max(20).saturating_add(8));
        }
        Ok(width)
    }
    async fn paint(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, g: ClassInstanceRef<()>) -> Result<()> {
        let _: () = jvm
            .invoke_special(&this, COMPONENT, "paintContent", "(Lorg/kwis/msp/lcdui/Graphics;)V", (g.clone(),))
            .await?;
        let commands = Self::entries(jvm, &this).await?;
        if commands.is_empty() {
            return Ok(());
        }
        let selected: i32 = jvm.get_field(&this, "wieSelected", "I").await?;
        let width: i32 = jvm.get_field(&this, "w", "I").await?;
        let height: i32 = jvm.get_field(&this, "h", "I").await?;
        let font: ClassInstanceRef<()> = jvm.get_field(&this, "wieCommandFont", "Lorg/kwis/msp/lcdui/Font;").await?;
        let _: () = jvm
            .invoke_virtual(&g, GRAPHICS, "setFont", "(Lorg/kwis/msp/lcdui/Font;)V", (font,))
            .await?;
        let mut clip = [0; 4];
        for (value, method) in clip.iter_mut().zip(["getClipX", "getClipY", "getClipWidth", "getClipHeight"]) {
            *value = jvm.invoke_virtual(&g, GRAPHICS, method, "()I", ()).await?;
        }
        let count = commands.len() as i64;
        for (index, command) in commands.into_iter().enumerate() {
            let x = (width.max(0) as i64 * index as i64 / count) as i32;
            let right = (width.max(0) as i64 * (index + 1) as i64 / count) as i32;
            let result: Result<()> = async {
                let _: () = jvm.invoke_virtual(&g, GRAPHICS, "clipRect", "(IIII)V", (x, 0, right - x, height)).await?;
                let active = selected == index as i32;
                let _: () = jvm
                    .invoke_virtual(&g, GRAPHICS, "setColor", "(I)V", (if active { 0x334466 } else { 0xeeeeee },))
                    .await?;
                let _: () = jvm.invoke_virtual(&g, GRAPHICS, "fillRect", "(IIII)V", (x, 0, right - x, height)).await?;
                if command.is_null() {
                    return Ok(());
                }
                let _: () = jvm
                    .invoke_virtual(&g, GRAPHICS, "setColor", "(I)V", (if active { 0xffffff } else { 0x111111 },))
                    .await?;
                let text: ClassInstanceRef<()> = jvm.invoke_virtual(&command, COMMAND, "getString", "()Ljava/lang/String;", ()).await?;
                let image: ClassInstanceRef<()> = jvm
                    .invoke_virtual(
                        &command,
                        COMMAND,
                        if active { "getActiveImage" } else { "getNormalImage" },
                        "()Lorg/kwis/msp/lcdui/Image;",
                        (),
                    )
                    .await?;
                if !image.is_null() {
                    let _: () = jvm
                        .invoke_virtual(&g, GRAPHICS, "drawImage", "(Lorg/kwis/msp/lcdui/Image;III)V", (image, x + 2, 2, 0))
                        .await?;
                } else if !text.is_null() {
                    let _: () = jvm
                        .invoke_virtual(&g, GRAPHICS, "drawString", "(Ljava/lang/String;III)V", (text, x + 2, 2, 0))
                        .await?;
                }
                Ok(())
            }
            .await;
            let _: () = jvm
                .invoke_virtual(&g, GRAPHICS, "setClip", "(IIII)V", (clip[0], clip[1], clip[2], clip[3]))
                .await?;
            result?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;
    use test_utils::run_jvm_test;
    async fn event(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        mut this: ClassInstanceRef<()>,
        command: ClassInstanceRef<()>,
        kind: i32,
        object: ClassInstanceRef<()>,
    ) -> Result<()> {
        let count: i32 = jvm.get_field(&this, "calls", "I").await?;
        jvm.put_field(&mut this, "calls", "I", count + 1).await?;
        jvm.put_field(&mut this, "kind", "I", kind).await?;
        jvm.put_field(&mut this, "command", CMD, command).await?;
        jvm.put_field(&mut this, "object", "Ljava/lang/Object;", object.clone()).await?;
        if kind == CommandListener::SELECT && jvm.get_field::<bool>(&this, "clearOnSelect", "Z").await? {
            let _: () = jvm.invoke_virtual(&object, NAME, "removeAll", "()V", ()).await?;
        }
        if jvm.get_field::<bool>(&this, "fail", "Z").await? {
            return Err(jvm.exception("java/lang/IllegalStateException", "listener failure").await);
        }
        Ok(())
    }
    fn protos() -> Box<[WieJavaClassProto]> {
        let mut protos = crate::get_protos().into_iter().collect::<Vec<_>>();
        protos.push(WieJavaClassProto {
            name: "test/Commands",
            parent_class: Some("java/lang/Object"),
            interfaces: vec![LISTENER],
            methods: vec![JavaMethodProto::new(
                "commandAction",
                "(Lorg/kwis/msp/lwc/Command;ILjava/lang/Object;)V",
                event,
                MethodAccessFlags::PUBLIC,
            )],
            fields: vec![
                JavaFieldProto::new("calls", "I", FieldAccessFlags::PUBLIC),
                JavaFieldProto::new("kind", "I", FieldAccessFlags::PUBLIC),
                JavaFieldProto::new("command", CMD, FieldAccessFlags::PUBLIC),
                JavaFieldProto::new("object", "Ljava/lang/Object;", FieldAccessFlags::PUBLIC),
                JavaFieldProto::new("clearOnSelect", "Z", FieldAccessFlags::PUBLIC),
                JavaFieldProto::new("fail", "Z", FieldAccessFlags::PUBLIC),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
        });
        protos.into_boxed_slice()
    }
    async fn new_command(jvm: &Jvm) -> Result<ClassInstanceRef<()>> {
        Ok(jvm
            .new_class(
                COMMAND,
                "(Ljava/lang/String;Ljava/lang/Object;)V",
                (ClassInstanceRef::<()>::new(None), ClassInstanceRef::<()>::new(None)),
            )
            .await?
            .into())
    }
    #[test]
    fn command_storage_callbacks_and_key_cycles_survive_reentrant_removal() -> wie_util::Result<()> {
        run_jvm_test(Box::new([wie_midp::get_protos().into(), protos()]), |jvm| async move {
            let bar = jvm.new_class(NAME, "()V", ()).await?;
            let mut listener = jvm.instantiate_class("test/Commands").await?;
            let _: () = jvm.invoke_special(&listener, "java/lang/Object", "<init>", "()V", ()).await?;
            let payload = jvm.new_class("java/lang/Object", "()V", ()).await?;
            let _: () = jvm
                .invoke_virtual(
                    &bar,
                    NAME,
                    "setCommandListener",
                    "(Lorg/kwis/msp/lwc/CommandListener;Ljava/lang/Object;)V",
                    (listener.clone(), payload.clone()),
                )
                .await?;
            assert_eq!(jvm.invoke_virtual::<_, i32>(&bar, NAME, "getActiveIndex", "()I", ()).await?, -1);
            assert_eq!(
                jvm.invoke_virtual::<_, i32>(
                    &bar,
                    NAME,
                    "addCommand",
                    "(Lorg/kwis/msp/lwc/Command;)I",
                    (ClassInstanceRef::<()>::new(None),)
                )
                .await?,
                -1
            );
            let a = new_command(&jvm).await?;
            let b = new_command(&jvm).await?;
            for (command, index) in [(&a, 0), (&b, 1)] {
                assert_eq!(
                    jvm.invoke_virtual::<_, i32>(&bar, NAME, "addCommand", "(Lorg/kwis/msp/lwc/Command;)I", (command.clone(),))
                        .await?,
                    index
                );
            }
            for index in [-1, 2, i32::MAX] {
                let c: ClassInstanceRef<()> = jvm
                    .invoke_virtual(&bar, NAME, "getCommand", "(I)Lorg/kwis/msp/lwc/Command;", (index,))
                    .await?;
                assert!(c.is_null());
            }
            let _: () = jvm.invoke_virtual(&bar, NAME, "setActiveIndex", "(I)V", (1,)).await?;
            let _: () = jvm.invoke_virtual(&bar, NAME, "setActiveIndex", "(I)V", (1,)).await?;
            assert_eq!(jvm.get_field::<i32>(&listener, "calls", "I").await?, 1);
            assert_eq!(jvm.get_field::<i32>(&listener, "kind", "I").await?, 1);
            let actual: ClassInstanceRef<()> = jvm.get_field(&listener, "command", CMD).await?;
            assert_eq!(actual.identity(), b.identity());
            let actual: ClassInstanceRef<()> = jvm.get_field(&listener, "object", "Ljava/lang/Object;").await?;
            assert_eq!(actual.identity(), payload.identity());
            for kind in [1, 3, 2, 2, 4] {
                let _: bool = jvm
                    .invoke_virtual(&bar, NAME, "keyNotify", "(II)Z", (kind, WIPIKeyCode::FIRE as i32))
                    .await?;
            }
            assert_eq!(jvm.get_field::<i32>(&listener, "calls", "I").await?, 2);
            assert_eq!(jvm.get_field::<i32>(&listener, "kind", "I").await?, 2);
            let _: () = jvm
                .invoke_virtual(&bar, NAME, "removeCommand", "(Lorg/kwis/msp/lwc/Command;)V", (a,))
                .await?;
            assert_eq!(jvm.invoke_virtual::<_, i32>(&bar, NAME, "getActiveIndex", "()I", ()).await?, 0);
            let _: () = jvm
                .invoke_virtual(
                    &bar,
                    NAME,
                    "setCommandListener",
                    "(Lorg/kwis/msp/lwc/CommandListener;Ljava/lang/Object;)V",
                    (listener.clone(), bar.clone()),
                )
                .await?;
            jvm.put_field(&mut listener, "clearOnSelect", "Z", true).await?;
            for kind in [1, 2] {
                let _: bool = jvm
                    .invoke_virtual(&bar, NAME, "keyNotify", "(II)Z", (kind, WIPIKeyCode::FIRE as i32))
                    .await?;
            }
            assert_eq!(jvm.invoke_virtual::<_, i32>(&bar, NAME, "getSize", "()I", ()).await?, 0);
            assert_eq!(jvm.invoke_virtual::<_, i32>(&bar, NAME, "getActiveIndex", "()I", ()).await?, -1);
            assert_eq!(jvm.get_field::<i32>(&listener, "calls", "I").await?, 3);
            let _: i32 = jvm
                .invoke_virtual(&bar, NAME, "addCommand", "(Lorg/kwis/msp/lwc/Command;)I", (b,))
                .await?;
            jvm.put_field(&mut listener, "fail", "Z", true).await?;
            assert!(jvm.invoke_virtual::<_, ()>(&bar, NAME, "setActiveIndex", "(I)V", (0,)).await.is_err());
            assert_eq!(jvm.invoke_virtual::<_, i32>(&bar, NAME, "getActiveIndex", "()I", ()).await?, 0);
            Ok(())
        })
    }
    #[test]
    fn selected_and_unselected_cells_render_and_restore_clip() -> wie_util::Result<()> {
        run_jvm_test(Box::new([wie_midp::get_protos().into(), protos()]), |jvm| async move {
            let mut bar = jvm.new_class(NAME, "()V", ()).await?;
            jvm.put_field(&mut bar, "w", "I", 40).await?;
            jvm.put_field(&mut bar, "h", "I", 24).await?;
            for _ in 0..2 {
                let c = new_command(&jvm).await?;
                let _: i32 = jvm
                    .invoke_virtual(&bar, NAME, "addCommand", "(Lorg/kwis/msp/lwc/Command;)I", (c,))
                    .await?;
            }
            let _: () = jvm.invoke_virtual(&bar, NAME, "setActiveIndex", "(I)V", (0,)).await?;
            let image: ClassInstanceRef<()> = jvm
                .invoke_static("org/kwis/msp/lcdui/Image", "createImage", "(II)Lorg/kwis/msp/lcdui/Image;", (40, 24))
                .await?;
            let g: ClassInstanceRef<()> = jvm
                .invoke_virtual(&image, "org/kwis/msp/lcdui/Image", "getGraphics", "()Lorg/kwis/msp/lcdui/Graphics;", ())
                .await?;
            let _: () = jvm
                .invoke_virtual(&bar, NAME, "paintContent", "(Lorg/kwis/msp/lcdui/Graphics;)V", (g.clone(),))
                .await?;
            assert_eq!(jvm.invoke_virtual::<_, i32>(&g, GRAPHICS, "getPixel", "(II)I", (0, 0)).await?, 0x334466);
            assert_eq!(jvm.invoke_virtual::<_, i32>(&g, GRAPHICS, "getPixel", "(II)I", (39, 0)).await?, 0xeeeeee);
            assert_eq!(jvm.invoke_virtual::<_, i32>(&g, GRAPHICS, "getClipWidth", "()I", ()).await?, 40);
            assert_eq!(jvm.invoke_virtual::<_, i32>(&g, GRAPHICS, "getClipHeight", "()I", ()).await?, 24);
            Ok(())
        })
    }
}
