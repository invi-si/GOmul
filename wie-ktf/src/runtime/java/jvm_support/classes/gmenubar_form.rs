use alloc::vec;
use jvm::{ClassInstanceRef, Jvm, Result};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

pub struct GMenubarForm;
pub struct GTextModeListener;
const NAME: &str = "com/ktf/kfc/GMenubarForm";
const BASE: &str = "com/ktf/kfc/GFormBase";
const MENU: &str = "com/ktf/kfc/GMenuBar";
const MENU_REF: &str = "Lcom/ktf/kfc/GMenuBar;";
const COMPONENT: &str = "Lorg/kwis/msp/lwc/Component;";
impl GTextModeListener {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "com/ktf/kfc/GTextModeListener",
            parent_class: Some("java/lang/Object"),
            interfaces: vec![],
            fields: vec![],
            methods: vec![JavaMethodProto::new_abstract(
                "inputModeChanged",
                "(Ljava/lang/String;Lorg/kwis/msp/lwc/TextComponent;)V",
                MethodAccessFlags::PUBLIC | MethodAccessFlags::ABSTRACT,
            )],
            access_flags: ClassAccessFlags::PUBLIC | ClassAccessFlags::INTERFACE | ClassAccessFlags::ABSTRACT,
        }
    }
}
impl GMenubarForm {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: NAME,
            parent_class: Some(BASE),
            interfaces: vec!["org/kwis/msp/lwc/CommandListener", "com/ktf/kfc/GTextModeListener"],
            methods: vec![
                JavaMethodProto::new("keyNotify", "(II)Z", Self::key, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("<init>", "()V", Self::init, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("<init>", "(Ljava/lang/String;)V", Self::init_title, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "<init>",
                    "(Ljava/lang/String;Lorg/kwis/msp/lcdui/Image;)V",
                    Self::init_image,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new(
                    "<init>",
                    "(Ljava/lang/String;Ljava/lang/String;)V",
                    Self::init_path,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("getGMenuBar", "()Lcom/ktf/kfc/GMenuBar;", Self::menu, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getBaseGMenuBar", "()Lcom/ktf/kfc/GMenuBar;", Self::menu, MethodAccessFlags::PROTECTED),
                JavaMethodProto::new(
                    "setMenuBar",
                    "(Lcom/ktf/kfc/GMenuBar;)V",
                    Self::set_menu_default,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("setMenuBar", "(Lcom/ktf/kfc/GMenuBar;Z)V", Self::set_menu, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("initializeUI", "()V", Self::initialize, MethodAccessFlags::PROTECTED),
                JavaMethodProto::new("setIMEButtonPos", "(I)V", Self::set_ime, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getIMEButtonPos", "()I", Self::ime, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "inputModeChanged",
                    "(Ljava/lang/String;Lorg/kwis/msp/lwc/TextComponent;)V",
                    Self::mode_changed,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new(
                    "commandAction",
                    "(Lorg/kwis/msp/lwc/Command;ILjava/lang/Object;)V",
                    Self::command,
                    MethodAccessFlags::PUBLIC,
                ),
            ],
            fields: vec![
                JavaFieldProto::new("m_menubar", MENU_REF, FieldAccessFlags::PUBLIC),
                JavaFieldProto::new("m_cmpfocused", COMPONENT, FieldAccessFlags::PROTECTED),
                JavaFieldProto::new("wieIMEPosition", "I", FieldAccessFlags::PRIVATE),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }
    async fn finish(jvm: &Jvm, ctx: &mut WieJvmContext, mut this: ClassInstanceRef<Self>) -> Result<()> {
        jvm.put_field_in_class(&mut this, NAME, "wieIMEPosition", "I", -1).await?;
        let menu = jvm.new_class(MENU, "()V", ()).await?;
        Self::set_menu(jvm, ctx, this, menu.into(), true).await
    }
    async fn key(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>, kind: i32, code: i32) -> Result<bool> {
        let focused: ClassInstanceRef<()> = jvm.get_field_in_class(&this, NAME, "m_cmpfocused", COMPONENT).await?;
        let position = Self::ime(jvm, ctx, this.clone()).await?;
        let keys = [-6, -5, -7];
        if (0..3).contains(&position) && code == keys[position as usize] && !focused.is_null() {
            let handled: bool = jvm
                .invoke_virtual(&focused, "org/kwis/msp/lwc/TextComponent", "keyNotify", "(II)Z", (kind, code))
                .await?;
            if handled {
                return Ok(true);
            }
        }
        jvm.invoke_special(&this, BASE, "keyNotify", "(II)Z", (kind, code)).await
    }
    async fn init(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<()> {
        let _: () = jvm.invoke_special(&this, BASE, "<init>", "()V", ()).await?;
        Self::finish(jvm, ctx, this).await
    }
    async fn init_title(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>, title: ClassInstanceRef<()>) -> Result<()> {
        let _: () = jvm.invoke_special(&this, BASE, "<init>", "(Ljava/lang/String;)V", (title,)).await?;
        Self::finish(jvm, ctx, this).await
    }
    async fn init_image(
        jvm: &Jvm,
        ctx: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        title: ClassInstanceRef<()>,
        image: ClassInstanceRef<()>,
    ) -> Result<()> {
        let _: () = jvm
            .invoke_special(&this, BASE, "<init>", "(Ljava/lang/String;Lorg/kwis/msp/lcdui/Image;)V", (title, image))
            .await?;
        Self::finish(jvm, ctx, this).await
    }
    async fn init_path(
        jvm: &Jvm,
        ctx: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        title: ClassInstanceRef<()>,
        path: ClassInstanceRef<()>,
    ) -> Result<()> {
        let _: () = jvm
            .invoke_special(&this, BASE, "<init>", "(Ljava/lang/String;Ljava/lang/String;)V", (title, path))
            .await?;
        Self::finish(jvm, ctx, this).await
    }
    async fn menu(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<ClassInstanceRef<()>> {
        jvm.get_field_in_class(&this, NAME, "m_menubar", MENU_REF).await
    }
    async fn set_menu_default(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>, menu: ClassInstanceRef<()>) -> Result<()> {
        Self::set_menu(jvm, ctx, this, menu, true).await
    }
    async fn set_menu(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, menu: ClassInstanceRef<()>, grab: bool) -> Result<()> {
        let _: () = jvm
            .invoke_special(
                &this,
                "org/kwis/msp/lwc/ShellComponent",
                "setCommand",
                "(Lorg/kwis/msp/lwc/Component;Z)V",
                (menu.clone(), grab),
            )
            .await?;
        jvm.put_field_in_class(&mut this, NAME, "m_menubar", MENU_REF, menu.clone()).await?;
        if !menu.is_null() {
            let listener: ClassInstanceRef<()> = jvm
                .invoke_virtual(&menu, MENU, "getCommandListener", "()Lorg/kwis/msp/lwc/CommandListener;", ())
                .await?;
            if listener.is_null() {
                let _: () = jvm
                    .invoke_virtual(
                        &menu,
                        MENU,
                        "setCommandListener",
                        "(Lorg/kwis/msp/lwc/CommandListener;Ljava/lang/Object;)V",
                        (this.clone(), this),
                    )
                    .await?;
            }
        }
        Ok(())
    }
    async fn initialize(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<()> {
        let _: () = jvm.invoke_special(&this, BASE, "initializeUI", "()V", ()).await?;
        if Self::menu(jvm, ctx, this.clone()).await?.is_null() {
            let menu = jvm.new_class(MENU, "()V", ()).await?;
            Self::set_menu(jvm, ctx, this, menu.into(), true).await?;
        }
        Ok(())
    }
    async fn set_ime(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, position: i32) -> Result<()> {
        if !(-1..3).contains(&position) {
            return Err(jvm.exception("java/lang/IllegalArgumentException", "IME position").await);
        }
        jvm.put_field_in_class(&mut this, NAME, "wieIMEPosition", "I", position).await
    }
    async fn ime(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<i32> {
        jvm.get_field_in_class(&this, NAME, "wieIMEPosition", "I").await
    }
    async fn mode_changed(
        jvm: &Jvm,
        ctx: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        mode: ClassInstanceRef<()>,
        text: ClassInstanceRef<()>,
    ) -> Result<()> {
        jvm.put_field_in_class(&mut this, NAME, "m_cmpfocused", COMPONENT, text).await?;
        let position = Self::ime(jvm, ctx, this.clone()).await?;
        let menu = Self::menu(jvm, ctx, this).await?;
        if position >= 0 && !menu.is_null() {
            let _: () = jvm
                .invoke_virtual(&menu, MENU, "setItem", "(ILjava/lang/String;)V", (position, mode))
                .await?;
        }
        Ok(())
    }
    async fn command(
        jvm: &Jvm,
        ctx: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        command: ClassInstanceRef<()>,
        kind: i32,
        _: ClassInstanceRef<()>,
    ) -> Result<()> {
        if kind != 2 || command.is_null() {
            return Ok(());
        }
        let position = Self::ime(jvm, ctx, this.clone()).await?;
        let menu = Self::menu(jvm, ctx, this.clone()).await?;
        if position < 0 || menu.is_null() {
            return Ok(());
        }
        let mode_command: ClassInstanceRef<()> = jvm
            .invoke_virtual(&menu, MENU, "itemAt", "(I)Lorg/kwis/msp/lwc/Command;", (position,))
            .await?;
        if !mode_command.is_null() && mode_command.identity() == command.identity() {
            let focused: ClassInstanceRef<()> = jvm.get_field_in_class(&this, NAME, "m_cmpfocused", COMPONENT).await?;
            if !focused.is_null() && jvm.is_instance(&**focused, "com/ktf/kfc/GTextField") {
                return super::gtext_field::GTextField::cycle(jvm, ctx, jvm::JavaValue::from(focused).into()).await;
            }
        }
        Ok(())
    }
}
