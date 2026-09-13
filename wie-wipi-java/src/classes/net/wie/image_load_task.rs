use crate::classes::org::kwis::msp::lcdui::ImageObserver;
use alloc::vec;
use jvm::{ClassInstanceRef, JavaError, Jvm, Result};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

pub struct ImageLoadTask;
const NAME: &str = "net/wie/ImageLoadTask";
const TASK: &str = "Lnet/wie/ImageLoadTask;";
const IMAGE: &str = "Lorg/kwis/msp/lcdui/Image;";
const OBSERVER: &str = "Lorg/kwis/msp/lcdui/ImageObserver;";
impl ImageLoadTask {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: NAME,
            parent_class: Some("java/lang/Object"),
            interfaces: vec!["java/lang/Runnable"],
            methods: vec![JavaMethodProto::new("run", "()V", Self::run, MethodAccessFlags::PUBLIC)],
            fields: vec![
                JavaFieldProto::new("head", TASK, FieldAccessFlags::PRIVATE | FieldAccessFlags::STATIC),
                JavaFieldProto::new("next", TASK, FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("image", IMAGE, FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("observer", OBSERVER, FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("path", "Ljava/lang/String;", FieldAccessFlags::PRIVATE),
            ],
            access_flags: ClassAccessFlags::PUBLIC | ClassAccessFlags::FINAL,
        }
    }
    pub async fn enqueue(jvm: &Jvm, image: ClassInstanceRef<()>, path: ClassInstanceRef<()>, observer: ClassInstanceRef<()>) -> Result<()> {
        let mut task: ClassInstanceRef<()> = jvm.instantiate_class(NAME).await?.into();
        let _: () = jvm.invoke_special(&task, "java/lang/Object", "<init>", "()V", ()).await?;
        jvm.put_field(&mut task, "image", IMAGE, image).await?;
        jvm.put_field(&mut task, "observer", OBSERVER, observer).await?;
        jvm.put_field(&mut task, "path", "Ljava/lang/String;", path).await?;
        let head: ClassInstanceRef<()> = jvm.get_static_field(NAME, "head", TASK).await?;
        jvm.put_field(&mut task, "next", TASK, head).await?;
        jvm.put_static_field(NAME, "head", TASK, task.clone()).await?;
        let queue: ClassInstanceRef<()> = jvm
            .invoke_static("net/wie/EventQueue", "getEventQueue", "()Lnet/wie/EventQueue;", ())
            .await?;
        let result = jvm
            .invoke_virtual(&queue, "net/wie/EventQueue", "callSerially", "(Ljava/lang/Runnable;)V", (task.clone(),))
            .await;
        if result.is_err() {
            Self::detach(jvm, &task).await?;
            Self::clear(jvm, task).await?;
        }
        result
    }
    async fn detach(jvm: &Jvm, target: &ClassInstanceRef<()>) -> Result<()> {
        let mut current: ClassInstanceRef<()> = jvm.get_static_field(NAME, "head", TASK).await?;
        let mut previous: ClassInstanceRef<()> = None.into();
        while !current.is_null() {
            let next: ClassInstanceRef<()> = jvm.get_field(&current, "next", TASK).await?;
            if current.identity() == target.identity() {
                if previous.is_null() {
                    jvm.put_static_field(NAME, "head", TASK, next).await?;
                } else {
                    jvm.put_field(&mut previous, "next", TASK, next).await?;
                }
                return Ok(());
            }
            previous = current;
            current = next;
        }
        Ok(())
    }
    async fn clear(jvm: &Jvm, mut task: ClassInstanceRef<()>) -> Result<()> {
        for (name, ty) in [("next", TASK), ("image", IMAGE), ("observer", OBSERVER), ("path", "Ljava/lang/String;")] {
            jvm.put_field(&mut task, name, ty, ClassInstanceRef::<()>::from(None)).await?;
        }
        Ok(())
    }
    pub async fn cancel(jvm: &Jvm, observer: ClassInstanceRef<()>) -> Result<()> {
        if observer.is_null() {
            return Ok(());
        }
        let mut current: ClassInstanceRef<()> = jvm.get_static_field(NAME, "head", TASK).await?;
        while !current.is_null() {
            let next: ClassInstanceRef<()> = jvm.get_field(&current, "next", TASK).await?;
            let owner: ClassInstanceRef<()> = jvm.get_field(&current, "observer", OBSERVER).await?;
            if !owner.is_null() && owner.identity() == observer.identity() {
                Self::detach(jvm, &current).await?;
                Self::clear(jvm, current).await?;
            }
            current = next;
        }
        Ok(())
    }
    async fn run(jvm: &Jvm, _: &mut WieJvmContext, task: ClassInstanceRef<Self>) -> Result<()> {
        let task: ClassInstanceRef<()> = jvm::JavaValue::from(task).into();
        let mut image: ClassInstanceRef<()> = jvm.get_field(&task, "image", IMAGE).await?;
        if image.is_null() {
            return Ok(());
        }
        let path: ClassInstanceRef<()> = jvm.get_field(&task, "path", "Ljava/lang/String;").await?;
        let result: Result<ClassInstanceRef<()>> = jvm
            .invoke_static(
                "javax/microedition/lcdui/Image",
                "createImage",
                "(Ljava/lang/String;)Ljavax/microedition/lcdui/Image;",
                (path,),
            )
            .await;
        // stopImage may cancel a suspended load while resource I/O is in progress.
        let still_pending: ClassInstanceRef<()> = jvm.get_field(&task, "image", IMAGE).await?;
        if still_pending.is_null() {
            return Ok(());
        }
        let observer: ClassInstanceRef<()> = jvm.get_field(&task, "observer", OBSERVER).await?;
        Self::detach(jvm, &task).await?;
        Self::clear(jvm, task).await?;
        let status = match result {
            Ok(decoded) => {
                jvm.put_field_in_class(
                    &mut image,
                    "org/kwis/msp/lcdui/Image",
                    "midpImage",
                    "Ljavax/microedition/lcdui/Image;",
                    decoded,
                )
                .await?;
                ImageObserver::IMAGE_END
            }
            Err(JavaError::JavaException(ref exception)) if jvm.is_instance(&**exception, "java/io/FileNotFoundException") => {
                ImageObserver::NOT_EXIST
            }
            Err(JavaError::JavaException(ref exception))
                if jvm.is_instance(&**exception, "java/lang/IllegalArgumentException") || jvm.is_instance(&**exception, "java/io/IOException") =>
            {
                ImageObserver::DECODE_ERROR
            }
            Err(JavaError::JavaException(ref exception)) if jvm.is_instance(&**exception, "java/lang/OutOfMemoryError") => {
                ImageObserver::OUT_OF_MEMORY
            }
            Err(error) => return Err(error),
        };
        if !observer.is_null() {
            let _: () = jvm
                .invoke_virtual(
                    &observer,
                    "org/kwis/msp/lcdui/ImageObserver",
                    "notify",
                    "(Lorg/kwis/msp/lcdui/Image;I)V",
                    (image, status),
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
    use jvm::runtime::JavaLangString;
    const WIPI: &str = "org/kwis/msp/lcdui/Image";
    async fn decoder(jvm: &Jvm, _: &mut WieJvmContext, path: ClassInstanceRef<()>) -> Result<ClassInstanceRef<()>> {
        let path = JavaLangString::to_rust_string(jvm, &path).await?;
        let error = match path.as_str() {
            "missing" => Some("java/io/FileNotFoundException"),
            "invalid" => Some("java/lang/IllegalArgumentException"),
            "oom" => Some("java/lang/OutOfMemoryError"),
            "bug" => Some("java/lang/IllegalStateException"),
            _ => None,
        };
        if let Some(error) = error {
            return Err(jvm.exception(error, "test decoder").await);
        }
        jvm.invoke_static(
            "javax/microedition/lcdui/Image",
            "createImage",
            "(II)Ljavax/microedition/lcdui/Image;",
            (3, 4),
        )
        .await
    }
    async fn notify(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<()>, image: ClassInstanceRef<()>, status: i32) -> Result<()> {
        let count: i32 = jvm.get_field(&this, "count", "I").await?;
        jvm.put_field(&mut this, "count", "I", count + 1).await?;
        jvm.put_field(&mut this, "status", "I", status).await?;
        jvm.put_field(&mut this, "image", IMAGE, image.clone()).await?;
        let width: i32 = jvm.invoke_virtual(&image, WIPI, "getWidth", "()I", ()).await?;
        jvm.put_field(&mut this, "widthAtNotify", "I", width).await?;
        if jvm.get_field::<bool>(&this, "fail", "Z").await? {
            return Err(jvm.exception("java/lang/IllegalStateException", "observer").await);
        }
        Ok(())
    }
    async fn drain_one(jvm: &Jvm) -> Result<ClassInstanceRef<()>> {
        let queue: ClassInstanceRef<()> = jvm
            .invoke_static("net/wie/EventQueue", "getEventQueue", "()Lnet/wie/EventQueue;", ())
            .await?;
        let pending: ClassInstanceRef<()> = jvm.get_field(&queue, "callSeriallyEvents", "Ljava/util/Vector;").await?;
        let task: ClassInstanceRef<()> = jvm
            .invoke_virtual(&pending, "java/util/Vector", "remove", "(I)Ljava/lang/Object;", (0,))
            .await?;
        let _: () = jvm.invoke_virtual(&task, "java/lang/Runnable", "run", "()V", ()).await?;
        Ok(task)
    }
    async fn load(jvm: &Jvm, path: &str, observer: ClassInstanceRef<()>) -> Result<ClassInstanceRef<()>> {
        let path = JavaLangString::from_rust_string(jvm, path).await?;
        jvm.invoke_static(
            WIPI,
            "loadImage",
            "(Ljava/lang/String;Lorg/kwis/msp/lcdui/ImageObserver;)Lorg/kwis/msp/lcdui/Image;",
            (path, observer),
        )
        .await
    }
    #[test]
    fn deferred_images_complete_fail_and_cancel_without_stale_callbacks() -> wie_util::Result<()> {
        // Only the resource decoder is substituted. Real WIPI image objects,
        // event-queue storage, task lifecycle and observer dispatch are exercised.
        let mut midp = wie_midp::get_protos();
        let proto = midp.iter_mut().find(|p| p.name == "javax/microedition/lcdui/Image").unwrap();
        proto
            .methods
            .retain(|m| !(m.name == "createImage" && m.descriptor == "(Ljava/lang/String;)Ljavax/microedition/lcdui/Image;"));
        proto.methods.push(JavaMethodProto::new(
            "createImage",
            "(Ljava/lang/String;)Ljavax/microedition/lcdui/Image;",
            decoder,
            MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC,
        ));
        let observer = WieJavaClassProto {
            name: "test/ImageObserver",
            parent_class: Some("java/lang/Object"),
            interfaces: vec!["org/kwis/msp/lcdui/ImageObserver"],
            methods: vec![JavaMethodProto::new(
                "notify",
                "(Lorg/kwis/msp/lcdui/Image;I)V",
                notify,
                MethodAccessFlags::PUBLIC,
            )],
            fields: vec![
                JavaFieldProto::new("count", "I", FieldAccessFlags::PUBLIC),
                JavaFieldProto::new("status", "I", FieldAccessFlags::PUBLIC),
                JavaFieldProto::new("widthAtNotify", "I", FieldAccessFlags::PUBLIC),
                JavaFieldProto::new("image", IMAGE, FieldAccessFlags::PUBLIC),
                JavaFieldProto::new("fail", "Z", FieldAccessFlags::PUBLIC),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
        };
        test_utils::run_jvm_test(
            Box::new([midp.into(), crate::get_protos().into(), Box::new([observer])]),
            |jvm| async move {
                let mut observer: ClassInstanceRef<()> = jvm.instantiate_class("test/ImageObserver").await?.into();
                let _: () = jvm.invoke_special(&observer, "java/lang/Object", "<init>", "()V", ()).await?;
                let image = load(&jvm, "ok", observer.clone()).await?;
                let canvas: ClassInstanceRef<()> = jvm.invoke_static(WIPI, "createImage", "(II)Lorg/kwis/msp/lcdui/Image;", (2, 2)).await?;
                let graphics: ClassInstanceRef<()> = jvm
                    .invoke_virtual(&canvas, WIPI, "getGraphics", "()Lorg/kwis/msp/lcdui/Graphics;", ())
                    .await?;
                let graphics_class = "org/kwis/msp/lcdui/Graphics";
                let _: () = jvm.invoke_virtual(&graphics, graphics_class, "setColor", "(I)V", (0x123456,)).await?;
                let _: () = jvm.invoke_virtual(&graphics, graphics_class, "fillRect", "(IIII)V", (0, 0, 2, 2)).await?;
                let _: () = jvm
                    .invoke_virtual(
                        &graphics,
                        graphics_class,
                        "drawImage",
                        "(Lorg/kwis/msp/lcdui/Image;III)V",
                        (image.clone(), 0, 0, 20),
                    )
                    .await?;
                let backing = jvm.get_field(&canvas, "midpImage", "Ljavax/microedition/lcdui/Image;").await?;
                let pixels = wie_midp::classes::javax::microedition::lcdui::Image::image(&jvm, &backing).await?;
                let pixel = pixels.get_pixel(0, 0);
                assert_eq!((pixel.r, pixel.g, pixel.b), (0x12, 0x34, 0x56));
                assert_eq!(jvm.invoke_virtual::<_, i32>(&image, WIPI, "getWidth", "()I", ()).await?, 0);
                assert_eq!(jvm.invoke_virtual::<_, i32>(&image, WIPI, "getHeight", "()I", ()).await?, 0);
                assert_eq!(jvm.get_field::<i32>(&observer, "count", "I").await?, 0);
                let task = drain_one(&jvm).await?;
                assert_eq!(jvm.invoke_virtual::<_, i32>(&image, WIPI, "getHeight", "()I", ()).await?, 4);
                assert!(!jvm.invoke_virtual::<_, bool>(&image, WIPI, "isMutable", "()Z", ()).await?);
                assert_eq!(jvm.get_field::<i32>(&observer, "widthAtNotify", "I").await?, 3);
                assert_eq!(jvm.get_field::<i32>(&observer, "status", "I").await?, ImageObserver::IMAGE_END);
                let observed: ClassInstanceRef<()> = jvm.get_field(&observer, "image", IMAGE).await?;
                assert_eq!(observed.identity(), image.identity());
                let _: () = jvm.invoke_virtual(&task, "java/lang/Runnable", "run", "()V", ()).await?;
                assert_eq!(jvm.get_field::<i32>(&observer, "count", "I").await?, 1);
                for (path, expected) in [
                    ("missing", ImageObserver::NOT_EXIST),
                    ("invalid", ImageObserver::DECODE_ERROR),
                    ("oom", ImageObserver::OUT_OF_MEMORY),
                ] {
                    let failed = load(&jvm, path, observer.clone()).await?;
                    drain_one(&jvm).await?;
                    assert_eq!(jvm.get_field::<i32>(&observer, "status", "I").await?, expected);
                    assert_eq!(jvm.invoke_virtual::<_, i32>(&failed, WIPI, "getWidth", "()I", ()).await?, 0);
                }
                let canceled = load(&jvm, "ok", observer.clone()).await?;
                let anonymous = load(&jvm, "ok", None.into()).await?;
                let _ = load(&jvm, "ok", observer.clone()).await?;
                let _: () = jvm
                    .invoke_static(WIPI, "stopImage", "(Lorg/kwis/msp/lcdui/ImageObserver;)V", (observer.clone(),))
                    .await?;
                for _ in 0..3 {
                    drain_one(&jvm).await?;
                }
                assert_eq!(jvm.invoke_virtual::<_, i32>(&canceled, WIPI, "getWidth", "()I", ()).await?, 0);
                assert_eq!(jvm.invoke_virtual::<_, i32>(&anonymous, WIPI, "getWidth", "()I", ()).await?, 3);
                assert_eq!(jvm.get_field::<i32>(&observer, "count", "I").await?, 4);
                let _ = load(&jvm, "bug", observer.clone()).await?;
                assert!(drain_one(&jvm).await.is_err());
                jvm.put_field(&mut observer, "fail", "Z", true).await?;
                let _ = load(&jvm, "ok", observer.clone()).await?;
                assert!(drain_one(&jvm).await.is_err());
                let head: ClassInstanceRef<()> = jvm.get_static_field(NAME, "head", TASK).await?;
                assert!(head.is_null());
                let result: Result<ClassInstanceRef<()>> = jvm
                    .invoke_static(
                        WIPI,
                        "loadImage",
                        "(Ljava/lang/String;Lorg/kwis/msp/lcdui/ImageObserver;)Lorg/kwis/msp/lcdui/Image;",
                        (ClassInstanceRef::<()>::from(None), observer),
                    )
                    .await;
                assert!(matches!(result, Err(JavaError::JavaException(_))));
                Ok(())
            },
        )
    }
}
