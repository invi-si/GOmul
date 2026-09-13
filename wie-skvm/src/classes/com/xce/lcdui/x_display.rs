use alloc::vec;

use jvm::{ClassInstanceRef, Jvm, Result as JvmResult};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};

use wie_jvm_support::{WieJavaClassProto, WieJvmContext};
use wie_midp::classes::javax::microedition::lcdui::{Graphics, Image};

// class com.xce.lcdui.XDisplay
pub struct XDisplay;

impl XDisplay {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "com/xce/lcdui/XDisplay",
            parent_class: Some("java/lang/Object"),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("<clinit>", "()V", Self::cl_init, MethodAccessFlags::STATIC),
                JavaMethodProto::new("refresh", "(IIII)V", Self::refresh, MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC),
                JavaMethodProto::new(
                    "copyLCD",
                    "(Ljavax/microedition/lcdui/Graphics;Ljavax/microedition/lcdui/Image;IIII)V",
                    Self::copy_lcd,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC,
                ),
            ],
            fields: vec![
                JavaFieldProto::new("width", "I", FieldAccessFlags::PUBLIC | FieldAccessFlags::STATIC),
                JavaFieldProto::new("height", "I", FieldAccessFlags::PUBLIC | FieldAccessFlags::STATIC),
                JavaFieldProto::new("height2", "I", FieldAccessFlags::PUBLIC | FieldAccessFlags::STATIC),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }

    async fn cl_init(jvm: &Jvm, _: &mut WieJvmContext) -> JvmResult<()> {
        tracing::debug!("com.xce.lcdui.XDisplay::<clinit>()");

        // TODO: temp
        jvm.put_static_field("com/xce/lcdui/XDisplay", "width", "I", 240).await?;
        jvm.put_static_field("com/xce/lcdui/XDisplay", "height", "I", 320).await?;
        jvm.put_static_field("com/xce/lcdui/XDisplay", "height2", "I", 320).await?;

        Ok(())
    }

    async fn refresh(jvm: &Jvm, context: &mut WieJvmContext, x: i32, y: i32, width: i32, height: i32) -> JvmResult<()> {
        tracing::debug!("com.xce.lcdui.XDisplay::refresh({x}, {y}, {width}, {height})");
        if width <= 0 || height <= 0 {
            return Ok(());
        }
        let midlet: ClassInstanceRef<()> = jvm
            .get_static_field("javax/microedition/midlet/MIDlet", "currentMIDlet", "Ljavax/microedition/midlet/MIDlet;")
            .await?;
        if !midlet.is_null() {
            let display: ClassInstanceRef<()> = jvm
                .invoke_static(
                    "javax/microedition/lcdui/Display",
                    "getDisplay",
                    "(Ljavax/microedition/midlet/MIDlet;)Ljavax/microedition/lcdui/Display;",
                    (midlet,),
                )
                .await?;
            let image: ClassInstanceRef<Image> = jvm.get_field(&display, "screenImage", "Ljavax/microedition/lcdui/Image;").await?;
            let image = Image::image(jvm, &image).await?;
            // Full-screen refresh presents the guest LCD buffer without another guest paint callback.
            if x <= 0
                && y <= 0
                && i64::from(x) + i64::from(width) >= i64::from(image.width())
                && i64::from(y) + i64::from(height) >= i64::from(image.height())
            {
                context.system().platform().screen().paint(&*image);
            }
        }
        let platform = context.system().platform();
        let screen = platform.screen();
        screen.request_redraw().unwrap();

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn copy_lcd(
        _jvm: &Jvm,
        _context: &mut WieJvmContext,
        graphics: ClassInstanceRef<Graphics>,
        image: ClassInstanceRef<Image>,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> JvmResult<()> {
        tracing::warn!("stub com.xce.lcdui.XDisplay::copyLCD({graphics:?}, {image:?}, {x}, {y}, {width}, {height})",);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{boxed::Box, sync::Arc};
    use core::sync::atomic::{AtomicU32, Ordering};
    use test_utils::{TestPlatform, run_jvm_test_with_system};
    async fn init_midlet(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<()>) -> JvmResult<()> {
        jvm.invoke_special(&this, "javax/microedition/midlet/MIDlet", "<init>", "()V", ()).await
    }
    #[test]
    fn fullscreen_refresh_presents_buffered_pixels() {
        let painted = Arc::new(AtomicU32::new(0));
        let result = painted.clone();
        let platform = TestPlatform::new().with_paint_handler(move |image| {
            let c = image.get_pixel(0, 0);
            result.store(((c.r as u32) << 16) | ((c.g as u32) << 8) | c.b as u32, Ordering::Relaxed);
        });
        run_jvm_test_with_system(
            Box::new([
                wie_midp::get_protos().into(),
                [
                    XDisplay::as_proto(),
                    WieJavaClassProto {
                        name: "TestRefreshMIDlet",
                        parent_class: Some("javax/microedition/midlet/MIDlet"),
                        interfaces: vec![],
                        fields: vec![],
                        access_flags: ClassAccessFlags::PUBLIC,
                        methods: vec![JavaMethodProto::new("<init>", "()V", init_midlet, MethodAccessFlags::PUBLIC)],
                    },
                ]
                .into(),
            ]),
            Box::new(platform),
            move |jvm, _| async move {
                let midlet: ClassInstanceRef<()> = jvm.new_class("TestRefreshMIDlet", "()V", ()).await?.into();
                let display: ClassInstanceRef<()> = jvm.get_field(&midlet, "display", "Ljavax/microedition/lcdui/Display;").await?;
                let image: ClassInstanceRef<Image> = jvm.get_field(&display, "screenImage", "Ljavax/microedition/lcdui/Image;").await?;
                let graphics: ClassInstanceRef<Graphics> = jvm
                    .invoke_virtual(
                        &image,
                        "javax/microedition/lcdui/Image",
                        "getGraphics",
                        "()Ljavax/microedition/lcdui/Graphics;",
                        (),
                    )
                    .await?;
                let _: () = jvm
                    .invoke_virtual(&graphics, "javax/microedition/lcdui/Graphics", "setColor", "(I)V", (0x123456,))
                    .await?;
                let _: () = jvm
                    .invoke_virtual(&graphics, "javax/microedition/lcdui/Graphics", "fillRect", "(IIII)V", (0, 0, 1, 1))
                    .await?;
                let _: () = jvm
                    .invoke_static("com/xce/lcdui/XDisplay", "refresh", "(IIII)V", (0, 0, 320, 240))
                    .await?;
                assert_eq!(painted.load(Ordering::Relaxed), 0x123456);
                Ok(())
            },
        )
        .unwrap();
    }
}
