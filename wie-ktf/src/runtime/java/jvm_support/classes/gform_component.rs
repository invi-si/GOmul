use alloc::{vec, vec::Vec};
use jvm::{Array, ClassInstanceRef, Jvm, Result};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

pub struct GFormComponent;
const NAME: &str = "com/ktf/kfc/GFormComponent";
const FORM: &str = "org/kwis/msp/lwc/FormComponent";
const CONTAINER: &str = "org/kwis/msp/lwc/ContainerComponent";
const COMPONENT: &str = "org/kwis/msp/lwc/Component";
impl GFormComponent {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: NAME,
            parent_class: Some(FORM),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("<init>", "()V", Self::init, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "addComponent",
                    "(Lorg/kwis/msp/lwc/Component;IIII)I",
                    Self::add_positioned,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new(
                    "addComponent",
                    "(ILorg/kwis/msp/lwc/Component;IIII)V",
                    Self::insert_positioned,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("addComponent", "(Lorg/kwis/msp/lwc/Component;)I", Self::add, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "addComponent",
                    "(ILorg/kwis/msp/lwc/Component;)V",
                    Self::insert,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("addPosData", "(IIIII)V", Self::position, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("addPosData", "(I)V", Self::default_position, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("layout", "()V", Self::layout, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("calcPreferredSize", "(I)V", Self::preferred, MethodAccessFlags::PROTECTED),
            ],
            fields: vec![JavaFieldProto::new("bScroll", "Z", FieldAccessFlags::PROTECTED)],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }
    async fn init(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<()> {
        jvm.invoke_special(&this, FORM, "<init>", "()V", ()).await
    }
    #[allow(clippy::too_many_arguments)]
    async fn insert_positioned(
        jvm: &Jvm,
        ctx: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        index: i32,
        child: ClassInstanceRef<()>,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    ) -> Result<()> {
        let _: () = jvm
            .invoke_special(&this, CONTAINER, "addComponent", "(ILorg/kwis/msp/lwc/Component;)V", (index, child))
            .await?;
        Self::position(jvm, ctx, this, index, x, y, w, h).await
    }
    async fn add_positioned(
        jvm: &Jvm,
        ctx: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        child: ClassInstanceRef<()>,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    ) -> Result<i32> {
        let index: i32 = jvm.get_field(&this, "ncomp", "I").await?;
        Self::insert_positioned(jvm, ctx, this, index, child, x, y, w, h).await?;
        Ok(index)
    }
    async fn add(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>, child: ClassInstanceRef<()>) -> Result<i32> {
        let index: i32 = jvm.get_field(&this, "ncomp", "I").await?;
        Self::insert(jvm, ctx, this, index, child).await?;
        Ok(index)
    }
    async fn insert(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>, index: i32, child: ClassInstanceRef<()>) -> Result<()> {
        let _: () = jvm
            .invoke_special(&this, CONTAINER, "addComponent", "(ILorg/kwis/msp/lwc/Component;)V", (index, child))
            .await?;
        Self::default_position(jvm, ctx, this, index).await
    }
    async fn default_position(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>, index: i32) -> Result<()> {
        let child: ClassInstanceRef<()> = jvm
            .invoke_virtual(&this, CONTAINER, "getComponent", "(I)Lorg/kwis/msp/lwc/Component;", (index,))
            .await?;
        let w: i32 = jvm.invoke_virtual(&child, COMPONENT, "getPreferredWidth", "()I", ()).await?;
        let h: i32 = jvm.invoke_virtual(&child, COMPONENT, "getPreferredHeight", "(I)I", (w,)).await?;
        let y = if index > 0 {
            let previous: ClassInstanceRef<()> = jvm
                .invoke_virtual(&this, CONTAINER, "getComponent", "(I)Lorg/kwis/msp/lwc/Component;", (index - 1,))
                .await?;
            jvm.get_field::<i32>(&previous, "y", "I")
                .await?
                .saturating_add(jvm.get_field::<i32>(&previous, "h", "I").await?)
        } else {
            0
        };
        Self::position(jvm, ctx, this, index, 0, y, w, h).await
    }
    async fn position(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, index: i32, x: i32, y: i32, w: i32, h: i32) -> Result<()> {
        let mut child: ClassInstanceRef<()> = jvm
            .invoke_virtual(&this, CONTAINER, "getComponent", "(I)Lorg/kwis/msp/lwc/Component;", (index,))
            .await?;
        for (field, value) in [("x", x), ("y", y), ("w", w.max(0)), ("h", h.max(0))] {
            jvm.put_field(&mut child, field, "I", value).await?;
        }
        let _: () = jvm.invoke_virtual(&child, COMPONENT, "invalidate", "()V", ()).await?;
        jvm.invoke_virtual(&this, COMPONENT, "repaint", "()V", ()).await
    }
    async fn children(jvm: &Jvm, this: &ClassInstanceRef<Self>) -> Result<Vec<ClassInstanceRef<()>>> {
        let array: ClassInstanceRef<Array<ClassInstanceRef<()>>> = jvm.get_field(this, "cmps", "[Lorg/kwis/msp/lwc/Component;").await?;
        jvm.load_array(&array, 0, jvm.array_length(&array).await?).await
    }
    async fn preferred(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, _: i32) -> Result<()> {
        let mut width = 0;
        let mut height = 0;
        for child in Self::children(jvm, &this).await? {
            width = width.max(
                jvm.get_field::<i32>(&child, "x", "I")
                    .await?
                    .saturating_add(jvm.get_field::<i32>(&child, "w", "I").await?),
            );
            height = height.max(
                jvm.get_field::<i32>(&child, "y", "I")
                    .await?
                    .saturating_add(jvm.get_field::<i32>(&child, "h", "I").await?),
            );
        }
        jvm.put_field(&mut this, "prefW", "I", width).await?;
        jvm.put_field(&mut this, "prefH", "I", height).await
    }
    async fn layout(jvm: &Jvm, ctx: &mut WieJvmContext, mut this: ClassInstanceRef<Self>) -> Result<()> {
        for child in Self::children(jvm, &this).await? {
            let _: () = jvm.invoke_virtual(&child, COMPONENT, "layout", "()V", ()).await?;
        }
        Self::preferred(jvm, ctx, this.clone(), -1).await?;
        let scroll = jvm.get_field::<i32>(&this, "prefH", "I").await? > jvm.get_field::<i32>(&this, "h", "I").await?;
        jvm.put_field(&mut this, "bScroll", "Z", scroll).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;
    use test_utils::run_jvm_test;
    #[test]
    fn explicit_positions_survive_layout_and_extent_tracks_removal() -> wie_util::Result<()> {
        run_jvm_test(
            Box::new([
                wie_midp::get_protos().into(),
                wie_wipi_java::get_protos().into(),
                vec![GFormComponent::as_proto()].into(),
            ]),
            |jvm| async move {
                let form = jvm.new_class(NAME, "()V", ()).await?;
                let a = jvm.new_class("org/kwis/msp/lwc/ButtonComponent", "()V", ()).await?;
                let b = jvm.new_class("org/kwis/msp/lwc/ButtonComponent", "()V", ()).await?;
                let _: i32 = jvm
                    .invoke_virtual(
                        &form,
                        NAME,
                        "addComponent",
                        "(Lorg/kwis/msp/lwc/Component;IIII)I",
                        (a.clone(), 8, 9, 50, 20),
                    )
                    .await?;
                let _: () = jvm
                    .invoke_virtual(
                        &form,
                        NAME,
                        "addComponent",
                        "(ILorg/kwis/msp/lwc/Component;IIII)V",
                        (0, b.clone(), 70, 80, 10, 12),
                    )
                    .await?;
                let _: () = jvm.invoke_virtual(&form, NAME, "layout", "()V", ()).await?;
                assert_eq!(jvm.get_field::<i32>(&a, "x", "I").await?, 8);
                assert_eq!(jvm.get_field::<i32>(&b, "y", "I").await?, 80);
                assert_eq!(jvm.invoke_virtual::<_, i32>(&form, NAME, "getPreferredWidth", "()I", ()).await?, 80);
                assert_eq!(jvm.invoke_virtual::<_, i32>(&form, NAME, "getPreferredHeight", "()I", ()).await?, 92);
                let _: () = jvm.invoke_virtual(&form, CONTAINER, "removeComponent", "(I)V", (0,)).await?;
                assert_eq!(jvm.invoke_virtual::<_, i32>(&form, NAME, "getPreferredHeight", "()I", ()).await?, 29);
                let _: () = jvm.invoke_virtual(&form, NAME, "addPosData", "(IIIII)V", (0, 4, 5, 6, 7)).await?;
                assert_eq!(jvm.invoke_virtual::<_, i32>(&form, NAME, "getPreferredWidth", "()I", ()).await?, 10);
                assert!(
                    jvm.invoke_virtual::<_, ()>(&form, NAME, "addPosData", "(IIIII)V", (5, 0, 0, 1, 1))
                        .await
                        .is_err()
                );
                Ok(())
            },
        )
    }
}
