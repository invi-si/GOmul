use alloc::vec;
use jvm::{ClassInstanceRef, Jvm, Result};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use wie_jvm_support::{WieJavaClassProto, WieJvmContext};
use wie_wipi_java::classes::net::wie::WIPIKeyCode;

pub struct GFormBase;
const NAME: &str = "com/ktf/kfc/GFormBase";
const FORM: &str = "com/ktf/kfc/GForm";
const SHELL: &str = "org/kwis/msp/lwc/ShellComponent";
const COMPONENT: &str = "org/kwis/msp/lwc/Component";
const CONTAINER: &str = "org/kwis/msp/lwc/ContainerComponent";
const LABEL: &str = "org/kwis/msp/lwc/LabelComponent";
const GRAPHICS: &str = "org/kwis/msp/lcdui/Graphics";
const IMAGE: &str = "Lorg/kwis/msp/lcdui/Image;";
impl GFormBase {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: NAME,
            parent_class: Some(FORM),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("<init>", "()V", Self::init, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "<init>",
                    "(Lorg/kwis/msp/lwc/Component;)V",
                    Self::init_component,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("<init>", "(Ljava/lang/String;)V", Self::init_text, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("<init>", "(Ljava/lang/String;I)V", Self::init_text_id, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "<init>",
                    "(Ljava/lang/String;Lorg/kwis/msp/lcdui/Image;)V",
                    Self::init_image,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new(
                    "<init>",
                    "(Ljava/lang/String;Lorg/kwis/msp/lcdui/Image;I)V",
                    Self::init_image_id,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new(
                    "<init>",
                    "(Ljava/lang/String;Ljava/lang/String;)V",
                    Self::init_path,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new(
                    "getBaseTitle",
                    "()Lorg/kwis/msp/lwc/Component;",
                    Self::get_title,
                    MethodAccessFlags::PROTECTED,
                ),
                JavaMethodProto::new("setTitle", "(Ljava/lang/String;)V", Self::set_title, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setTitleString", "(Ljava/lang/String;)V", Self::set_title, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "setTitleData",
                    "(Ljava/lang/String;Lorg/kwis/msp/lcdui/Image;)V",
                    Self::set_title_data,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("initializeUI", "()V", Self::initialize_ui, MethodAccessFlags::PROTECTED),
                JavaMethodProto::new(
                    "getBackgroundImage",
                    "()Lorg/kwis/msp/lcdui/Image;",
                    Self::get_background,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new(
                    "setBackgroundImage",
                    "(Lorg/kwis/msp/lcdui/Image;)V",
                    Self::set_background,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("getBackImgX", "()I", Self::background_x, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getBackImgY", "()I", Self::background_y, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setBackImagePos", "(II)V", Self::background_position, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "paintContent",
                    "(Lorg/kwis/msp/lcdui/Graphics;)V",
                    Self::paint_content,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("paint", "(Lorg/kwis/msp/lcdui/Graphics;)V", Self::paint, MethodAccessFlags::PROTECTED),
                JavaMethodProto::new(
                    "afterFirstPainted",
                    "(Lorg/kwis/msp/lcdui/Graphics;)V",
                    Self::after_first_painted,
                    MethodAccessFlags::PROTECTED,
                ),
                JavaMethodProto::new("checkForms", "()V", Self::check_forms, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("focusNotify", "(Z)V", Self::focus_notify, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("focusChanged", "(Z)Z", Self::focus_changed, MethodAccessFlags::PROTECTED),
                JavaMethodProto::new("keyNotify", "(II)Z", Self::key_notify, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("onCancelKey", "(I)Z", Self::on_cancel_key, MethodAccessFlags::PROTECTED),
            ],
            fields: vec![
                JavaFieldProto::new("wieBackgroundImage", IMAGE, FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("wieBackX", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("wieBackY", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("wiePainted", "Z", FieldAccessFlags::PRIVATE),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }
    async fn init(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<()> {
        Self::init_text_id(jvm, ctx, this, None.into(), 0).await
    }
    async fn init_component(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, title: ClassInstanceRef<()>) -> Result<()> {
        let _: () = jvm.invoke_special(&this, FORM, "<init>", "(I)V", (0,)).await?;
        jvm.invoke_special(&this, SHELL, "setTitle", "(Lorg/kwis/msp/lwc/Component;)V", (title,))
            .await
    }
    async fn init_text(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>, title: ClassInstanceRef<()>) -> Result<()> {
        Self::init_text_id(jvm, ctx, this, title, 0).await
    }
    async fn init_text_id(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>, title: ClassInstanceRef<()>, id: i32) -> Result<()> {
        Self::init_image_id(jvm, ctx, this, title, None.into(), id).await
    }
    async fn init_image(
        jvm: &Jvm,
        ctx: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        title: ClassInstanceRef<()>,
        image: ClassInstanceRef<()>,
    ) -> Result<()> {
        Self::init_image_id(jvm, ctx, this, title, image, 0).await
    }
    async fn init_image_id(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        title: ClassInstanceRef<()>,
        image: ClassInstanceRef<()>,
        id: i32,
    ) -> Result<()> {
        let _: () = jvm.invoke_special(&this, FORM, "<init>", "(I)V", (id,)).await?;
        let label = jvm
            .new_class(LABEL, "(Ljava/lang/String;Lorg/kwis/msp/lcdui/Image;)V", (title, image))
            .await?;
        jvm.invoke_special(&this, SHELL, "setTitle", "(Lorg/kwis/msp/lwc/Component;)V", (label,))
            .await
    }
    async fn init_path(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        title: ClassInstanceRef<()>,
        path: ClassInstanceRef<()>,
    ) -> Result<()> {
        let _: () = jvm.invoke_special(&this, FORM, "<init>", "(I)V", (0,)).await?;
        let label = jvm.new_class(LABEL, "(Ljava/lang/String;Ljava/lang/String;)V", (title, path)).await?;
        jvm.invoke_special(&this, SHELL, "setTitle", "(Lorg/kwis/msp/lwc/Component;)V", (label,))
            .await
    }
    async fn get_title(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<ClassInstanceRef<()>> {
        jvm.invoke_special(&this, SHELL, "getTitle", "()Lorg/kwis/msp/lwc/Component;", ()).await
    }
    async fn label(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<ClassInstanceRef<()>> {
        let title = Self::get_title(jvm, ctx, this.clone()).await?;
        if !title.is_null() && jvm.is_instance(&**title, LABEL) {
            return Ok(title);
        }
        let label = jvm.new_class(LABEL, "()V", ()).await?;
        let _: () = jvm
            .invoke_special(&this, SHELL, "setTitle", "(Lorg/kwis/msp/lwc/Component;)V", (label.clone(),))
            .await?;
        Ok(label.into())
    }
    async fn set_title(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>, text: ClassInstanceRef<()>) -> Result<()> {
        let label = Self::label(jvm, ctx, this.clone()).await?;
        let _: () = jvm.invoke_virtual(&label, LABEL, "setLabel", "(Ljava/lang/String;)V", (text,)).await?;
        Self::changed(jvm, this).await
    }
    async fn set_title_data(
        jvm: &Jvm,
        ctx: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        text: ClassInstanceRef<()>,
        image: ClassInstanceRef<()>,
    ) -> Result<()> {
        let label = Self::label(jvm, ctx, this.clone()).await?;
        let _: () = jvm.invoke_virtual(&label, LABEL, "setLabel", "(Ljava/lang/String;)V", (text,)).await?;
        let _: () = jvm
            .invoke_virtual(&label, LABEL, "setImage", "(Lorg/kwis/msp/lcdui/Image;)V", (image,))
            .await?;
        Self::changed(jvm, this).await
    }
    async fn initialize_ui(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<()> {
        if Self::get_title(jvm, ctx, this.clone()).await?.is_null() {
            Self::label(jvm, ctx, this.clone()).await?;
        }
        Self::changed(jvm, this).await
    }
    async fn changed(jvm: &Jvm, this: ClassInstanceRef<Self>) -> Result<()> {
        let _: () = jvm.invoke_virtual(&this, COMPONENT, "invalidate", "()V", ()).await?;
        jvm.invoke_virtual(&this, COMPONENT, "repaint", "()V", ()).await
    }
    async fn get_background(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<ClassInstanceRef<()>> {
        jvm.get_field_in_class(&this, NAME, "wieBackgroundImage", IMAGE).await
    }
    async fn set_background(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, image: ClassInstanceRef<()>) -> Result<()> {
        jvm.put_field_in_class(&mut this, NAME, "wieBackgroundImage", IMAGE, image).await?;
        Self::changed(jvm, this).await
    }
    async fn background_x(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<i32> {
        jvm.get_field_in_class(&this, NAME, "wieBackX", "I").await
    }
    async fn background_y(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<i32> {
        jvm.get_field_in_class(&this, NAME, "wieBackY", "I").await
    }
    async fn background_position(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, x: i32, y: i32) -> Result<()> {
        jvm.put_field_in_class(&mut this, NAME, "wieBackX", "I", x).await?;
        jvm.put_field_in_class(&mut this, NAME, "wieBackY", "I", y).await?;
        Self::changed(jvm, this).await
    }
    async fn paint_content(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>, g: ClassInstanceRef<()>) -> Result<()> {
        let _: () = jvm
            .invoke_special(&this, COMPONENT, "paintContent", "(Lorg/kwis/msp/lcdui/Graphics;)V", (g.clone(),))
            .await?;
        let image = Self::get_background(jvm, ctx, this.clone()).await?;
        if !image.is_null() {
            let x = Self::background_x(jvm, ctx, this.clone()).await?;
            let y = Self::background_y(jvm, ctx, this).await?;
            let _: () = jvm
                .invoke_virtual(&g, GRAPHICS, "drawImage", "(Lorg/kwis/msp/lcdui/Image;III)V", (image, x, y, 0))
                .await?;
        }
        Ok(())
    }
    async fn paint(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, g: ClassInstanceRef<()>) -> Result<()> {
        let _: () = jvm
            .invoke_special(&this, CONTAINER, "paint", "(Lorg/kwis/msp/lcdui/Graphics;)V", (g.clone(),))
            .await?;
        if !jvm.get_field_in_class::<bool>(&this, NAME, "wiePainted", "Z").await? {
            jvm.put_field_in_class(&mut this, NAME, "wiePainted", "Z", true).await?;
            let _: () = jvm
                .invoke_virtual(&this, NAME, "afterFirstPainted", "(Lorg/kwis/msp/lcdui/Graphics;)V", (g,))
                .await?;
        }
        Ok(())
    }
    async fn after_first_painted(_: &Jvm, _: &mut WieJvmContext, _: ClassInstanceRef<Self>, _: ClassInstanceRef<()>) -> Result<()> {
        Ok(())
    }
    async fn check_forms(_: &Jvm, _: &mut WieJvmContext, _: ClassInstanceRef<Self>) -> Result<()> {
        Ok(())
    }
    async fn focus_notify(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, focused: bool) -> Result<()> {
        let _: () = jvm.invoke_special(&this, COMPONENT, "focusNotify", "(Z)V", (focused,)).await?;
        let _: bool = jvm.invoke_virtual(&this, NAME, "focusChanged", "(Z)Z", (focused,)).await?;
        Ok(())
    }
    async fn focus_changed(_: &Jvm, _: &mut WieJvmContext, _: ClassInstanceRef<Self>, _: bool) -> Result<bool> {
        Ok(false)
    }
    async fn key_notify(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, kind: i32, code: i32) -> Result<bool> {
        let handled: bool = jvm.invoke_special(&this, SHELL, "keyNotify", "(II)Z", (kind, code)).await?;
        if handled {
            return Ok(true);
        }
        if code == WIPIKeyCode::CLEAR as i32 {
            return jvm.invoke_virtual(&this, NAME, "onCancelKey", "(I)Z", (kind,)).await;
        }
        Ok(false)
    }
    async fn on_cancel_key(_: &Jvm, _: &mut WieJvmContext, _: ClassInstanceRef<Self>, _: i32) -> Result<bool> {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;
    use jvm::runtime::JavaLangString;
    use test_utils::run_jvm_test;

    fn protos() -> Box<[WieJavaClassProto]> {
        vec![
            super::super::gform::GForm::as_proto(),
            GFormBase::as_proto(),
            WieJavaClassProto {
                name: "test/BaseJlet",
                parent_class: Some("org/kwis/msp/lcdui/Jlet"),
                interfaces: vec![],
                methods: vec![],
                fields: vec![],
                access_flags: ClassAccessFlags::PUBLIC,
            },
            WieJavaClassProto {
                name: "test/BaseHooks",
                parent_class: Some(NAME),
                interfaces: vec![],
                methods: vec![
                    JavaMethodProto::new(
                        "afterFirstPainted",
                        "(Lorg/kwis/msp/lcdui/Graphics;)V",
                        first_paint,
                        MethodAccessFlags::PROTECTED,
                    ),
                    JavaMethodProto::new("focusChanged", "(Z)Z", focus, MethodAccessFlags::PROTECTED),
                    JavaMethodProto::new("onCancelKey", "(I)Z", cancel, MethodAccessFlags::PROTECTED),
                ],
                fields: vec![
                    JavaFieldProto::new("paintCalls", "I", FieldAccessFlags::PUBLIC),
                    JavaFieldProto::new("focusCalls", "I", FieldAccessFlags::PUBLIC),
                    JavaFieldProto::new("cancelKind", "I", FieldAccessFlags::PUBLIC),
                ],
                access_flags: ClassAccessFlags::PUBLIC,
            },
        ]
        .into_boxed_slice()
    }
    async fn init(jvm: &Jvm) -> Result<()> {
        let _ = jvm.new_class("net/wie/WIPIMIDlet", "()V", ()).await?;
        let jlet = jvm.instantiate_class("test/BaseJlet").await?;
        jvm.invoke_special(&jlet, "org/kwis/msp/lcdui/Jlet", "<init>", "()V", ()).await
    }
    async fn first_paint(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<()>, _: ClassInstanceRef<()>) -> Result<()> {
        let calls: i32 = jvm.get_field(&this, "paintCalls", "I").await?;
        jvm.put_field(&mut this, "paintCalls", "I", calls + 1).await
    }
    async fn focus(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<()>, _: bool) -> Result<bool> {
        let calls: i32 = jvm.get_field(&this, "focusCalls", "I").await?;
        jvm.put_field(&mut this, "focusCalls", "I", calls + 1).await?;
        Ok(true)
    }
    async fn cancel(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<()>, kind: i32) -> Result<bool> {
        jvm.put_field(&mut this, "cancelKind", "I", kind).await?;
        Ok(true)
    }
    #[test]
    fn titles_keep_icons_and_custom_components_and_form_ids() -> wie_util::Result<()> {
        run_jvm_test(
            Box::new([wie_midp::get_protos().into(), wie_wipi_java::get_protos().into(), protos()]),
            |jvm| async move {
                init(&jvm).await?;
                let title = JavaLangString::from_rust_string(&jvm, "제목").await?;
                let icon: ClassInstanceRef<()> = jvm
                    .invoke_static("org/kwis/msp/lcdui/Image", "createImage", "(II)Lorg/kwis/msp/lcdui/Image;", (4, 4))
                    .await?;
                let form = jvm
                    .new_class(
                        NAME,
                        "(Ljava/lang/String;Lorg/kwis/msp/lcdui/Image;I)V",
                        (title.clone(), icon.clone(), 77),
                    )
                    .await?;
                assert_eq!(jvm.invoke_virtual::<_, i32>(&form, FORM, "getFormID", "()I", ()).await?, 77);
                let label: ClassInstanceRef<()> = jvm
                    .invoke_virtual(&form, NAME, "getBaseTitle", "()Lorg/kwis/msp/lwc/Component;", ())
                    .await?;
                let original: ClassInstanceRef<()> = jvm.invoke_virtual(&label, LABEL, "getLabel", "()Ljava/lang/String;", ()).await?;
                assert_eq!(original.identity(), title.identity());
                let changed = JavaLangString::from_rust_string(&jvm, "새 제목").await?;
                let _: () = jvm
                    .invoke_virtual(&form, NAME, "setTitleString", "(Ljava/lang/String;)V", (changed.clone(),))
                    .await?;
                let stored: ClassInstanceRef<()> = jvm.invoke_virtual(&label, LABEL, "getLabel", "()Ljava/lang/String;", ()).await?;
                assert_eq!(stored.identity(), changed.identity());
                let stored_icon: ClassInstanceRef<()> = jvm.invoke_virtual(&label, LABEL, "getImage", "()Lorg/kwis/msp/lcdui/Image;", ()).await?;
                assert_eq!(stored_icon.identity(), icon.identity());
                let _: () = jvm
                    .invoke_virtual(
                        &form,
                        NAME,
                        "setTitleData",
                        "(Ljava/lang/String;Lorg/kwis/msp/lcdui/Image;)V",
                        (title, ClassInstanceRef::<()>::new(None)),
                    )
                    .await?;
                let stored_icon: ClassInstanceRef<()> = jvm.invoke_virtual(&label, LABEL, "getImage", "()Lorg/kwis/msp/lcdui/Image;", ()).await?;
                assert!(stored_icon.is_null());
                let custom = jvm.new_class("org/kwis/msp/lwc/ButtonComponent", "()V", ()).await?;
                let other = jvm.new_class(NAME, "(Lorg/kwis/msp/lwc/Component;)V", (custom.clone(),)).await?;
                let _: () = jvm.invoke_virtual(&other, NAME, "initializeUI", "()V", ()).await?;
                let stored: ClassInstanceRef<()> = jvm
                    .invoke_virtual(&other, SHELL, "getTitle", "()Lorg/kwis/msp/lwc/Component;", ())
                    .await?;
                assert_eq!(stored.identity(), custom.identity());
                Ok(())
            },
        )
    }
    #[test]
    fn background_pixels_and_first_paint_focus_cancel_hooks() -> wie_util::Result<()> {
        run_jvm_test(
            Box::new([wie_midp::get_protos().into(), wie_wipi_java::get_protos().into(), protos()]),
            |jvm| async move {
                init(&jvm).await?;
                let form = jvm.instantiate_class("test/BaseHooks").await?;
                let _: () = jvm
                    .invoke_special(
                        &form,
                        NAME,
                        "<init>",
                        "(Lorg/kwis/msp/lwc/Component;)V",
                        (ClassInstanceRef::<()>::new(None),),
                    )
                    .await?;
                let _: () = jvm.invoke_virtual(&form, COMPONENT, "setBackground", "(I)V", (0x112233,)).await?;
                let image: ClassInstanceRef<()> = jvm
                    .invoke_static("org/kwis/msp/lcdui/Image", "createImage", "(II)Lorg/kwis/msp/lcdui/Image;", (2, 2))
                    .await?;
                let ig: ClassInstanceRef<()> = jvm
                    .invoke_virtual(&image, "org/kwis/msp/lcdui/Image", "getGraphics", "()Lorg/kwis/msp/lcdui/Graphics;", ())
                    .await?;
                let _: () = jvm.invoke_virtual(&ig, GRAPHICS, "setColor", "(I)V", (0xabcdef,)).await?;
                let _: () = jvm.invoke_virtual(&ig, GRAPHICS, "fillRect", "(IIII)V", (0, 0, 2, 2)).await?;
                let _: () = jvm
                    .invoke_virtual(&form, NAME, "setBackgroundImage", "(Lorg/kwis/msp/lcdui/Image;)V", (image.clone(),))
                    .await?;
                let _: () = jvm.invoke_virtual(&form, NAME, "setBackImagePos", "(II)V", (3, 4)).await?;
                let stored: ClassInstanceRef<()> = jvm
                    .invoke_virtual(&form, NAME, "getBackgroundImage", "()Lorg/kwis/msp/lcdui/Image;", ())
                    .await?;
                assert_eq!(stored.identity(), image.identity());
                let canvas: ClassInstanceRef<()> = jvm
                    .invoke_static("org/kwis/msp/lcdui/Image", "createImage", "(II)Lorg/kwis/msp/lcdui/Image;", (20, 20))
                    .await?;
                let g: ClassInstanceRef<()> = jvm
                    .invoke_virtual(&canvas, "org/kwis/msp/lcdui/Image", "getGraphics", "()Lorg/kwis/msp/lcdui/Graphics;", ())
                    .await?;
                for _ in 0..2 {
                    let _: () = jvm
                        .invoke_virtual(&form, NAME, "paint", "(Lorg/kwis/msp/lcdui/Graphics;)V", (g.clone(),))
                        .await?;
                }
                assert_eq!(jvm.get_field::<i32>(&form, "paintCalls", "I").await?, 1);
                for (x, y, color) in [(3, 4, 0xabcdef), (4, 5, 0xabcdef), (2, 4, 0x112233), (5, 5, 0x112233)] {
                    assert_eq!(jvm.invoke_virtual::<_, i32>(&g, GRAPHICS, "getPixel", "(II)I", (x, y)).await?, color);
                }
                let _: () = jvm
                    .invoke_virtual(
                        &form,
                        NAME,
                        "setBackgroundImage",
                        "(Lorg/kwis/msp/lcdui/Image;)V",
                        (ClassInstanceRef::<()>::new(None),),
                    )
                    .await?;
                let _: () = jvm
                    .invoke_virtual(&form, NAME, "paint", "(Lorg/kwis/msp/lcdui/Graphics;)V", (g.clone(),))
                    .await?;
                assert_eq!(jvm.invoke_virtual::<_, i32>(&g, GRAPHICS, "getPixel", "(II)I", (3, 4)).await?, 0x112233);
                for focused in [true, false] {
                    let _: () = jvm.invoke_virtual(&form, NAME, "focusNotify", "(Z)V", (focused,)).await?;
                }
                assert_eq!(jvm.get_field::<i32>(&form, "focusCalls", "I").await?, 2);
                assert!(!jvm.invoke_virtual::<_, bool>(&form, COMPONENT, "hasFocus", "()Z", ()).await?);
                assert!(
                    jvm.invoke_virtual::<_, bool>(&form, NAME, "keyNotify", "(II)Z", (1, WIPIKeyCode::CLEAR as i32))
                        .await?
                );
                assert_eq!(jvm.get_field::<i32>(&form, "cancelKind", "I").await?, 1);
                assert!(!jvm.invoke_virtual::<_, bool>(&form, NAME, "keyNotify", "(II)Z", (1, 49)).await?);
                Ok(())
            },
        )
    }
}
