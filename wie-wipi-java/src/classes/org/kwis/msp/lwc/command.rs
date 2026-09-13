use alloc::vec;
use jvm::{ClassInstanceRef, Jvm, Result};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

pub struct Command;
const IMAGE: &str = "Lorg/kwis/msp/lcdui/Image;";
impl Command {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "org/kwis/msp/lwc/Command",
            parent_class: Some("java/lang/Object"),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new(
                    "<init>",
                    "(Ljava/lang/String;Ljava/lang/Object;)V",
                    Self::init_text,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new(
                    "<init>",
                    "(Ljava/lang/String;Lorg/kwis/msp/lcdui/Image;Ljava/lang/Object;)V",
                    Self::init_image,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new(
                    "<init>",
                    "(Ljava/lang/String;Lorg/kwis/msp/lcdui/Image;Lorg/kwis/msp/lcdui/Image;Ljava/lang/Object;)V",
                    Self::init_images,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new(
                    "<init>",
                    "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/Object;)V",
                    Self::init_path,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new(
                    "<init>",
                    "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/Object;)V",
                    Self::init_paths,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("getString", "()Ljava/lang/String;", Self::text, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getExtObject", "()Ljava/lang/Object;", Self::object, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getNormalImage", "()Lorg/kwis/msp/lcdui/Image;", Self::normal, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getActiveImage", "()Lorg/kwis/msp/lcdui/Image;", Self::active, MethodAccessFlags::PUBLIC),
            ],
            fields: vec![
                JavaFieldProto::new("wieText", "Ljava/lang/String;", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("wieObject", "Ljava/lang/Object;", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("wieNormal", IMAGE, FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("wieActive", IMAGE, FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("wieNormalPath", "Ljava/lang/String;", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("wieActivePath", "Ljava/lang/String;", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("wieSharedImage", "Z", FieldAccessFlags::PRIVATE),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }
    async fn init_text(
        jvm: &Jvm,
        ctx: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        text: ClassInstanceRef<()>,
        object: ClassInstanceRef<()>,
    ) -> Result<()> {
        Self::init_images(jvm, ctx, this, text, None.into(), None.into(), object).await
    }
    async fn init_image(
        jvm: &Jvm,
        ctx: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        text: ClassInstanceRef<()>,
        image: ClassInstanceRef<()>,
        object: ClassInstanceRef<()>,
    ) -> Result<()> {
        Self::init_images(jvm, ctx, this, text, image.clone(), image, object).await
    }
    async fn init_images(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        text: ClassInstanceRef<()>,
        normal: ClassInstanceRef<()>,
        active: ClassInstanceRef<()>,
        object: ClassInstanceRef<()>,
    ) -> Result<()> {
        let _: () = jvm.invoke_special(&this, "java/lang/Object", "<init>", "()V", ()).await?;
        jvm.put_field(&mut this, "wieText", "Ljava/lang/String;", text).await?;
        jvm.put_field(&mut this, "wieObject", "Ljava/lang/Object;", object).await?;
        jvm.put_field(&mut this, "wieNormal", IMAGE, normal).await?;
        jvm.put_field(&mut this, "wieActive", IMAGE, active).await
    }
    async fn init_path(
        jvm: &Jvm,
        ctx: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        text: ClassInstanceRef<()>,
        path: ClassInstanceRef<()>,
        object: ClassInstanceRef<()>,
    ) -> Result<()> {
        Self::init_paths(jvm, ctx, this.clone(), text, path, None.into(), object).await?;
        jvm.put_field(&mut this, "wieSharedImage", "Z", true).await
    }
    async fn init_paths(
        jvm: &Jvm,
        ctx: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        text: ClassInstanceRef<()>,
        normal: ClassInstanceRef<()>,
        active: ClassInstanceRef<()>,
        object: ClassInstanceRef<()>,
    ) -> Result<()> {
        Self::init_text(jvm, ctx, this.clone(), text, object).await?;
        jvm.put_field(&mut this, "wieNormalPath", "Ljava/lang/String;", normal).await?;
        jvm.put_field(&mut this, "wieActivePath", "Ljava/lang/String;", active).await
    }
    async fn text(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<ClassInstanceRef<()>> {
        jvm.get_field(&this, "wieText", "Ljava/lang/String;").await
    }
    async fn object(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<ClassInstanceRef<()>> {
        jvm.get_field(&this, "wieObject", "Ljava/lang/Object;").await
    }
    async fn normal(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<ClassInstanceRef<()>> {
        Self::load(jvm, this, "wieNormal", "wieNormalPath").await
    }
    async fn active(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<ClassInstanceRef<()>> {
        if jvm.get_field::<bool>(&this, "wieSharedImage", "Z").await? {
            Self::normal(jvm, ctx, this).await
        } else {
            Self::load(jvm, this, "wieActive", "wieActivePath").await
        }
    }
    async fn load(jvm: &Jvm, mut this: ClassInstanceRef<Self>, field: &str, path_field: &str) -> Result<ClassInstanceRef<()>> {
        let mut image: ClassInstanceRef<()> = jvm.get_field(&this, field, IMAGE).await?;
        if image.is_null() {
            let path: ClassInstanceRef<()> = jvm.get_field(&this, path_field, "Ljava/lang/String;").await?;
            if !path.is_null() {
                image = jvm
                    .invoke_static(
                        "org/kwis/msp/lcdui/Image",
                        "loadImage",
                        "(Ljava/lang/String;Lorg/kwis/msp/lcdui/ImageObserver;)Lorg/kwis/msp/lcdui/Image;",
                        (path, ClassInstanceRef::<()>::new(None)),
                    )
                    .await?;
                jvm.put_field(&mut this, field, IMAGE, image.clone()).await?;
            }
        }
        Ok(image)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;
    use jvm::runtime::JavaLangString;
    use test_utils::run_jvm_test;
    const NAME: &str = "org/kwis/msp/lwc/Command";

    #[test]
    fn command_preserves_nullable_text_payload_and_distinct_images() -> wie_util::Result<()> {
        run_jvm_test(Box::new([wie_midp::get_protos().into(), crate::get_protos().into()]), |jvm| async move {
            let text = JavaLangString::from_rust_string(&jvm, "확인").await?;
            let payload = jvm.new_class("java/lang/Object", "()V", ()).await?;
            let image: ClassInstanceRef<()> = jvm
                .invoke_static("org/kwis/msp/lcdui/Image", "createImage", "(II)Lorg/kwis/msp/lcdui/Image;", (20, 20))
                .await?;
            let active: ClassInstanceRef<()> = jvm
                .invoke_static("org/kwis/msp/lcdui/Image", "createImage", "(II)Lorg/kwis/msp/lcdui/Image;", (20, 20))
                .await?;
            let command = jvm
                .new_class(
                    NAME,
                    "(Ljava/lang/String;Lorg/kwis/msp/lcdui/Image;Lorg/kwis/msp/lcdui/Image;Ljava/lang/Object;)V",
                    (text.clone(), image.clone(), active.clone(), payload.clone()),
                )
                .await?;
            for (method, desc, expected) in [
                ("getString", "()Ljava/lang/String;", text.identity()),
                ("getExtObject", "()Ljava/lang/Object;", payload.identity()),
                ("getNormalImage", "()Lorg/kwis/msp/lcdui/Image;", image.identity()),
                ("getActiveImage", "()Lorg/kwis/msp/lcdui/Image;", active.identity()),
            ] {
                let result: ClassInstanceRef<()> = jvm.invoke_virtual(&command, NAME, method, desc, ()).await?;
                assert_eq!(result.identity(), expected);
            }
            let plain = jvm
                .new_class(
                    NAME,
                    "(Ljava/lang/String;Ljava/lang/Object;)V",
                    (ClassInstanceRef::<()>::new(None), ClassInstanceRef::<()>::new(None)),
                )
                .await?;
            for (method, desc) in [
                ("getString", "()Ljava/lang/String;"),
                ("getExtObject", "()Ljava/lang/Object;"),
                ("getNormalImage", "()Lorg/kwis/msp/lcdui/Image;"),
                ("getActiveImage", "()Lorg/kwis/msp/lcdui/Image;"),
            ] {
                let result: ClassInstanceRef<()> = jvm.invoke_virtual(&plain, NAME, method, desc, ()).await?;
                assert!(result.is_null());
            }
            let single = jvm
                .new_class(
                    NAME,
                    "(Ljava/lang/String;Lorg/kwis/msp/lcdui/Image;Ljava/lang/Object;)V",
                    (text, image.clone(), payload),
                )
                .await?;
            let result: ClassInstanceRef<()> = jvm
                .invoke_virtual(&single, NAME, "getActiveImage", "()Lorg/kwis/msp/lcdui/Image;", ())
                .await?;
            assert_eq!(result.identity(), image.identity());
            Ok(())
        })
    }

    // A loader double tests Command's lazy dispatch independently of the currently
    // incomplete asynchronous Image.loadImage backend.
    async fn test_load(jvm: &Jvm, _: &mut WieJvmContext, path: ClassInstanceRef<()>, observer: ClassInstanceRef<()>) -> Result<ClassInstanceRef<()>> {
        assert!(observer.is_null());
        let calls: i32 = jvm.get_static_field("org/kwis/msp/lcdui/Image", "calls", "I").await?;
        jvm.put_static_field("org/kwis/msp/lcdui/Image", "calls", "I", calls + 1).await?;
        let mut image = jvm.instantiate_class("org/kwis/msp/lcdui/Image").await?;
        jvm.put_field(&mut image, "path", "Ljava/lang/String;", path).await?;
        Ok(image.into())
    }
    #[test]
    fn resource_icons_are_lazy_cached_and_independent() -> wie_util::Result<()> {
        let mut protos = crate::get_protos().into_iter().collect::<alloc::vec::Vec<_>>();
        protos.retain(|p| p.name != "org/kwis/msp/lcdui/Image");
        protos.push(WieJavaClassProto {
            name: "org/kwis/msp/lcdui/Image",
            parent_class: Some("java/lang/Object"),
            interfaces: vec![],
            methods: vec![JavaMethodProto::new(
                "loadImage",
                "(Ljava/lang/String;Lorg/kwis/msp/lcdui/ImageObserver;)Lorg/kwis/msp/lcdui/Image;",
                test_load,
                MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC,
            )],
            fields: vec![
                JavaFieldProto::new("calls", "I", FieldAccessFlags::STATIC),
                JavaFieldProto::new("path", "Ljava/lang/String;", FieldAccessFlags::PUBLIC),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
        });
        run_jvm_test(Box::new([wie_midp::get_protos().into(), protos.into_boxed_slice()]), |jvm| async move {
            let normal = JavaLangString::from_rust_string(&jvm, "normal.png").await?;
            let active = JavaLangString::from_rust_string(&jvm, "active.png").await?;
            let command = jvm
                .new_class(
                    NAME,
                    "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/Object;)V",
                    (
                        ClassInstanceRef::<()>::new(None),
                        normal.clone(),
                        active.clone(),
                        ClassInstanceRef::<()>::new(None),
                    ),
                )
                .await?;
            assert_eq!(jvm.get_static_field::<i32>("org/kwis/msp/lcdui/Image", "calls", "I").await?, 0);
            for (method, expected, calls) in [
                ("getActiveImage", active.clone(), 1),
                ("getActiveImage", active, 1),
                ("getNormalImage", normal.clone(), 2),
                ("getNormalImage", normal.clone(), 2),
            ] {
                let image: ClassInstanceRef<()> = jvm.invoke_virtual(&command, NAME, method, "()Lorg/kwis/msp/lcdui/Image;", ()).await?;
                let path: ClassInstanceRef<()> = jvm.get_field(&image, "path", "Ljava/lang/String;").await?;
                assert_eq!(path.identity(), expected.identity());
                assert_eq!(jvm.get_static_field::<i32>("org/kwis/msp/lcdui/Image", "calls", "I").await?, calls);
            }
            let single = jvm
                .new_class(
                    NAME,
                    "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/Object;)V",
                    (ClassInstanceRef::<()>::new(None), normal, ClassInstanceRef::<()>::new(None)),
                )
                .await?;
            let a: ClassInstanceRef<()> = jvm
                .invoke_virtual(&single, NAME, "getActiveImage", "()Lorg/kwis/msp/lcdui/Image;", ())
                .await?;
            let b: ClassInstanceRef<()> = jvm
                .invoke_virtual(&single, NAME, "getNormalImage", "()Lorg/kwis/msp/lcdui/Image;", ())
                .await?;
            assert_eq!(a.identity(), b.identity());
            assert_eq!(jvm.get_static_field::<i32>("org/kwis/msp/lcdui/Image", "calls", "I").await?, 3);
            Ok(())
        })
    }
}
