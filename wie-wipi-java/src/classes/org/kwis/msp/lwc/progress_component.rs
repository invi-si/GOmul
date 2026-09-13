use alloc::vec;
use jvm::{ClassInstanceRef, Jvm, Result};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

pub struct ProgressComponent;
pub struct ChangeListener;
const NAME: &str = "org/kwis/msp/lwc/ProgressComponent";
const COMPONENT: &str = "org/kwis/msp/lwc/Component";
const GRAPHICS: &str = "org/kwis/msp/lcdui/Graphics";
const LISTENER: &str = "Lorg/kwis/msp/lwc/ChangeListener;";

impl ChangeListener {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "org/kwis/msp/lwc/ChangeListener",
            parent_class: None,
            interfaces: vec![],
            methods: vec![JavaMethodProto::new_abstract(
                "changed",
                "(Lorg/kwis/msp/lwc/Component;Ljava/lang/Object;)V",
                MethodAccessFlags::PUBLIC | MethodAccessFlags::ABSTRACT,
            )],
            fields: vec![],
            access_flags: ClassAccessFlags::PUBLIC | ClassAccessFlags::INTERFACE | ClassAccessFlags::ABSTRACT,
        }
    }
}

impl ProgressComponent {
    pub fn as_proto() -> WieJavaClassProto {
        let mut fields = vec![];
        for field in ["value", "maximum", "step", "topMargin", "bottomMargin"] {
            fields.push(JavaFieldProto::new(field, "I", FieldAccessFlags::PRIVATE));
        }
        fields.push(JavaFieldProto::new("listener", LISTENER, FieldAccessFlags::PRIVATE));
        fields.push(JavaFieldProto::new("argument", "Ljava/lang/Object;", FieldAccessFlags::PRIVATE));
        WieJavaClassProto {
            name: NAME,
            parent_class: Some(COMPONENT),
            interfaces: vec![],
            fields,
            access_flags: ClassAccessFlags::PUBLIC,
            methods: vec![
                JavaMethodProto::new("<init>", "(ZI)V", Self::init, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setValue", "(I)I", Self::set_value, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getValue", "()I", Self::value, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setStep", "(I)V", Self::set_step, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getStep", "()I", Self::step, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setMaxValue", "(I)V", Self::set_maximum, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getMaxValue", "()I", Self::maximum, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setMargin", "(II)V", Self::margin, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "setChangeListener",
                    "(Lorg/kwis/msp/lwc/ChangeListener;Ljava/lang/Object;)V",
                    Self::listener,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("keyNotify", "(II)Z", Self::key, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("calcPreferredSize", "(I)V", Self::preferred, MethodAccessFlags::PROTECTED),
                JavaMethodProto::new("paintContent", "(Lorg/kwis/msp/lcdui/Graphics;)V", Self::paint, MethodAccessFlags::PUBLIC),
            ],
        }
    }
    async fn init(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, interactive: bool, maximum: i32) -> Result<()> {
        if maximum <= 0 {
            return Err(jvm.exception("java/lang/IllegalArgumentException", "maximum must be positive").await);
        }
        let _: () = jvm.invoke_special(&this, COMPONENT, "<init>", "()V", ()).await?;
        jvm.put_field(&mut this, "wieInput", "Z", interactive).await?;
        jvm.put_field(&mut this, "maximum", "I", maximum).await?;
        jvm.put_field(&mut this, "step", "I", 1).await
    }
    async fn value(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<i32> {
        jvm.get_field(&this, "value", "I").await
    }
    async fn step(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<i32> {
        jvm.get_field(&this, "step", "I").await
    }
    async fn maximum(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<i32> {
        jvm.get_field(&this, "maximum", "I").await
    }
    async fn set_value(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, value: i32) -> Result<i32> {
        let step: i32 = jvm.get_field(&this, "step", "I").await?;
        let maximum: i32 = jvm.get_field(&this, "maximum", "I").await?;
        let value = if value > maximum { maximum } else { value.max(0) - value.max(0) % step };
        jvm.put_field(&mut this, "value", "I", value).await?;
        let _: () = jvm.invoke_virtual(&this, COMPONENT, "repaint", "()V", ()).await?;
        Ok(value)
    }
    async fn set_step(jvm: &Jvm, ctx: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, step: i32) -> Result<()> {
        let maximum: i32 = jvm.get_field(&this, "maximum", "I").await?;
        if step <= 0 || step > maximum {
            return Err(jvm.exception("java/lang/IllegalArgumentException", "invalid progress step").await);
        }
        jvm.put_field(&mut this, "step", "I", step).await?;
        let value: i32 = jvm.get_field(&this, "value", "I").await?;
        Self::set_value(jvm, ctx, this, value - value % step).await?;
        Ok(())
    }
    async fn set_maximum(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, maximum: i32) -> Result<()> {
        if maximum <= 0 {
            return Err(jvm.exception("java/lang/IllegalArgumentException", "maximum must be positive").await);
        }
        let value: i32 = jvm.get_field(&this, "value", "I").await?;
        jvm.put_field(&mut this, "maximum", "I", maximum).await?;
        jvm.put_field(&mut this, "value", "I", value.min(maximum)).await?;
        jvm.invoke_virtual(&this, COMPONENT, "repaint", "()V", ()).await
    }
    async fn margin(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, top: i32, bottom: i32) -> Result<()> {
        jvm.put_field(&mut this, "topMargin", "I", top.max(0)).await?;
        jvm.put_field(&mut this, "bottomMargin", "I", bottom.max(0)).await?;
        let _: () = jvm.invoke_virtual(&this, COMPONENT, "invalidate", "()V", ()).await?;
        jvm.invoke_virtual(&this, COMPONENT, "repaint", "()V", ()).await
    }
    async fn listener(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        listener: ClassInstanceRef<()>,
        argument: ClassInstanceRef<()>,
    ) -> Result<()> {
        jvm.put_field(&mut this, "listener", LISTENER, listener).await?;
        jvm.put_field(&mut this, "argument", "Ljava/lang/Object;", argument).await
    }
    async fn key(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>, kind: i32, key: i32) -> Result<bool> {
        if !jvm.get_field::<bool>(&this, "wieInput", "Z").await? || !matches!(key, -4..=-1) {
            return Ok(false);
        }
        if matches!(kind, 1 | 3) {
            let old: i32 = jvm.get_field(&this, "value", "I").await?;
            let step: i32 = jvm.get_field(&this, "step", "I").await?;
            let next = if matches!(key, -1 | -4) {
                old.saturating_add(step)
            } else {
                old.saturating_sub(step)
            };
            let next = Self::set_value(jvm, ctx, this.clone(), next).await?;
            let listener: ClassInstanceRef<()> = jvm.get_field(&this, "listener", LISTENER).await?;
            if next != old && !listener.is_null() {
                let argument: ClassInstanceRef<()> = jvm.get_field(&this, "argument", "Ljava/lang/Object;").await?;
                let _: () = jvm
                    .invoke_virtual(
                        &listener,
                        "org/kwis/msp/lwc/ChangeListener",
                        "changed",
                        "(Lorg/kwis/msp/lwc/Component;Ljava/lang/Object;)V",
                        (this, argument),
                    )
                    .await?;
            }
        }
        Ok(true)
    }
    async fn preferred(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, width: i32) -> Result<()> {
        let top: i32 = jvm.get_field(&this, "topMargin", "I").await?;
        let bottom: i32 = jvm.get_field(&this, "bottomMargin", "I").await?;
        jvm.put_field(&mut this, "prefW", "I", if width > 0 { width } else { 80 }).await?;
        jvm.put_field(&mut this, "prefH", "I", 12i32.saturating_add(top).saturating_add(bottom))
            .await
    }
    async fn paint(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, g: ClassInstanceRef<()>) -> Result<()> {
        let _: () = jvm
            .invoke_special(&this, COMPONENT, "paintContent", "(Lorg/kwis/msp/lcdui/Graphics;)V", (g.clone(),))
            .await?;
        let width = jvm.get_field::<i32>(&this, "w", "I").await?.max(0);
        let height = jvm.get_field::<i32>(&this, "h", "I").await?.max(0);
        let top = jvm.get_field::<i32>(&this, "topMargin", "I").await?.min(height);
        let bottom: i32 = jvm.get_field(&this, "bottomMargin", "I").await?;
        let height = height.saturating_sub(top).saturating_sub(bottom).max(0);
        if width == 0 || height == 0 {
            return Ok(());
        }
        let value: i32 = jvm.get_field(&this, "value", "I").await?;
        let maximum: i32 = jvm.get_field(&this, "maximum", "I").await?;
        let foreground: i32 = jvm.get_field(&this, "fg", "I").await?;
        let _: () = jvm.invoke_virtual(&g, GRAPHICS, "setColor", "(I)V", (0xcccccc,)).await?;
        let _: () = jvm.invoke_virtual(&g, GRAPHICS, "fillRect", "(IIII)V", (0, top, width, height)).await?;
        let _: () = jvm.invoke_virtual(&g, GRAPHICS, "setColor", "(I)V", (foreground,)).await?;
        let filled = (width as i64 * value as i64 / maximum as i64) as i32;
        jvm.invoke_virtual(&g, GRAPHICS, "fillRect", "(IIII)V", (0, top, filled, height)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;
    use test_utils::run_jvm_test;

    #[test]
    fn progress_bounds_step_and_input_follow_the_component_contract() -> wie_util::Result<()> {
        run_jvm_test(Box::new([wie_midp::get_protos().into(), crate::get_protos().into()]), |jvm| async move {
            let p = jvm.new_class(NAME, "(ZI)V", (true, 13)).await?;
            assert!(jvm.new_class(NAME, "(ZI)V", (false, 0)).await.is_err());
            assert_eq!(jvm.invoke_virtual::<_, i32>(&p, NAME, "getStep", "()I", ()).await?, 1);
            for bad in [-1, 0, 14] {
                assert!(jvm.invoke_virtual::<_, ()>(&p, NAME, "setStep", "(I)V", (bad,)).await.is_err());
            }
            let _: () = jvm.invoke_virtual(&p, NAME, "setStep", "(I)V", (4,)).await?;
            for (input, expected) in [(-1, 0), (7, 4), (13, 12), (14, 13)] {
                assert_eq!(jvm.invoke_virtual::<_, i32>(&p, NAME, "setValue", "(I)I", (input,)).await?, expected);
            }
            let _: () = jvm.invoke_virtual(&p, NAME, "setMaxValue", "(I)V", (8,)).await?;
            assert_eq!(jvm.invoke_virtual::<_, i32>(&p, NAME, "getValue", "()I", ()).await?, 8);
            assert!(jvm.invoke_virtual::<_, ()>(&p, NAME, "setMaxValue", "(I)V", (0,)).await.is_err());
            assert!(jvm.invoke_virtual::<_, bool>(&p, NAME, "keyNotify", "(II)Z", (1, -3)).await?);
            assert_eq!(jvm.invoke_virtual::<_, i32>(&p, NAME, "getValue", "()I", ()).await?, 4);
            let _: bool = jvm.invoke_virtual(&p, NAME, "keyNotify", "(II)Z", (2, -3)).await?;
            assert_eq!(jvm.invoke_virtual::<_, i32>(&p, NAME, "getValue", "()I", ()).await?, 4);
            let passive = jvm.new_class(NAME, "(ZI)V", (false, 10)).await?;
            assert!(!jvm.invoke_virtual::<_, bool>(&passive, NAME, "keyNotify", "(II)Z", (1, -4)).await?);
            Ok(())
        })
    }

    #[test]
    fn progress_paint_erases_previous_fill_and_preserves_margins() -> wie_util::Result<()> {
        run_jvm_test(Box::new([wie_midp::get_protos().into(), crate::get_protos().into()]), |jvm| async move {
            let mut p = jvm.new_class(NAME, "(ZI)V", (false, 100)).await?;
            // Containers assign these guest-backed layout fields before paint.
            jvm.put_field(&mut p, "w", "I", 20).await?;
            jvm.put_field(&mut p, "h", "I", 12).await?;
            let _: () = jvm.invoke_virtual(&p, COMPONENT, "setForeground", "(I)V", (0x123456,)).await?;
            let _: () = jvm.invoke_virtual(&p, NAME, "setMargin", "(II)V", (2, 2)).await?;
            let image: ClassInstanceRef<()> = jvm
                .invoke_static("org/kwis/msp/lcdui/Image", "createImage", "(II)Lorg/kwis/msp/lcdui/Image;", (20, 12))
                .await?;
            let g: ClassInstanceRef<()> = jvm
                .invoke_virtual(&image, "org/kwis/msp/lcdui/Image", "getGraphics", "()Lorg/kwis/msp/lcdui/Graphics;", ())
                .await?;
            let margin: i32 = jvm.invoke_virtual(&g, GRAPHICS, "getPixel", "(II)I", (0, 0)).await?;
            for value in [100, 50] {
                let _: i32 = jvm.invoke_virtual(&p, NAME, "setValue", "(I)I", (value,)).await?;
                let _: () = jvm
                    .invoke_virtual(&p, NAME, "paintContent", "(Lorg/kwis/msp/lcdui/Graphics;)V", (g.clone(),))
                    .await?;
            }
            assert_eq!(jvm.invoke_virtual::<_, i32>(&g, GRAPHICS, "getPixel", "(II)I", (1, 5)).await?, 0x123456);
            assert_eq!(jvm.invoke_virtual::<_, i32>(&g, GRAPHICS, "getPixel", "(II)I", (18, 5)).await?, 0xcccccc);
            assert_eq!(jvm.invoke_virtual::<_, i32>(&g, GRAPHICS, "getPixel", "(II)I", (0, 0)).await?, margin);
            Ok(())
        })
    }
}
