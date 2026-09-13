use alloc::vec;

use jvm::{ClassInstanceRef, Jvm, Result as JvmResult};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};

use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

// class org.kwis.msp.lwc.Component
pub struct Component;

impl Component {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "org/kwis/msp/lwc/Component",
            parent_class: Some("java/lang/Object"),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("canHandleInput", "()Z", Self::can_input, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("hasFocus", "()Z", Self::has_focus, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getPreferredWidth", "()I", Self::preferred_width, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getPreferredHeight", "()I", Self::preferred_height, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getPreferredHeight", "(I)I", Self::preferred_height_for_width, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("calcPreferredSize", "(I)V", Self::calc_preferred, MethodAccessFlags::PROTECTED),
                JavaMethodProto::new("validate", "()V", Self::validate, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("invalidate", "()V", Self::invalidate, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("isValid", "()Z", Self::is_valid, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("layout", "()V", Self::layout, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "paintContent",
                    "(Lorg/kwis/msp/lcdui/Graphics;)V",
                    Self::paint_content,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("setForeground", "(I)V", Self::set_foreground, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getForeground", "()I", Self::get_foreground, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setBackground", "(I)V", Self::set_background, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getBackground", "()I", Self::get_background, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("<init>", "()V", Self::init, MethodAccessFlags::PROTECTED),
                JavaMethodProto::new("keyNotify", "(II)Z", Self::key_notify, MethodAccessFlags::PROTECTED),
                JavaMethodProto::new("focusNotify", "(Z)V", Self::focus_notify, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("showNotify", "(Z)V", Self::show_notify, MethodAccessFlags::PROTECTED),
                JavaMethodProto::new("configure", "(IIIII)V", Self::configure, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setFocus", "()V", Self::set_focus, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getHeight", "()I", Self::get_height, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getX", "()I", Self::get_x, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getY", "()I", Self::get_y, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getWidth", "()I", Self::get_width, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getXOnScreen", "()I", Self::get_x_on_screen, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getYOnScreen", "()I", Self::get_y_on_screen, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getCard", "()Lorg/kwis/msp/lcdui/Card;", Self::get_card, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("repaint", "()V", Self::repaint, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("serviceRepaints", "()V", Self::service_repaints, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("isShown", "()Z", Self::is_shown, MethodAccessFlags::PUBLIC),
            ],
            fields: vec![
                JavaFieldProto::new("wieInput", "Z", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("wieFocused", "Z", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("prefW", "I", FieldAccessFlags::PROTECTED),
                JavaFieldProto::new("prefH", "I", FieldAccessFlags::PROTECTED),
                JavaFieldProto::new("wieValid", "Z", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("fg", "I", FieldAccessFlags::PROTECTED),
                JavaFieldProto::new("bg", "I", FieldAccessFlags::PROTECTED),
                JavaFieldProto::new("x", "I", FieldAccessFlags::PROTECTED),
                JavaFieldProto::new("y", "I", FieldAccessFlags::PROTECTED),
                JavaFieldProto::new("w", "I", FieldAccessFlags::PROTECTED),
                JavaFieldProto::new("h", "I", FieldAccessFlags::PROTECTED),
                JavaFieldProto::new("parent", "Lorg/kwis/msp/lwc/ContainerComponent;", FieldAccessFlags::PROTECTED),
            ],
            access_flags: ClassAccessFlags::PUBLIC | ClassAccessFlags::ABSTRACT,
        }
    }

    async fn is_valid(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<bool> {
        jvm.get_field(&this, "wieValid", "Z").await
    }
    async fn validate(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>) -> JvmResult<()> {
        let valid: bool = jvm.get_field(&this, "wieValid", "Z").await?;
        if !valid {
            let _: () = jvm.invoke_virtual(&this, "org/kwis/msp/lwc/Component", "layout", "()V", ()).await?;
            jvm.put_field(&mut this, "wieValid", "Z", true).await?;
        }
        Ok(())
    }
    async fn invalidate(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>) -> JvmResult<()> {
        jvm.put_field(&mut this, "wieValid", "Z", false).await?;
        let parent: ClassInstanceRef<()> = jvm.get_field(&this, "parent", "Lorg/kwis/msp/lwc/ContainerComponent;").await?;
        if !parent.is_null() {
            let _: () = jvm.invoke_virtual(&parent, "org/kwis/msp/lwc/Component", "invalidate", "()V", ()).await?;
        }
        Ok(())
    }
    async fn layout(_: &Jvm, _: &mut WieJvmContext, _: ClassInstanceRef<Self>) -> JvmResult<()> {
        Ok(())
    }
    async fn paint_content(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, g: ClassInstanceRef<()>) -> JvmResult<()> {
        let _: () = jvm.invoke_virtual(&this, "org/kwis/msp/lwc/Component", "validate", "()V", ()).await?;
        let bg: i32 = jvm.get_field(&this, "bg", "I").await?;
        if bg != -1 {
            let width: i32 = jvm.get_field(&this, "w", "I").await?;
            let height: i32 = jvm.get_field(&this, "h", "I").await?;
            let _: () = jvm.invoke_virtual(&g, "org/kwis/msp/lcdui/Graphics", "setColor", "(I)V", (bg,)).await?;
            let _: () = jvm
                .invoke_virtual(&g, "org/kwis/msp/lcdui/Graphics", "fillRect", "(IIII)V", (0, 0, width, height))
                .await?;
        }
        Ok(())
    }

    async fn get_foreground(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        jvm.get_field(&this, "fg", "I").await
    }
    async fn set_foreground(jvm: &Jvm, context: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, color: i32) -> JvmResult<()> {
        jvm.put_field(&mut this, "fg", "I", color).await?;
        Self::repaint(jvm, context, this).await
    }

    async fn get_background(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        jvm.get_field(&this, "bg", "I").await
    }
    async fn set_background(jvm: &Jvm, context: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, color: i32) -> JvmResult<()> {
        jvm.put_field(&mut this, "bg", "I", color).await?;
        Self::repaint(jvm, context, this).await
    }
    async fn init(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>) -> JvmResult<()> {
        tracing::debug!("stub org.kwis.msp.lwc.Component::<init>({this:?})");

        let _: () = jvm.invoke_special(&this, "java/lang/Object", "<init>", "()V", ()).await?;

        jvm.put_field(&mut this, "bg", "I", -1).await?;
        Ok(())
    }

    async fn can_input(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<bool> {
        jvm.get_field(&this, "wieInput", "Z").await
    }
    async fn has_focus(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<bool> {
        jvm.get_field(&this, "wieFocused", "Z").await
    }
    async fn key_notify(_: &Jvm, _: &mut WieJvmContext, _: ClassInstanceRef<Self>, _: i32, _: i32) -> JvmResult<bool> {
        Ok(false)
    }
    async fn focus_notify(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, focused: bool) -> JvmResult<()> {
        jvm.put_field(&mut this, "wieFocused", "Z", focused).await?;
        jvm.invoke_virtual(&this, "org/kwis/msp/lwc/Component", "repaint", "()V", ()).await
    }
    async fn calc_preferred(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, _: i32) -> JvmResult<()> {
        for (src, dst) in [("w", "prefW"), ("h", "prefH")] {
            let value: i32 = jvm.get_field(&this, src, "I").await?;
            jvm.put_field(&mut this, dst, "I", value.max(0)).await?;
        }
        Ok(())
    }
    async fn preferred_width(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        let _: () = jvm
            .invoke_virtual(&this, "org/kwis/msp/lwc/Component", "calcPreferredSize", "(I)V", (-1,))
            .await?;
        jvm.get_field(&this, "prefW", "I").await
    }
    async fn preferred_height(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        Self::preferred_height_for_width(jvm, ctx, this, -1).await
    }
    async fn preferred_height_for_width(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, width: i32) -> JvmResult<i32> {
        let _: () = jvm
            .invoke_virtual(&this, "org/kwis/msp/lwc/Component", "calcPreferredSize", "(I)V", (width,))
            .await?;
        jvm.get_field(&this, "prefH", "I").await
    }

    async fn show_notify(_: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, show: bool) -> JvmResult<()> {
        tracing::warn!("stub org.kwis.msp.lwc.Component::showNotify({this:?}, {show:?})");

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn configure(_: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, x: i32, y: i32, w: i32, h: i32, mask: i32) -> JvmResult<()> {
        tracing::warn!("stub org.kwis.msp.lwc.Component::configure({this:?}, {x}, {y}, {w}, {h}, {mask})");

        Ok(())
    }

    async fn set_focus(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<()> {
        let parent: ClassInstanceRef<()> = jvm.get_field(&this, "parent", "Lorg/kwis/msp/lwc/ContainerComponent;").await?;
        if !parent.is_null() {
            let _: () = jvm
                .invoke_virtual(
                    &parent,
                    "org/kwis/msp/lwc/ContainerComponent",
                    "setFocus",
                    "(Lorg/kwis/msp/lwc/Component;)V",
                    (this,),
                )
                .await?;
        }
        Ok(())
    }

    async fn get_x(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        jvm.get_field(&this, "x", "I").await
    }
    async fn get_y(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        jvm.get_field(&this, "y", "I").await
    }
    async fn get_width(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        jvm.get_field(&this, "w", "I").await
    }
    async fn get_height(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        jvm.get_field(&this, "h", "I").await
    }
    async fn get_card(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<ClassInstanceRef<()>> {
        let parent: ClassInstanceRef<()> = jvm.get_field(&this, "parent", "Lorg/kwis/msp/lwc/ContainerComponent;").await?;
        if parent.is_null() {
            return Ok(None.into());
        }
        jvm.invoke_virtual(&parent, "org/kwis/msp/lwc/Component", "getCard", "()Lorg/kwis/msp/lcdui/Card;", ())
            .await
    }
    async fn repaint(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<()> {
        let card: ClassInstanceRef<()> = jvm
            .invoke_virtual(&this, "org/kwis/msp/lwc/Component", "getCard", "()Lorg/kwis/msp/lcdui/Card;", ())
            .await?;
        if !card.is_null() {
            let _: () = jvm.invoke_virtual(&card, "org/kwis/msp/lcdui/Card", "repaint", "()V", ()).await?;
        }
        Ok(())
    }
    async fn service_repaints(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<()> {
        let card: ClassInstanceRef<()> = jvm
            .invoke_virtual(&this, "org/kwis/msp/lwc/Component", "getCard", "()Lorg/kwis/msp/lcdui/Card;", ())
            .await?;
        if !card.is_null() {
            let _: () = jvm.invoke_virtual(&card, "org/kwis/msp/lcdui/Card", "serviceRepaints", "()V", ()).await?;
        }
        Ok(())
    }
    async fn is_shown(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<bool> {
        let card: ClassInstanceRef<()> = jvm
            .invoke_virtual(&this, "org/kwis/msp/lwc/Component", "getCard", "()Lorg/kwis/msp/lcdui/Card;", ())
            .await?;
        if card.is_null() {
            return Ok(false);
        }
        jvm.invoke_virtual(&card, "org/kwis/msp/lcdui/Card", "isShown", "()Z", ()).await
    }
    async fn get_x_on_screen(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        let position: i32 = jvm.get_field(&this, "x", "I").await?;
        let parent: ClassInstanceRef<()> = jvm.get_field(&this, "parent", "Lorg/kwis/msp/lwc/ContainerComponent;").await?;
        if parent.is_null() {
            return Ok(position);
        }
        let origin: i32 = jvm
            .invoke_virtual(&parent, "org/kwis/msp/lwc/Component", "getXOnScreen", "()I", ())
            .await?;
        Ok(origin
            .wrapping_add(position)
            .wrapping_add(jvm.get_field::<i32>(&parent, "offsetX", "I").await?))
    }
    async fn get_y_on_screen(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        let position: i32 = jvm.get_field(&this, "y", "I").await?;
        let parent: ClassInstanceRef<()> = jvm.get_field(&this, "parent", "Lorg/kwis/msp/lwc/ContainerComponent;").await?;
        if parent.is_null() {
            return Ok(position);
        }
        let origin: i32 = jvm
            .invoke_virtual(&parent, "org/kwis/msp/lwc/Component", "getYOnScreen", "()I", ())
            .await?;
        Ok(origin
            .wrapping_add(position)
            .wrapping_add(jvm.get_field::<i32>(&parent, "offsetY", "I").await?))
    }
}

#[cfg(test)]
mod tests {
    use alloc::{boxed::Box, vec};
    use jvm::{ClassInstanceRef, Jvm, Result as JvmResult};
    use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
    use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
    use test_utils::run_jvm_test;
    use wie_jvm_support::{WieJavaClassProto, WieJvmContext};
    use wie_util::Result;

    async fn repaint(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<()>) -> JvmResult<()> {
        let count: i32 = jvm.get_field(&this, "requests", "I").await?;
        jvm.put_field(&mut this, "requests", "I", count + 1).await
    }

    #[test]
    fn geometry_is_guest_backed_and_repaint_reaches_only_the_owning_card() -> Result<()> {
        let card = WieJavaClassProto {
            name: "test/LwcCard",
            parent_class: Some("org/kwis/msp/lcdui/Card"),
            interfaces: vec![],
            methods: vec![JavaMethodProto::new("repaint", "()V", repaint, MethodAccessFlags::PUBLIC)],
            fields: vec![JavaFieldProto::new("requests", "I", FieldAccessFlags::PUBLIC)],
            access_flags: ClassAccessFlags::PUBLIC,
        };
        run_jvm_test(
            Box::new([wie_midp::get_protos().into(), crate::get_protos().into(), vec![card].into()]),
            |jvm| async move {
                let mut shell = jvm.new_class("org/kwis/msp/lwc/ShellComponent", "(IIII)V", (5, 7, 120, 160)).await?;
                for (method, expected) in [("getX", 5), ("getY", 7), ("getWidth", 120), ("getHeight", 160)] {
                    assert_eq!(
                        jvm.invoke_virtual::<_, i32>(&shell, "org/kwis/msp/lwc/Component", method, "()I", ())
                            .await?,
                        expected
                    );
                }
                let other = jvm.new_class("org/kwis/msp/lwc/ShellComponent", "(IIII)V", (0, 0, 20, 30)).await?;
                let mut child = jvm.new_class("org/kwis/msp/lwc/FormComponent", "()V", ()).await?;
                let _: () = jvm
                    .invoke_virtual(&child, "org/kwis/msp/lwc/ContainerComponent", "repaint", "()V", ())
                    .await?;
                let card = jvm.instantiate_class("test/LwcCard").await?;
                jvm.put_field(&mut shell, "cd", "Lorg/kwis/msp/lcdui/Card;", card.clone()).await?;
                jvm.put_field(&mut child, "parent", "Lorg/kwis/msp/lwc/ContainerComponent;", shell.clone())
                    .await?;
                jvm.put_field(&mut child, "x", "I", 9).await?;
                let found: ClassInstanceRef<()> = jvm
                    .invoke_virtual(&child, "org/kwis/msp/lwc/Component", "getCard", "()Lorg/kwis/msp/lcdui/Card;", ())
                    .await?;
                assert_eq!(found.identity(), card.identity());
                assert_eq!(
                    jvm.invoke_virtual::<_, i32>(&child, "org/kwis/msp/lwc/Component", "getXOnScreen", "()I", ())
                        .await?,
                    14
                );
                let _: () = jvm
                    .invoke_virtual(&child, "org/kwis/msp/lwc/ContainerComponent", "repaint", "()V", ())
                    .await?;
                let _: () = jvm.invoke_virtual(&other, "org/kwis/msp/lwc/Component", "repaint", "()V", ()).await?;
                assert_eq!(jvm.get_field::<i32>(&card, "requests", "I").await?, 1);
                Ok(())
            },
        )
    }
}
