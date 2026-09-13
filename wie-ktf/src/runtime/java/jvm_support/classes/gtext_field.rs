use alloc::vec;
use jvm::{ClassInstanceRef, Jvm, Result, runtime::JavaLangString};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use wie_jvm_support::{WieJavaClassProto, WieJvmContext};
use wie_wipi_java::classes::net::wie::WIPIKeyCode;
pub struct GTextField;
const NAME: &str = "com/ktf/kfc/GTextField";
const BASE: &str = "org/kwis/msp/lwc/TextFieldComponent";
const TEXT: &str = "org/kwis/msp/lwc/TextComponent";
const FORM: &str = "com/ktf/kfc/GMenubarForm";
impl GTextField {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: NAME,
            parent_class: Some(BASE),
            interfaces: vec![],
            fields: vec![
                JavaFieldProto::new("wieForm", "Lcom/ktf/kfc/GMenubarForm;", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("wieMode", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("wieIME", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("wieActionKeys", "Z", FieldAccessFlags::PRIVATE),
            ],
            methods: vec![
                JavaMethodProto::new(
                    "<init>",
                    "(Lcom/ktf/kfc/GMenubarForm;Ljava/lang/String;I)V",
                    Self::init,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("focusNotify", "(Z)V", Self::focus, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("keyNotify", "(II)Z", Self::key, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setIMEButtonPos", "(I)V", Self::set_ime, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setActionKeyB", "(Z)V", Self::set_action, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getActionKeyB", "()Z", Self::action, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getCaretPosition", "()I", Self::caret, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setCaretPosition", "(I)V", Self::set_caret, MethodAccessFlags::PUBLIC),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }
    async fn init(
        jvm: &Jvm,
        ctx: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        form: ClassInstanceRef<()>,
        text: ClassInstanceRef<()>,
        constraint: i32,
    ) -> Result<()> {
        let _: () = jvm
            .invoke_special(&this, BASE, "<init>", "(Ljava/lang/String;I)V", (text, constraint))
            .await?;
        let position = if form.is_null() {
            -1
        } else {
            jvm.invoke_virtual(&form, FORM, "getIMEButtonPos", "()I", ()).await?
        };
        jvm.put_field(&mut this, "wieForm", "Lcom/ktf/kfc/GMenubarForm;", form).await?;
        jvm.put_field(&mut this, "wieIME", "I", position).await?;
        Self::apply_mode(jvm, ctx, this, if matches!(constraint, 1 | 2 | 5) { 3 } else { 0 }, false).await
    }
    async fn apply_mode(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, mode: i32, notify: bool) -> Result<()> {
        jvm.put_field(&mut this, "wieMode", "I", mode).await?;
        jvm.put_field_in_class(&mut this, TEXT, "wieInputModeOverride", "I", if mode == 0 { 1 } else { 2 })
            .await?;
        jvm.put_field_in_class(&mut this, TEXT, "wieUppercase", "Z", mode == 1).await?;
        jvm.put_field_in_class(&mut this, TEXT, "wieNumeric", "Z", mode == 3).await?;
        jvm.put_field_in_class(&mut this, TEXT, "wieLastKey", "I", 0).await?;
        jvm.put_field_in_class(&mut this, TEXT, "wieHangulKeys", "[C", ClassInstanceRef::<()>::from(None))
            .await?;
        if notify {
            let form: ClassInstanceRef<()> = jvm.get_field(&this, "wieForm", "Lcom/ktf/kfc/GMenubarForm;").await?;
            if !form.is_null() {
                let label = JavaLangString::from_rust_string(jvm, ["KO", "EN/L", "EN/S", "N123"][mode as usize]).await?;
                let _: () = jvm
                    .invoke_virtual(
                        &form,
                        FORM,
                        "inputModeChanged",
                        "(Ljava/lang/String;Lorg/kwis/msp/lwc/TextComponent;)V",
                        (label, this.clone()),
                    )
                    .await?;
            }
        }
        let _: () = jvm.invoke_virtual(&this, TEXT, "repaint", "()V", ()).await?;
        Ok(())
    }
    pub async fn cycle(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<()> {
        let constraint: i32 = jvm.invoke_virtual(&this, TEXT, "getConstraint", "()I", ()).await?;
        let mode: i32 = jvm.get_field(&this, "wieMode", "I").await?;
        Self::apply_mode(jvm, ctx, this, if matches!(constraint, 1 | 2 | 5) { 3 } else { (mode + 1) % 4 }, true).await
    }
    async fn focus(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>, focused: bool) -> Result<()> {
        let _: () = jvm
            .invoke_special(&this, "org/kwis/msp/lwc/Component", "focusNotify", "(Z)V", (focused,))
            .await?;
        if focused {
            let constraint: i32 = jvm.invoke_virtual(&this, TEXT, "getConstraint", "()I", ()).await?;
            Self::apply_mode(jvm, ctx, this, if matches!(constraint, 1 | 2 | 5) { 3 } else { 0 }, true).await?;
        } else {
            let mut form: ClassInstanceRef<()> = jvm.get_field(&this, "wieForm", "Lcom/ktf/kfc/GMenubarForm;").await?;
            if !form.is_null() {
                let previous: ClassInstanceRef<()> = jvm
                    .get_field_in_class(&form, FORM, "m_cmpfocused", "Lorg/kwis/msp/lwc/Component;")
                    .await?;
                if !previous.is_null() && previous.identity() == this.identity() {
                    jvm.put_field_in_class(
                        &mut form,
                        FORM,
                        "m_cmpfocused",
                        "Lorg/kwis/msp/lwc/Component;",
                        ClassInstanceRef::<()>::from(None),
                    )
                    .await?;
                }
            }
        }
        Ok(())
    }
    async fn set_ime(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, position: i32) -> Result<()> {
        jvm.put_field(&mut this, "wieIME", "I", position).await
    }
    async fn set_action(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, value: bool) -> Result<()> {
        jvm.put_field(&mut this, "wieActionKeys", "Z", value).await
    }
    async fn action(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<bool> {
        jvm.get_field(&this, "wieActionKeys", "Z").await
    }
    async fn caret(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<i32> {
        jvm.get_field_in_class(&this, TEXT, "m_cPos", "I").await
    }
    async fn set_caret(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, position: i32) -> Result<()> {
        let value: ClassInstanceRef<()> = jvm.invoke_virtual(&this, TEXT, "getString", "()Ljava/lang/String;", ()).await?;
        let size: i32 = jvm.invoke_virtual(&value, "java/lang/String", "length", "()I", ()).await?;
        jvm.put_field_in_class(&mut this, TEXT, "m_cPos", "I", position.clamp(0, size)).await
    }
    async fn key(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>, kind: i32, code: i32) -> Result<bool> {
        let slot = match code {
            x if x == WIPIKeyCode::LEFT_SOFT_KEY as i32 => 0,
            x if x == WIPIKeyCode::FIRE as i32 => 1,
            x if x == WIPIKeyCode::RIGHT_SOFT_KEY as i32 => 2,
            _ => -1,
        };
        if slot >= 0 {
            let ime: i32 = jvm.get_field(&this, "wieIME", "I").await?;
            if slot == ime {
                if kind == 1 {
                    Self::cycle(jvm, ctx, this).await?;
                }
                return Ok(true);
            }
            if !Self::action(jvm, ctx, this.clone()).await? {
                return Ok(false);
            }
        }
        jvm.invoke_special(&this, TEXT, "keyNotify", "(II)Z", (kind, code)).await
    }
}
