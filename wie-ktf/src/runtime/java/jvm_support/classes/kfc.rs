use alloc::vec;
use jvm::{Array, ClassInstanceRef, Jvm, Result, runtime::JavaLangString};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

pub struct ChoiceText;
const NAME: &str = "com/ktf/kfc/ChoiceText";
const COMPONENT: &str = "org/kwis/msp/lwc/Component";
const FONT: &str = "org/kwis/msp/lcdui/Font";
const GRAPHICS: &str = "org/kwis/msp/lcdui/Graphics";
type Strings = ClassInstanceRef<Array<ClassInstanceRef<()>>>;
impl ChoiceText {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: NAME,
            parent_class: Some(COMPONENT),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("<init>", "([Ljava/lang/String;)V", Self::init, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getAllText", "()[Ljava/lang/String;", Self::all, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setAllText", "([Ljava/lang/String;)V", Self::set_all, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getTextAt", "(I)Ljava/lang/String;", Self::get, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setTextAt", "(ILjava/lang/String;)Z", Self::set, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getSelectedIndex", "()I", Self::selected, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setSelectedIndex", "(I)Z", Self::select, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getFont", "()Lorg/kwis/msp/lcdui/Font;", Self::font, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setFont", "(Lorg/kwis/msp/lcdui/Font;)V", Self::set_font, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setBound", "(IIII)V", Self::bound, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("paintContent", "(Lorg/kwis/msp/lcdui/Graphics;)V", Self::paint, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "paintChoiceText",
                    "(Lorg/kwis/msp/lcdui/Graphics;)V",
                    Self::paint,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("keyNotify", "(II)Z", Self::key, MethodAccessFlags::PUBLIC),
            ],
            fields: vec![
                JavaFieldProto::new("texts", "[Ljava/lang/String;", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("selected", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("font", "Lorg/kwis/msp/lcdui/Font;", FieldAccessFlags::PRIVATE),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }
    async fn init(jvm: &Jvm, context: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, strings: Strings) -> Result<()> {
        let _: () = jvm.invoke_special(&this, COMPONENT, "<init>", "()V", ()).await?;
        jvm.put_field(&mut this, "wieInput", "Z", true).await?;
        let font: ClassInstanceRef<()> = jvm.invoke_static(FONT, "getDefaultFont", "()Lorg/kwis/msp/lcdui/Font;", ()).await?;
        jvm.put_field(&mut this, "font", "Lorg/kwis/msp/lcdui/Font;", font).await?;
        Self::set_all(jvm, context, this, strings).await
    }
    async fn all(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<Strings> {
        jvm.get_field(&this, "texts", "[Ljava/lang/String;").await
    }
    async fn set_all(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, strings: Strings) -> Result<()> {
        if strings.is_null() {
            return Err(jvm.exception("java/lang/NullPointerException", "choice texts").await);
        }
        let count = jvm.array_length(&strings).await?;
        jvm.put_field(&mut this, "texts", "[Ljava/lang/String;", strings).await?;
        let selected: i32 = jvm.get_field(&this, "selected", "I").await?;
        jvm.put_field(
            &mut this,
            "selected",
            "I",
            if count == 0 {
                -1
            } else if selected < 0 || selected as usize >= count {
                0
            } else {
                selected
            },
        )
        .await?;
        jvm.invoke_virtual(&this, COMPONENT, "repaint", "()V", ()).await
    }
    async fn get(jvm: &Jvm, context: &mut WieJvmContext, this: ClassInstanceRef<Self>, index: i32) -> Result<ClassInstanceRef<()>> {
        let texts = Self::all(jvm, context, this).await?;
        Ok(jvm.load_array(&texts, index as _, 1).await?.remove(0))
    }
    async fn set(jvm: &Jvm, context: &mut WieJvmContext, this: ClassInstanceRef<Self>, index: i32, text: ClassInstanceRef<()>) -> Result<bool> {
        let mut texts = Self::all(jvm, context, this.clone()).await?;
        if index < 0 || index as usize >= jvm.array_length(&texts).await? {
            return Ok(false);
        }
        jvm.store_array(&mut texts, index as _, vec![text]).await?;
        let _: () = jvm.invoke_virtual(&this, COMPONENT, "repaint", "()V", ()).await?;
        Ok(true)
    }
    async fn selected(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<i32> {
        jvm.get_field(&this, "selected", "I").await
    }
    async fn select(jvm: &Jvm, context: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, index: i32) -> Result<bool> {
        let texts = Self::all(jvm, context, this.clone()).await?;
        if index < 0 || index as usize >= jvm.array_length(&texts).await? {
            return Ok(false);
        }
        jvm.put_field(&mut this, "selected", "I", index).await?;
        let _: () = jvm.invoke_virtual(&this, COMPONENT, "repaint", "()V", ()).await?;
        Ok(true)
    }
    async fn font(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<ClassInstanceRef<()>> {
        jvm.get_field(&this, "font", "Lorg/kwis/msp/lcdui/Font;").await
    }
    async fn set_font(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, font: ClassInstanceRef<()>) -> Result<()> {
        if font.is_null() {
            return Err(jvm.exception("java/lang/NullPointerException", "choice font").await);
        }
        jvm.put_field(&mut this, "font", "Lorg/kwis/msp/lcdui/Font;", font).await?;
        jvm.invoke_virtual(&this, COMPONENT, "repaint", "()V", ()).await
    }
    async fn bound(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, x: i32, y: i32, w: i32, h: i32) -> Result<()> {
        for (field, value) in [("x", x), ("y", y), ("w", w), ("h", h)] {
            jvm.put_field(&mut this, field, "I", value).await?;
        }
        jvm.invoke_virtual(&this, COMPONENT, "repaint", "()V", ()).await
    }
    async fn key(jvm: &Jvm, context: &mut WieJvmContext, this: ClassInstanceRef<Self>, kind: i32, code: i32) -> Result<bool> {
        use wie_wipi_java::classes::net::wie::WIPIKeyCode;
        let direction = if code == WIPIKeyCode::LEFT as i32 {
            -1
        } else if code == WIPIKeyCode::RIGHT as i32 {
            1
        } else {
            return Ok(false);
        };
        if kind == 1 || kind == 3 {
            let selected = Self::selected(jvm, context, this.clone()).await?;
            Self::select(jvm, context, this, selected.saturating_add(direction)).await?;
        }
        Ok(true)
    }
    async fn paint(jvm: &Jvm, context: &mut WieJvmContext, this: ClassInstanceRef<Self>, g: ClassInstanceRef<()>) -> Result<()> {
        let _: () = jvm
            .invoke_special(&this, COMPONENT, "paintContent", "(Lorg/kwis/msp/lcdui/Graphics;)V", (g.clone(),))
            .await?;
        let font = Self::font(jvm, context, this.clone()).await?;
        let _: () = jvm
            .invoke_virtual(&g, GRAPHICS, "setFont", "(Lorg/kwis/msp/lcdui/Font;)V", (font.clone(),))
            .await?;
        let fg: i32 = jvm.get_field(&this, "fg", "I").await?;
        let _: () = jvm.invoke_virtual(&g, GRAPHICS, "setColor", "(I)V", (fg,)).await?;
        let selected = Self::selected(jvm, context, this.clone()).await?;
        if selected < 0 {
            return Ok(());
        }
        let text = Self::get(jvm, context, this.clone(), selected).await?;
        let w: i32 = jvm.get_field(&this, "w", "I").await?;
        let h: i32 = jvm.get_field(&this, "h", "I").await?;
        let fh: i32 = jvm.invoke_virtual(&font, FONT, "getHeight", "()I", ()).await?;
        let y = (h - fh).max(0) / 2;
        let arrow = JavaLangString::from_rust_string(jvm, "<").await?;
        let _: () = jvm
            .invoke_virtual(&g, GRAPHICS, "drawString", "(Ljava/lang/String;III)V", (arrow, 0, y, 0))
            .await?;
        let arrow = JavaLangString::from_rust_string(jvm, ">").await?;
        let _: () = jvm
            .invoke_virtual(&g, GRAPHICS, "drawString", "(Ljava/lang/String;III)V", (arrow, (w - fh).max(0), y, 0))
            .await?;
        if !text.is_null() {
            let tw: i32 = jvm
                .invoke_virtual(&font, FONT, "stringWidth", "(Ljava/lang/String;)I", (text.clone(),))
                .await?;
            let _: () = jvm
                .invoke_virtual(
                    &g,
                    GRAPHICS,
                    "drawString",
                    "(Ljava/lang/String;III)V",
                    (text, ((w - tw) / 2).max(fh), y, 0),
                )
                .await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;
    use test_utils::run_jvm_test;
    #[test]
    fn choices_preserve_guest_strings_and_only_move_on_press_or_repeat() -> wie_util::Result<()> {
        run_jvm_test(
            Box::new([
                wie_midp::get_protos().into(),
                wie_wipi_java::get_protos().into(),
                vec![ChoiceText::as_proto()].into(),
            ]),
            |jvm| async move {
                let mut texts = jvm.instantiate_array("Ljava/lang/String;", 2).await?;
                let a = JavaLangString::from_rust_string(&jvm, "첫번째").await?;
                let b = JavaLangString::from_rust_string(&jvm, "두번째").await?;
                jvm.store_array(&mut texts, 0, vec![a.clone(), b.clone()]).await?;
                let choice = jvm.new_class(NAME, "([Ljava/lang/String;)V", (texts.clone(),)).await?;
                assert_eq!(jvm.invoke_virtual::<_, i32>(&choice, NAME, "getSelectedIndex", "()I", ()).await?, 0);
                assert!(!jvm.invoke_virtual::<_, bool>(&choice, NAME, "setSelectedIndex", "(I)Z", (2,)).await?);
                let right = wie_wipi_java::classes::net::wie::WIPIKeyCode::RIGHT as i32;
                assert!(jvm.invoke_virtual::<_, bool>(&choice, NAME, "keyNotify", "(II)Z", (2, right)).await?);
                assert_eq!(jvm.invoke_virtual::<_, i32>(&choice, NAME, "getSelectedIndex", "()I", ()).await?, 0);
                assert!(jvm.invoke_virtual::<_, bool>(&choice, NAME, "keyNotify", "(II)Z", (1, right)).await?);
                assert_eq!(jvm.invoke_virtual::<_, i32>(&choice, NAME, "getSelectedIndex", "()I", ()).await?, 1);
                assert!(!jvm.invoke_virtual::<_, bool>(&choice, NAME, "keyNotify", "(II)Z", (1, 48)).await?);
                let value: ClassInstanceRef<()> = jvm.invoke_virtual(&choice, NAME, "getTextAt", "(I)Ljava/lang/String;", (1,)).await?;
                assert_eq!(value.identity(), b.identity());
                assert!(
                    jvm.invoke_virtual::<_, bool>(&choice, NAME, "setTextAt", "(ILjava/lang/String;)Z", (1, a.clone()))
                        .await?
                );
                let values: alloc::vec::Vec<ClassInstanceRef<()>> = jvm.load_array(&texts, 1, 1).await?;
                assert_eq!(values[0].identity(), a.identity());
                let empty = jvm.instantiate_array("Ljava/lang/String;", 0).await?;
                let _: () = jvm
                    .invoke_virtual(&choice, NAME, "setAllText", "([Ljava/lang/String;)V", (empty,))
                    .await?;
                assert_eq!(jvm.invoke_virtual::<_, i32>(&choice, NAME, "getSelectedIndex", "()I", ()).await?, -1);
                Ok(())
            },
        )
    }
}
