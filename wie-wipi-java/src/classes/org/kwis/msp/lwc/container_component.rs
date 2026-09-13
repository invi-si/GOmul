use alloc::{format, vec};

use jvm::{Array, ClassInstanceRef, Jvm, Result as JvmResult};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};

use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

use crate::classes::org::kwis::msp::lwc::Component;

// class org.kwis.msp.lwc.ContainerComponent
pub struct ContainerComponent;

impl ContainerComponent {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "org/kwis/msp/lwc/ContainerComponent",
            parent_class: Some("org/kwis/msp/lwc/Component"),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("getIndexOf", "(Lorg/kwis/msp/lwc/Component;)I", Self::index_of, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "setFocus",
                    "(Lorg/kwis/msp/lwc/Component;)V",
                    Self::focus_child,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("keyNotify", "(II)Z", Self::key_notify, MethodAccessFlags::PROTECTED),
                JavaMethodProto::new("paint", "(Lorg/kwis/msp/lcdui/Graphics;)V", Self::paint, MethodAccessFlags::PROTECTED),
                JavaMethodProto::new(
                    "addComponent",
                    "(ILorg/kwis/msp/lwc/Component;)V",
                    Self::insert_component,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new(
                    "getComponent",
                    "(I)Lorg/kwis/msp/lwc/Component;",
                    Self::get_component,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("getNumberOfComponent", "()I", Self::count, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("removeAllComponents", "()V", Self::remove_all, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("<init>", "()V", Self::init, MethodAccessFlags::PROTECTED),
                JavaMethodProto::new(
                    "addComponent",
                    "(Lorg/kwis/msp/lwc/Component;)I",
                    Self::add_component,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("removeComponent", "(I)V", Self::remove_component_index, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "removeComponent",
                    "(Lorg/kwis/msp/lwc/Component;)V",
                    Self::remove_component,
                    MethodAccessFlags::PUBLIC,
                ),
            ],
            fields: vec![
                JavaFieldProto::new("cmpFocus", "Lorg/kwis/msp/lwc/Component;", FieldAccessFlags::PROTECTED),
                JavaFieldProto::new("cmps", "[Lorg/kwis/msp/lwc/Component;", FieldAccessFlags::PROTECTED),
                JavaFieldProto::new("ncomp", "I", FieldAccessFlags::PROTECTED),
                JavaFieldProto::new("offsetX", "I", FieldAccessFlags::PROTECTED),
                JavaFieldProto::new("offsetY", "I", FieldAccessFlags::PROTECTED),
            ],
            access_flags: ClassAccessFlags::PUBLIC | ClassAccessFlags::ABSTRACT,
        }
    }

    async fn init(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>) -> JvmResult<()> {
        tracing::debug!("stub org.kwis.msp.lwc.ContainerComponent::<init>({this:?})");

        let _: () = jvm.invoke_special(&this, "org/kwis/msp/lwc/Component", "<init>", "()V", ()).await?;

        let children = jvm.instantiate_array("Lorg/kwis/msp/lwc/Component;", 0).await?;
        jvm.put_field(&mut this, "cmps", "[Lorg/kwis/msp/lwc/Component;", children).await?;
        Ok(())
    }

    async fn paint(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, g: ClassInstanceRef<()>) -> JvmResult<()> {
        // A guest override of paintContent need not call the default implementation.
        // Resolve child geometry before computing any child clips.
        let _: () = jvm.invoke_virtual(&this, "org/kwis/msp/lwc/Component", "validate", "()V", ()).await?;
        let _: () = jvm
            .invoke_virtual(
                &this,
                "org/kwis/msp/lwc/Component",
                "paintContent",
                "(Lorg/kwis/msp/lcdui/Graphics;)V",
                (g.clone(),),
            )
            .await?;
        let count: i32 = jvm.get_field(&this, "ncomp", "I").await?;
        let array: ClassInstanceRef<Array<ClassInstanceRef<Component>>> = jvm.get_field(&this, "cmps", "[Lorg/kwis/msp/lwc/Component;").await?;
        let children: alloc::vec::Vec<ClassInstanceRef<Component>> = jvm.load_array(&array, 0, count as _).await?;
        for child in children {
            let x: i32 = jvm
                .get_field::<i32>(&child, "x", "I")
                .await?
                .wrapping_add(jvm.get_field::<i32>(&this, "offsetX", "I").await?);
            let y: i32 = jvm
                .get_field::<i32>(&child, "y", "I")
                .await?
                .wrapping_add(jvm.get_field::<i32>(&this, "offsetY", "I").await?);
            let width: i32 = jvm.get_field(&child, "w", "I").await?;
            let height: i32 = jvm.get_field(&child, "h", "I").await?;
            let mut clip = [0i32; 4];
            for (slot, name) in clip.iter_mut().zip(["getClipX", "getClipY", "getClipWidth", "getClipHeight"]) {
                *slot = jvm.invoke_virtual(&g, "org/kwis/msp/lcdui/Graphics", name, "()I", ()).await?;
            }
            let _: () = jvm
                .invoke_virtual(&g, "org/kwis/msp/lcdui/Graphics", "translate", "(II)V", (x, y))
                .await?;
            let _: () = jvm
                .invoke_virtual(&g, "org/kwis/msp/lcdui/Graphics", "clipRect", "(IIII)V", (0, 0, width, height))
                .await?;
            let (class, method) = if jvm.is_instance(&**child, "org/kwis/msp/lwc/ContainerComponent") {
                ("org/kwis/msp/lwc/ContainerComponent", "paint")
            } else {
                ("org/kwis/msp/lwc/Component", "paintContent")
            };
            let painted: JvmResult<()> = jvm
                .invoke_virtual(&child, class, method, "(Lorg/kwis/msp/lcdui/Graphics;)V", (g.clone(),))
                .await;
            let _: () = jvm
                .invoke_virtual(
                    &g,
                    "org/kwis/msp/lcdui/Graphics",
                    "translate",
                    "(II)V",
                    (x.wrapping_neg(), y.wrapping_neg()),
                )
                .await?;
            let _: () = jvm
                .invoke_virtual(
                    &g,
                    "org/kwis/msp/lcdui/Graphics",
                    "setClip",
                    "(IIII)V",
                    (clip[0], clip[1], clip[2], clip[3]),
                )
                .await?;
            painted?;
        }
        Ok(())
    }
    async fn index_of(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, child: ClassInstanceRef<Component>) -> JvmResult<i32> {
        let array: ClassInstanceRef<Array<ClassInstanceRef<Component>>> = jvm.get_field(&this, "cmps", "[Lorg/kwis/msp/lwc/Component;").await?;
        let children: alloc::vec::Vec<ClassInstanceRef<Component>> = jvm.load_array(&array, 0, jvm.array_length(&array).await?).await?;
        Ok(children
            .iter()
            .position(|c| child.instance.as_ref().is_some_and(|target| c.identity() == target.identity()))
            .map_or(-1, |i| i as i32))
    }
    async fn focus_child(jvm: &Jvm, ctx: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, child: ClassInstanceRef<Component>) -> JvmResult<()> {
        if !child.is_null() && Self::index_of(jvm, ctx, this.clone(), child.clone()).await? < 0 {
            return Err(jvm.exception("java/lang/IllegalArgumentException", "Focus target is not a child").await);
        }
        let previous: ClassInstanceRef<Component> = jvm.get_field(&this, "cmpFocus", "Lorg/kwis/msp/lwc/Component;").await?;
        if previous.instance.as_ref().map(|x| x.identity()) == child.instance.as_ref().map(|x| x.identity()) {
            return Ok(());
        }
        jvm.put_field(&mut this, "cmpFocus", "Lorg/kwis/msp/lwc/Component;", child.clone())
            .await?;
        for (mut c, focused) in [(previous, false), (child, true)] {
            if !c.is_null() {
                jvm.put_field(&mut c, "wieFocused", "Z", focused).await?;
                let _: () = jvm
                    .invoke_virtual(&c, "org/kwis/msp/lwc/Component", "focusNotify", "(Z)V", (focused,))
                    .await?;
            }
        }
        Ok(())
    }
    async fn key_notify(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, kind: i32, key: i32) -> JvmResult<bool> {
        let child: ClassInstanceRef<Component> = jvm.get_field(&this, "cmpFocus", "Lorg/kwis/msp/lwc/Component;").await?;
        if child.is_null() {
            return Ok(false);
        }
        jvm.invoke_virtual(&child, "org/kwis/msp/lwc/Component", "keyNotify", "(II)Z", (kind, key))
            .await
    }
    async fn count(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        jvm.get_field(&this, "ncomp", "I").await
    }
    async fn get_component(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, index: i32) -> JvmResult<ClassInstanceRef<Component>> {
        let array: ClassInstanceRef<Array<ClassInstanceRef<Component>>> = jvm.get_field(&this, "cmps", "[Lorg/kwis/msp/lwc/Component;").await?;
        let count = jvm.array_length(&array).await?;
        if index < 0 || index as usize >= count {
            return Err(jvm
                .exception(
                    "java/lang/IndexOutOfBoundsException",
                    &format!("{}: child {index}, count {count}", this.class_definition().name()),
                )
                .await);
        }
        Ok(jvm.load_array(&array, index as _, 1).await?.remove(0))
    }
    async fn add_component(
        jvm: &Jvm,
        context: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        component: ClassInstanceRef<Component>,
    ) -> JvmResult<i32> {
        let count = jvm.get_field(&this, "ncomp", "I").await?;
        Self::insert_component(jvm, context, this, count, component).await?;
        Ok(count)
    }
    async fn insert_component(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        index: i32,
        mut component: ClassInstanceRef<Component>,
    ) -> JvmResult<()> {
        if component.is_null() {
            return Err(jvm.exception("java/lang/NullPointerException", "component").await);
        }
        let count: i32 = jvm.get_field(&this, "ncomp", "I").await?;
        if index < 0 || index > count {
            return Err(jvm.exception("java/lang/ArrayIndexOutOfBoundsException", "component index").await);
        }
        let parent: ClassInstanceRef<()> = jvm.get_field(&component, "parent", "Lorg/kwis/msp/lwc/ContainerComponent;").await?;
        if !parent.is_null() {
            return Err(jvm.exception("java/lang/IllegalArgumentException", "component already attached").await);
        }
        let mut ancestor: ClassInstanceRef<()> = this.instance.clone().into();
        while !ancestor.is_null() {
            if ancestor.identity() == component.identity() {
                return Err(jvm.exception("java/lang/IllegalArgumentException", "component cycle").await);
            }
            ancestor = jvm.get_field(&ancestor, "parent", "Lorg/kwis/msp/lwc/ContainerComponent;").await?;
        }
        let old: ClassInstanceRef<Array<ClassInstanceRef<Component>>> = jvm.get_field(&this, "cmps", "[Lorg/kwis/msp/lwc/Component;").await?;
        let mut values = jvm.load_array(&old, 0, count as _).await?;
        values.insert(index as _, component.clone());
        let mut new = jvm.instantiate_array("Lorg/kwis/msp/lwc/Component;", values.len()).await?;
        jvm.store_array(&mut new, 0, values).await?;
        jvm.put_field(&mut component, "parent", "Lorg/kwis/msp/lwc/ContainerComponent;", this.clone())
            .await?;
        jvm.put_field(&mut this, "cmps", "[Lorg/kwis/msp/lwc/Component;", new).await?;
        jvm.put_field(&mut this, "ncomp", "I", count + 1).await?;
        jvm.invoke_virtual(&this, "org/kwis/msp/lwc/Component", "invalidate", "()V", ()).await
    }
    async fn remove_component_index(jvm: &Jvm, ctx: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, index: i32) -> JvmResult<()> {
        let count: i32 = jvm.get_field(&this, "ncomp", "I").await?;
        if index < 0 || index >= count {
            return Err(jvm.exception("java/lang/ArrayIndexOutOfBoundsException", "component index").await);
        }
        let old: ClassInstanceRef<Array<ClassInstanceRef<Component>>> = jvm.get_field(&this, "cmps", "[Lorg/kwis/msp/lwc/Component;").await?;
        let mut values = jvm.load_array(&old, 0, count as _).await?;
        let mut removed: ClassInstanceRef<Component> = values.remove(index as _);
        let focused: ClassInstanceRef<Component> = jvm.get_field(&this, "cmpFocus", "Lorg/kwis/msp/lwc/Component;").await?;
        if !focused.is_null() && focused.identity() == removed.identity() {
            Self::focus_child(jvm, ctx, this.clone(), None.into()).await?;
        }
        let mut new = jvm.instantiate_array("Lorg/kwis/msp/lwc/Component;", values.len()).await?;
        jvm.store_array(&mut new, 0, values).await?;
        jvm.put_field(
            &mut removed,
            "parent",
            "Lorg/kwis/msp/lwc/ContainerComponent;",
            ClassInstanceRef::<()>::new(None),
        )
        .await?;
        jvm.put_field(&mut this, "cmps", "[Lorg/kwis/msp/lwc/Component;", new).await?;
        jvm.put_field(&mut this, "ncomp", "I", count - 1).await?;
        jvm.invoke_virtual(&this, "org/kwis/msp/lwc/Component", "invalidate", "()V", ()).await
    }
    async fn remove_component(
        jvm: &Jvm,
        context: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        component: ClassInstanceRef<Component>,
    ) -> JvmResult<()> {
        if component.is_null() {
            return Ok(());
        }
        let count: i32 = jvm.get_field(&this, "ncomp", "I").await?;
        let array: ClassInstanceRef<Array<ClassInstanceRef<Component>>> = jvm.get_field(&this, "cmps", "[Lorg/kwis/msp/lwc/Component;").await?;
        let values: alloc::vec::Vec<ClassInstanceRef<Component>> = jvm.load_array(&array, 0, count as _).await?;
        if let Some(index) = values.iter().position(|c| c.identity() == component.identity()) {
            Self::remove_component_index(jvm, context, this, index as _).await?;
        }
        Ok(())
    }
    async fn remove_all(jvm: &Jvm, context: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<()> {
        let count: i32 = jvm.get_field(&this, "ncomp", "I").await?;
        for index in (0..count).rev() {
            Self::remove_component_index(jvm, context, this.clone(), index).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use alloc::boxed::Box;
    use jvm::ClassInstanceRef;
    use test_utils::run_jvm_test;
    use wie_util::Result;

    #[test]
    fn attachment_replacement_and_cycle_rejection_preserve_the_tree() -> Result<()> {
        run_jvm_test(
            Box::new([
                wie_midp::get_protos().into(),
                crate::classes::org::kwis::msp::lwc::shell_component::test_support::protos(),
            ]),
            |jvm| async move {
                crate::classes::org::kwis::msp::lwc::shell_component::test_support::init(&jvm).await?;
                let shell_name = "org/kwis/msp/lwc/ShellComponent";
                let container = "org/kwis/msp/lwc/ContainerComponent";
                let shell = jvm.new_class(shell_name, "(IIII)V", (2, 3, 80, 90)).await?;
                let other = jvm.new_class(shell_name, "()V", ()).await?;
                let a = jvm.new_class("org/kwis/msp/lwc/FormComponent", "()V", ()).await?;
                let b = jvm.new_class("org/kwis/msp/lwc/FormComponent", "()V", ()).await?;
                let _: () = jvm
                    .invoke_virtual(&shell, shell_name, "setWorkComponent", "(Lorg/kwis/msp/lwc/Component;)V", (a.clone(),))
                    .await?;
                let _: () = jvm.invoke_virtual(&shell, shell_name, "layout", "()V", ()).await?;
                assert_eq!(jvm.get_field::<i32>(&a, "w", "I").await?, 80);
                assert_eq!(jvm.get_field::<i32>(&a, "h", "I").await?, 90);
                assert!(
                    jvm.invoke_virtual::<_, i32>(&a, container, "addComponent", "(Lorg/kwis/msp/lwc/Component;)I", (shell.clone(),))
                        .await
                        .is_err()
                );
                assert!(
                    jvm.invoke_virtual::<_, i32>(&other, container, "addComponent", "(Lorg/kwis/msp/lwc/Component;)I", (a.clone(),))
                        .await
                        .is_err()
                );
                assert!(
                    jvm.invoke_virtual::<_, ()>(&shell, container, "removeComponent", "(I)V", (9,))
                        .await
                        .is_err()
                );
                assert_eq!(
                    jvm.invoke_virtual::<_, i32>(&shell, container, "getNumberOfComponent", "()I", ()).await?,
                    1
                );
                let _: () = jvm
                    .invoke_virtual(&shell, shell_name, "setWorkComponent", "(Lorg/kwis/msp/lwc/Component;)V", (b.clone(),))
                    .await?;
                let parent: ClassInstanceRef<()> = jvm.get_field(&a, "parent", "Lorg/kwis/msp/lwc/ContainerComponent;").await?;
                assert!(parent.is_null());
                let child: ClassInstanceRef<()> = jvm
                    .invoke_virtual(&shell, container, "getComponent", "(I)Lorg/kwis/msp/lwc/Component;", (0,))
                    .await?;
                assert_eq!(child.identity(), b.identity());
                let _: () = jvm.invoke_virtual(&shell, container, "removeAllComponents", "()V", ()).await?;
                assert_eq!(
                    jvm.invoke_virtual::<_, i32>(&shell, container, "getNumberOfComponent", "()I", ()).await?,
                    0
                );
                let parent: ClassInstanceRef<()> = jvm.get_field(&b, "parent", "Lorg/kwis/msp/lwc/ContainerComponent;").await?;
                assert!(parent.is_null());
                Ok(())
            },
        )
    }
}
