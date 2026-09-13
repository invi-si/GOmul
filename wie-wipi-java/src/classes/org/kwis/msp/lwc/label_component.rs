use alloc::vec;
use jvm::{ClassInstanceRef, Jvm, Result};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use wie_jvm_support::{WieJavaClassProto, WieJvmContext};
pub struct LabelComponent;
const NAME: &str = "org/kwis/msp/lwc/LabelComponent";
const GRAPHICS: &str = "org/kwis/msp/lcdui/Graphics";
impl LabelComponent {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: NAME,
            parent_class: Some("org/kwis/msp/lwc/Component"),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("calcPreferredSize", "(I)V", Self::preferred, MethodAccessFlags::PROTECTED),
                JavaMethodProto::new("<init>", "()V", Self::init, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("<init>", "(Ljava/lang/String;)V", Self::init_text, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "<init>",
                    "(Ljava/lang/String;Lorg/kwis/msp/lcdui/Image;)V",
                    Self::init_image,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new(
                    "<init>",
                    "(Ljava/lang/String;Ljava/lang/String;)V",
                    Self::init_resource,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("paintContent", "(Lorg/kwis/msp/lcdui/Graphics;)V", Self::paint, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getLabel", "()Ljava/lang/String;", Self::get_label, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setLabel", "(Ljava/lang/String;)V", Self::set_label, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getImage", "()Lorg/kwis/msp/lcdui/Image;", Self::get_image, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setImage", "(Lorg/kwis/msp/lcdui/Image;)V", Self::set_image, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getFont", "()Lorg/kwis/msp/lcdui/Font;", Self::get_font, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setFont", "(Lorg/kwis/msp/lcdui/Font;)V", Self::set_font, MethodAccessFlags::PUBLIC),
            ],
            fields: vec![
                JavaFieldProto::new("m_str", "Ljava/lang/String;", FieldAccessFlags::PROTECTED),
                JavaFieldProto::new("m_image", "Lorg/kwis/msp/lcdui/Image;", FieldAccessFlags::PROTECTED),
                JavaFieldProto::new("m_ft", "Lorg/kwis/msp/lcdui/Font;", FieldAccessFlags::PROTECTED),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }
    async fn init(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<()> {
        Self::init_image(jvm, ctx, this, None.into(), None.into()).await
    }
    async fn init_text(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>, text: ClassInstanceRef<()>) -> Result<()> {
        Self::init_image(jvm, ctx, this, text, None.into()).await
    }
    async fn init_image(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        text: ClassInstanceRef<()>,
        image: ClassInstanceRef<()>,
    ) -> Result<()> {
        let _: () = jvm.invoke_special(&this, "org/kwis/msp/lwc/Component", "<init>", "()V", ()).await?;
        let font: ClassInstanceRef<()> = jvm
            .invoke_static("org/kwis/msp/lcdui/Font", "getDefaultFont", "()Lorg/kwis/msp/lcdui/Font;", ())
            .await?;
        jvm.put_field(&mut this, "m_str", "Ljava/lang/String;", text).await?;
        jvm.put_field(&mut this, "m_image", "Lorg/kwis/msp/lcdui/Image;", image).await?;
        jvm.put_field(&mut this, "m_ft", "Lorg/kwis/msp/lcdui/Font;", font).await
    }
    async fn init_resource(
        jvm: &Jvm,
        ctx: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        text: ClassInstanceRef<()>,
        resource: ClassInstanceRef<()>,
    ) -> Result<()> {
        let image = if resource.is_null() {
            None.into()
        } else {
            jvm.invoke_static(
                "org/kwis/msp/lcdui/Image",
                "createImage",
                "(Ljava/lang/String;)Lorg/kwis/msp/lcdui/Image;",
                (resource,),
            )
            .await?
        };
        Self::init_image(jvm, ctx, this, text, image).await
    }
    async fn preferred(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, _: i32) -> Result<()> {
        let text: ClassInstanceRef<()> = jvm.get_field(&this, "m_str", "Ljava/lang/String;").await?;
        let image: ClassInstanceRef<()> = jvm.get_field(&this, "m_image", "Lorg/kwis/msp/lcdui/Image;").await?;
        let font: ClassInstanceRef<()> = jvm.get_field(&this, "m_ft", "Lorg/kwis/msp/lcdui/Font;").await?;
        let mut w = 0i32;
        let mut h = 0i32;
        if !text.is_null() {
            w = jvm
                .invoke_virtual(&font, "org/kwis/msp/lcdui/Font", "stringWidth", "(Ljava/lang/String;)I", (text,))
                .await?;
            h = jvm.invoke_virtual(&font, "org/kwis/msp/lcdui/Font", "getHeight", "()I", ()).await?;
        }
        if !image.is_null() {
            let iw: i32 = jvm.invoke_virtual(&image, "org/kwis/msp/lcdui/Image", "getWidth", "()I", ()).await?;
            let ih: i32 = jvm.invoke_virtual(&image, "org/kwis/msp/lcdui/Image", "getHeight", "()I", ()).await?;
            w = w.saturating_add(iw);
            h = h.max(ih);
        }
        jvm.put_field(&mut this, "prefW", "I", w).await?;
        jvm.put_field(&mut this, "prefH", "I", h).await
    }
    async fn get_label(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<ClassInstanceRef<()>> {
        jvm.get_field(&this, "m_str", "Ljava/lang/String;").await
    }
    async fn set_label(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, value: ClassInstanceRef<()>) -> Result<()> {
        jvm.put_field(&mut this, "m_str", "Ljava/lang/String;", value).await?;
        jvm.invoke_virtual(&this, "org/kwis/msp/lwc/Component", "repaint", "()V", ()).await
    }
    async fn get_image(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<ClassInstanceRef<()>> {
        jvm.get_field(&this, "m_image", "Lorg/kwis/msp/lcdui/Image;").await
    }
    async fn set_image(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, value: ClassInstanceRef<()>) -> Result<()> {
        jvm.put_field(&mut this, "m_image", "Lorg/kwis/msp/lcdui/Image;", value).await?;
        jvm.invoke_virtual(&this, "org/kwis/msp/lwc/Component", "repaint", "()V", ()).await
    }
    async fn get_font(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<ClassInstanceRef<()>> {
        jvm.get_field(&this, "m_ft", "Lorg/kwis/msp/lcdui/Font;").await
    }
    async fn set_font(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, value: ClassInstanceRef<()>) -> Result<()> {
        jvm.put_field(&mut this, "m_ft", "Lorg/kwis/msp/lcdui/Font;", value).await?;
        jvm.invoke_virtual(&this, "org/kwis/msp/lwc/Component", "repaint", "()V", ()).await
    }
    async fn paint(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, g: ClassInstanceRef<()>) -> Result<()> {
        let _: () = jvm
            .invoke_special(
                &this,
                "org/kwis/msp/lwc/Component",
                "paintContent",
                "(Lorg/kwis/msp/lcdui/Graphics;)V",
                (g.clone(),),
            )
            .await?;
        let text: ClassInstanceRef<()> = jvm.get_field(&this, "m_str", "Ljava/lang/String;").await?;
        let image: ClassInstanceRef<()> = jvm.get_field(&this, "m_image", "Lorg/kwis/msp/lcdui/Image;").await?;
        let font: ClassInstanceRef<()> = jvm.get_field(&this, "m_ft", "Lorg/kwis/msp/lcdui/Font;").await?;
        let fg: i32 = jvm.get_field(&this, "fg", "I").await?;
        let _: () = jvm.invoke_virtual(&g, GRAPHICS, "setColor", "(I)V", (fg,)).await?;
        let _: () = jvm
            .invoke_virtual(&g, GRAPHICS, "setFont", "(Lorg/kwis/msp/lcdui/Font;)V", (font,))
            .await?;
        let mut x = 0i32;
        if !image.is_null() {
            x = jvm.invoke_virtual(&image, "org/kwis/msp/lcdui/Image", "getWidth", "()I", ()).await?;
            let _: () = jvm
                .invoke_virtual(&g, GRAPHICS, "drawImage", "(Lorg/kwis/msp/lcdui/Image;III)V", (image, 0, 0, 0))
                .await?;
        }
        if !text.is_null() {
            let _: () = jvm
                .invoke_virtual(&g, GRAPHICS, "drawString", "(Ljava/lang/String;III)V", (text, x, 0, 0))
                .await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;
    use jvm::runtime::JavaLangString;
    use test_utils::run_jvm_test;
    #[test]
    fn nullable_label_state_is_independent_and_content_paints() -> wie_util::Result<()> {
        run_jvm_test(Box::new([wie_midp::get_protos().into(), crate::get_protos().into()]), |jvm| async move {
            let text = JavaLangString::from_rust_string(&jvm, "가나다").await?;
            let mut label = jvm.new_class(NAME, "(Ljava/lang/String;)V", (text.clone(),)).await?;
            let other = jvm.new_class(NAME, "()V", ()).await?;
            let stored: ClassInstanceRef<()> = jvm.invoke_virtual(&label, NAME, "getLabel", "()Ljava/lang/String;", ()).await?;
            assert_eq!(stored.identity(), text.identity());
            let empty: ClassInstanceRef<()> = jvm.invoke_virtual(&other, NAME, "getLabel", "()Ljava/lang/String;", ()).await?;
            assert!(empty.is_null());
            let font: ClassInstanceRef<()> = jvm.invoke_virtual(&label, NAME, "getFont", "()Lorg/kwis/msp/lcdui/Font;", ()).await?;
            assert!(!font.is_null());
            jvm.put_field(&mut label, "w", "I", 64).await?;
            jvm.put_field(&mut label, "h", "I", 32).await?;
            let image: ClassInstanceRef<()> = jvm
                .invoke_static("org/kwis/msp/lcdui/Image", "createImage", "(II)Lorg/kwis/msp/lcdui/Image;", (64, 32))
                .await?;
            let g: ClassInstanceRef<()> = jvm
                .invoke_virtual(&image, "org/kwis/msp/lcdui/Image", "getGraphics", "()Lorg/kwis/msp/lcdui/Graphics;", ())
                .await?;
            let _: () = jvm.invoke_virtual(&label, NAME, "setBackground", "(I)V", (0x123456,)).await?;
            let _: () = jvm
                .invoke_virtual(&label, NAME, "paintContent", "(Lorg/kwis/msp/lcdui/Graphics;)V", (g.clone(),))
                .await?;
            assert_eq!(jvm.invoke_virtual::<_, i32>(&g, GRAPHICS, "getPixel", "(II)I", (63, 31)).await?, 0x123456);
            let _: () = jvm
                .invoke_virtual(&label, NAME, "setLabel", "(Ljava/lang/String;)V", (ClassInstanceRef::<()>::new(None),))
                .await?;
            let empty: ClassInstanceRef<()> = jvm.invoke_virtual(&label, NAME, "getLabel", "()Ljava/lang/String;", ()).await?;
            assert!(empty.is_null());
            Ok(())
        })
    }
}
