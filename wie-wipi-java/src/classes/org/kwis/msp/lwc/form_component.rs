use crate::classes::net::wie::WIPIKeyCode;
use alloc::{vec, vec::Vec};
use jvm::{Array, ClassInstanceRef, Jvm, Result};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

pub struct FormComponent;
const NAME: &str = "org/kwis/msp/lwc/FormComponent";
const CONTAINER: &str = "org/kwis/msp/lwc/ContainerComponent";
const COMPONENT: &str = "org/kwis/msp/lwc/Component";
impl FormComponent {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: NAME,
            parent_class: Some(CONTAINER),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("<init>", "()V", Self::init, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("<init>", "(Z)V", Self::init_orientation, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setPacked", "(Z)V", Self::set_packed, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getPacked", "()Z", Self::packed, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setGab", "(I)V", Self::set_gap, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getGab", "()I", Self::gap, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("layout", "()V", Self::layout, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("layoutChildVertical", "()V", Self::layout_vertical, MethodAccessFlags::PROTECTED),
                JavaMethodProto::new("layoutChildHorizontal", "()V", Self::layout_horizontal, MethodAccessFlags::PROTECTED),
                JavaMethodProto::new("calcPreferredSize", "(I)V", Self::calc_preferred, MethodAccessFlags::PROTECTED),
                JavaMethodProto::new("keyNotify", "(II)Z", Self::key, MethodAccessFlags::PROTECTED),
                JavaMethodProto::new("focusNotify", "(Z)V", Self::focus, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "getNextTraversalComponent",
                    "()Lorg/kwis/msp/lwc/Component;",
                    Self::next,
                    MethodAccessFlags::PROTECTED,
                ),
                JavaMethodProto::new(
                    "getPrevTraversalComponent",
                    "()Lorg/kwis/msp/lwc/Component;",
                    Self::previous,
                    MethodAccessFlags::PROTECTED,
                ),
                JavaMethodProto::new("scrollTo", "(II)Z", Self::scroll_to, MethodAccessFlags::PROTECTED),
            ],
            fields: vec![
                JavaFieldProto::new("vertical", "Z", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("packed", "Z", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("gap", "I", FieldAccessFlags::PRIVATE),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }
    async fn init(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<()> {
        Self::init_orientation(jvm, ctx, this, true).await
    }
    async fn init_orientation(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, vertical: bool) -> Result<()> {
        let _: () = jvm.invoke_special(&this, CONTAINER, "<init>", "()V", ()).await?;
        jvm.put_field(&mut this, "vertical", "Z", vertical).await?;
        jvm.put_field(&mut this, "wieInput", "Z", true).await
    }
    async fn packed(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<bool> {
        jvm.get_field(&this, "packed", "Z").await
    }
    async fn set_packed(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, packed: bool) -> Result<()> {
        jvm.put_field(&mut this, "packed", "Z", packed).await?;
        jvm.invoke_virtual(&this, COMPONENT, "invalidate", "()V", ()).await
    }
    async fn gap(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<i32> {
        jvm.get_field(&this, "gap", "I").await
    }
    async fn set_gap(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, gap: i32) -> Result<()> {
        jvm.put_field(&mut this, "gap", "I", gap).await?;
        jvm.invoke_virtual(&this, COMPONENT, "invalidate", "()V", ()).await
    }
    async fn children(jvm: &Jvm, this: &ClassInstanceRef<Self>) -> Result<Vec<ClassInstanceRef<()>>> {
        let a: ClassInstanceRef<Array<ClassInstanceRef<()>>> = jvm.get_field(this, "cmps", "[Lorg/kwis/msp/lwc/Component;").await?;
        jvm.load_array(&a, 0, jvm.array_length(&a).await?).await
    }
    async fn arrange(jvm: &Jvm, mut this: ClassInstanceRef<Self>, vertical: bool, width: i32, place: bool) -> Result<()> {
        let packed: bool = jvm.get_field(&this, "packed", "Z").await?;
        let gap: i32 = jvm.get_field(&this, "gap", "I").await?;
        let mut along = 0i32;
        let mut across = 0i32;
        let children = Self::children(jvm, &this).await?;
        for (index, mut child) in children.into_iter().enumerate() {
            if index > 0 {
                along = along.saturating_add(gap);
            }
            let pw: i32 = jvm.invoke_virtual(&child, COMPONENT, "getPreferredWidth", "()I", ()).await?;
            let w = if vertical && packed && width >= 0 { width } else { pw.max(0) };
            let h: i32 = jvm.invoke_virtual(&child, COMPONENT, "getPreferredHeight", "(I)I", (w,)).await?;
            let h = h.max(0);
            if place {
                for (field, v) in [
                    ("x", if vertical { 0 } else { along }),
                    ("y", if vertical { along } else { 0 }),
                    ("w", w),
                    ("h", h),
                ] {
                    jvm.put_field(&mut child, field, "I", v).await?;
                }
                let _: () = jvm.invoke_virtual(&child, COMPONENT, "layout", "()V", ()).await?;
            }
            along = along.saturating_add(if vertical { h } else { w });
            across = across.max(if vertical { w } else { h });
        }
        jvm.put_field(&mut this, "prefW", "I", if vertical { across } else { along }).await?;
        jvm.put_field(&mut this, "prefH", "I", if vertical { along } else { across }).await
    }
    async fn layout(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<()> {
        let vertical: bool = jvm.get_field(&this, "vertical", "Z").await?;
        if vertical {
            Self::layout_vertical(jvm, ctx, this).await
        } else {
            Self::layout_horizontal(jvm, ctx, this).await
        }
    }
    async fn layout_vertical(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<()> {
        let w = jvm.get_field(&this, "w", "I").await?;
        Self::arrange(jvm, this, true, w, true).await
    }
    async fn layout_horizontal(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<()> {
        let w = jvm.get_field(&this, "w", "I").await?;
        Self::arrange(jvm, this, false, w, true).await
    }
    async fn calc_preferred(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, width: i32) -> Result<()> {
        let vertical = jvm.get_field(&this, "vertical", "Z").await?;
        Self::arrange(jvm, this, vertical, width, false).await
    }
    async fn traverse(jvm: &Jvm, this: ClassInstanceRef<Self>, forward: bool) -> Result<ClassInstanceRef<()>> {
        let children = Self::children(jvm, &this).await?;
        let current: ClassInstanceRef<()> = jvm.get_field(&this, "cmpFocus", "Lorg/kwis/msp/lwc/Component;").await?;
        let index = children
            .iter()
            .position(|c| current.instance.as_ref().is_some_and(|target| c.identity() == target.identity()));
        let indices: Vec<usize> = if forward {
            ((index.map_or(0, |i| i + 1))..children.len()).collect()
        } else {
            (0..index.unwrap_or(children.len())).rev().collect()
        };
        for i in indices {
            if jvm
                .invoke_virtual::<_, bool>(&children[i], COMPONENT, "canHandleInput", "()Z", ())
                .await?
            {
                return Ok(children[i].clone());
            }
        }
        Ok(None.into())
    }
    async fn next(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<ClassInstanceRef<()>> {
        Self::traverse(jvm, this, true).await
    }
    async fn previous(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<ClassInstanceRef<()>> {
        Self::traverse(jvm, this, false).await
    }
    async fn focus(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, focused: bool) -> Result<()> {
        let _: () = jvm.invoke_special(&this, COMPONENT, "focusNotify", "(Z)V", (focused,)).await?;
        let current: ClassInstanceRef<()> = jvm.get_field(&this, "cmpFocus", "Lorg/kwis/msp/lwc/Component;").await?;
        if focused && current.is_null() {
            let first = Self::traverse(jvm, this.clone(), true).await?;
            let _: () = jvm
                .invoke_virtual(&this, CONTAINER, "setFocus", "(Lorg/kwis/msp/lwc/Component;)V", (first,))
                .await?;
        } else if !current.is_null() {
            let _: () = jvm.invoke_virtual(&current, COMPONENT, "focusNotify", "(Z)V", (focused,)).await?;
        }
        Ok(())
    }
    async fn key(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>, kind: i32, key: i32) -> Result<bool> {
        let focused: ClassInstanceRef<()> = jvm.get_field(&this, "cmpFocus", "Lorg/kwis/msp/lwc/Component;").await?;
        if focused.is_null() {
            Self::focus(jvm, ctx, this.clone(), true).await?;
        }
        if jvm.invoke_special::<_, bool>(&this, CONTAINER, "keyNotify", "(II)Z", (kind, key)).await? {
            return Ok(true);
        }
        let forward = if key == WIPIKeyCode::DOWN as i32 {
            true
        } else if key == WIPIKeyCode::UP as i32 {
            false
        } else {
            return Ok(false);
        };
        if kind != 1 && kind != 3 {
            return Ok(true);
        }
        let child = Self::traverse(jvm, this.clone(), forward).await?;
        if child.is_null() {
            return Ok(false);
        }
        let _: () = jvm
            .invoke_virtual(&this, CONTAINER, "setFocus", "(Lorg/kwis/msp/lwc/Component;)V", (child.clone(),))
            .await?;
        let y: i32 = jvm.get_field(&child, "y", "I").await?;
        let h: i32 = jvm.get_field(&child, "h", "I").await?;
        let viewport: i32 = jvm.get_field(&this, "h", "I").await?;
        let offset: i32 = jvm.get_field(&this, "offsetY", "I").await?;
        let dy = if y + offset < 0 {
            y + offset
        } else {
            (y + h + offset - viewport).max(0)
        };
        if dy != 0 {
            Self::scroll_to(jvm, ctx, this, 0, dy).await?;
        }
        Ok(true)
    }
    async fn scroll_to(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, dx: i32, dy: i32) -> Result<bool> {
        let mut changed = false;
        for (offset, extent, viewport, delta) in [("offsetX", "prefW", "w", dx), ("offsetY", "prefH", "h", dy)] {
            let old: i32 = jvm.get_field(&this, offset, "I").await?;
            let size: i32 = jvm.get_field(&this, extent, "I").await?;
            let view: i32 = jvm.get_field(&this, viewport, "I").await?;
            let value = old.saturating_sub(delta).clamp(-(size.saturating_sub(view).max(0)), 0);
            changed |= old != value;
            jvm.put_field(&mut this, offset, "I", value).await?;
        }
        if changed {
            let _: () = jvm.invoke_virtual(&this, COMPONENT, "repaint", "()V", ()).await?;
        }
        Ok(changed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;
    use test_utils::run_jvm_test;
    #[test]
    fn layout_focus_and_detachment_preserve_child_state() -> wie_util::Result<()> {
        run_jvm_test(Box::new([wie_midp::get_protos().into(), crate::get_protos().into()]), |jvm| async move {
            let mut form = jvm.new_class(NAME, "()V", ()).await?;
            let mut children = Vec::new();
            for i in 0..3 {
                let mut child = jvm
                    .new_class("org/kwis/msp/lwc/ShellComponent", "(IIII)V", (0, 0, 20 + i * 10, 15 + i * 5))
                    .await?;
                jvm.put_field(&mut child, "wieInput", "Z", i != 1).await?;
                let _: i32 = jvm
                    .invoke_virtual(&form, CONTAINER, "addComponent", "(Lorg/kwis/msp/lwc/Component;)I", (child.clone(),))
                    .await?;
                children.push(child);
            }
            jvm.put_field(&mut form, "w", "I", 100).await?;
            jvm.put_field(&mut form, "h", "I", 30).await?;
            let _: () = jvm.invoke_virtual(&form, NAME, "setGab", "(I)V", (3,)).await?;
            let _: () = jvm.invoke_virtual(&form, NAME, "setPacked", "(Z)V", (true,)).await?;
            let _: () = jvm.invoke_virtual(&form, NAME, "layout", "()V", ()).await?;
            assert_eq!(jvm.get_field::<i32>(&children[0], "w", "I").await?, 100);
            assert_eq!(jvm.get_field::<i32>(&children[1], "y", "I").await?, 18);
            assert_eq!(jvm.get_field::<i32>(&children[2], "y", "I").await?, 41);
            let _: () = jvm.invoke_virtual(&form, NAME, "focusNotify", "(Z)V", (true,)).await?;
            let focus: ClassInstanceRef<()> = jvm.get_field(&form, "cmpFocus", "Lorg/kwis/msp/lwc/Component;").await?;
            assert_eq!(focus.identity(), children[0].identity());
            let _: bool = jvm
                .invoke_virtual(&form, NAME, "keyNotify", "(II)Z", (1, WIPIKeyCode::DOWN as i32))
                .await?;
            let focus: ClassInstanceRef<()> = jvm.get_field(&form, "cmpFocus", "Lorg/kwis/msp/lwc/Component;").await?;
            assert_eq!(focus.identity(), children[2].identity());
            assert!(!jvm.get_field::<bool>(&children[0], "wieFocused", "Z").await?);
            assert_eq!(jvm.get_field::<i32>(&form, "offsetY", "I").await?, -36);
            assert_eq!(jvm.invoke_virtual::<_, i32>(&children[2], COMPONENT, "getYOnScreen", "()I", ()).await?, 5);
            let _: () = jvm.invoke_virtual(&form, CONTAINER, "removeComponent", "(I)V", (2,)).await?;
            let focus: ClassInstanceRef<()> = jvm.get_field(&form, "cmpFocus", "Lorg/kwis/msp/lwc/Component;").await?;
            assert!(focus.is_null());
            assert!(!jvm.get_field::<bool>(&children[2], "wieFocused", "Z").await?);
            assert!(
                jvm.invoke_virtual::<_, ()>(&form, CONTAINER, "setFocus", "(Lorg/kwis/msp/lwc/Component;)V", (children[2].clone(),))
                    .await
                    .is_err()
            );
            Ok(())
        })
    }
}
