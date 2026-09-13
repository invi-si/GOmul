use alloc::vec;

use jvm::{ClassInstanceRef, Jvm, Result as JvmResult};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use rustjava_runtime::classes::java::lang::String;

use wie_jvm_support::{WieJavaClassProto, WieJvmContext};
use wie_midp::classes::javax::microedition::midlet::MIDlet;

use crate::classes::org::kwis::msp::lcdui::{Display, EventQueue};

// class org.kwis.msp.lcdui.Jlet
pub struct Jlet;

impl Jlet {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "org/kwis/msp/lcdui/Jlet",
            parent_class: Some("java/lang/Object"),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("<init>", "()V", Self::init, MethodAccessFlags::PROTECTED),
                JavaMethodProto::new(
                    "getCurrentJlet",
                    "()Lorg/kwis/msp/lcdui/Jlet;",
                    Self::get_active_jlet,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC,
                ),
                JavaMethodProto::new(
                    "getActiveJlet",
                    "()Lorg/kwis/msp/lcdui/Jlet;",
                    Self::get_active_jlet,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC,
                ),
                JavaMethodProto::new(
                    "getEventQueue",
                    "()Lorg/kwis/msp/lcdui/EventQueue;",
                    Self::get_event_queue,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::FINAL,
                ),
                JavaMethodProto::new(
                    "getAppProperty",
                    "(Ljava/lang/String;)Ljava/lang/String;",
                    Self::get_app_property,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::FINAL,
                ),
                JavaMethodProto::new(
                    "notifyDestroyed",
                    "()V",
                    Self::notify_destroyed,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::FINAL,
                ),
                JavaMethodProto::new_abstract(
                    "startApp",
                    "([Ljava/lang/String;)V",
                    MethodAccessFlags::PROTECTED | MethodAccessFlags::ABSTRACT,
                ),
                JavaMethodProto::new_abstract("pauseApp", "()V", MethodAccessFlags::PROTECTED | MethodAccessFlags::ABSTRACT),
                JavaMethodProto::new_abstract("resumeApp", "()V", MethodAccessFlags::PROTECTED | MethodAccessFlags::ABSTRACT),
                JavaMethodProto::new_abstract("destroyApp", "(Z)V", MethodAccessFlags::PROTECTED | MethodAccessFlags::ABSTRACT),
            ],
            fields: vec![
                JavaFieldProto::new("wipiMidlet", "Lnet/wie/WIPIMIDlet;", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("dis", "Lorg/kwis/msp/lcdui/Display;", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("eq", "Lorg/kwis/msp/lcdui/EventQueue;", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new(
                    "currentJlet",
                    "Lorg/kwis/msp/lcdui/Jlet;",
                    FieldAccessFlags::PRIVATE | FieldAccessFlags::STATIC,
                ),
            ],
            access_flags: ClassAccessFlags::PUBLIC | ClassAccessFlags::ABSTRACT,
        }
    }

    async fn init(jvm: &Jvm, _context: &mut WieJvmContext, mut this: ClassInstanceRef<Self>) -> JvmResult<()> {
        tracing::debug!("org.kwis.msp.lcdui.Jlet::<init>({this:?})");

        let _: () = jvm.invoke_special(&this, "java/lang/Object", "<init>", "()V", ()).await?;

        let midlet: ClassInstanceRef<MIDlet> = jvm
            .get_static_field("javax/microedition/midlet/MIDlet", "currentMIDlet", "Ljavax/microedition/midlet/MIDlet;")
            .await?;
        jvm.put_field(&mut this, "wipiMidlet", "Lnet/wie/WIPIMIDlet;", midlet.clone()).await?;
        let _: () = jvm
            .invoke_virtual(
                &midlet,
                "net/wie/WIPIMIDlet",
                "setCurrentJlet",
                "(Lorg/kwis/msp/lcdui/Jlet;)V",
                (this.clone(),),
            )
            .await?;

        let display = jvm
            .new_class(
                "org/kwis/msp/lcdui/Display",
                "(Lorg/kwis/msp/lcdui/Jlet;Lorg/kwis/msp/lcdui/DisplayProxy;)V",
                (this.clone(), None),
            )
            .await?;

        jvm.put_field(&mut this, "dis", "Lorg/kwis/msp/lcdui/Display;", display).await?;

        let event_queue = jvm
            .new_class("org/kwis/msp/lcdui/EventQueue", "(Lorg/kwis/msp/lcdui/Jlet;)V", (this.clone(),))
            .await?;

        jvm.put_field(&mut this, "eq", "Lorg/kwis/msp/lcdui/EventQueue;", event_queue).await?;

        jvm.put_static_field("org/kwis/msp/lcdui/Jlet", "currentJlet", "Lorg/kwis/msp/lcdui/Jlet;", this.clone())
            .await?;

        Ok(())
    }

    async fn get_active_jlet(jvm: &Jvm, _: &mut WieJvmContext) -> JvmResult<ClassInstanceRef<Jlet>> {
        tracing::debug!("org.kwis.msp.lcdui.Jlet::getActiveJlet");

        let jlet = jvm
            .get_static_field("org/kwis/msp/lcdui/Jlet", "currentJlet", "Lorg/kwis/msp/lcdui/Jlet;")
            .await?;

        Ok(jlet)
    }

    async fn get_event_queue(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<ClassInstanceRef<EventQueue>> {
        tracing::debug!("org.kwis.msp.lcdui.Jlet::getEventQueue({this:?})");

        let eq = jvm.get_field(&this, "eq", "Lorg/kwis/msp/lcdui/EventQueue;").await?;

        Ok(eq)
    }

    async fn get_app_property(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        key: ClassInstanceRef<String>,
    ) -> JvmResult<ClassInstanceRef<String>> {
        tracing::debug!("org.kwis.msp.lcdui.Jlet::getAppProperty({this:?}, {key:?})");

        let midlet = jvm.get_field(&this, "wipiMidlet", "Lnet/wie/WIPIMIDlet;").await?;
        let value = jvm
            .invoke_virtual(
                &midlet,
                "net/wie/WIPIMIDlet",
                "getAppProperty",
                "(Ljava/lang/String;)Ljava/lang/String;",
                (key,),
            )
            .await?;

        Ok(value)
    }

    async fn notify_destroyed(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<()> {
        tracing::debug!("org.kwis.msp.lcdui.Jlet::notifyDestroyed({this:?})");

        let midlet: ClassInstanceRef<MIDlet> = jvm.get_field(&this, "wipiMidlet", "Lnet/wie/WIPIMIDlet;").await?;
        let _: () = jvm.invoke_virtual(&midlet, "net/wie/WIPIMIDlet", "notifyDestroyed", "()V", ()).await?;

        let _: () = jvm
            .invoke_virtual(&this, "org/kwis/msp/lcdui/Jlet", "destroyApp", "(Z)V", (false,))
            .await?;

        Ok(())
    }

    pub async fn midlet(jvm: &Jvm, this: &ClassInstanceRef<Self>) -> JvmResult<ClassInstanceRef<MIDlet>> {
        jvm.get_field(this, "wipiMidlet", "Lnet/wie/WIPIMIDlet;").await
    }

    pub async fn display(jvm: &Jvm, this: &ClassInstanceRef<Self>) -> JvmResult<ClassInstanceRef<Display>> {
        jvm.get_field(this, "dis", "Lorg/kwis/msp/lcdui/Display;").await
    }
}

#[cfg(test)]
mod current_jlet_tests {
    #[test]
    fn current_and_active_names_read_the_same_guest_field() -> wie_util::Result<()> {
        let mut protos = alloc::vec::Vec::from(crate::get_protos());
        let mut subclass = super::Jlet::as_proto();
        subclass.name = "test/ConcreteJlet";
        subclass.parent_class = Some("org/kwis/msp/lcdui/Jlet");
        subclass.access_flags = jvm_types::ClassAccessFlags::PUBLIC;
        subclass.methods.clear();
        subclass.fields.clear();
        protos.push(subclass);
        test_utils::run_jvm_test(alloc::boxed::Box::new([protos.into()]), |jvm| async move {
            let object = jvm.instantiate_class("test/ConcreteJlet").await?;
            jvm.put_static_field("org/kwis/msp/lcdui/Jlet", "currentJlet", "Lorg/kwis/msp/lcdui/Jlet;", object.clone())
                .await?;
            for name in ["getCurrentJlet", "getActiveJlet"] {
                let result: jvm::ClassInstanceRef<super::Jlet> = jvm
                    .invoke_static("org/kwis/msp/lcdui/Jlet", name, "()Lorg/kwis/msp/lcdui/Jlet;", ())
                    .await?;
                assert_eq!(&*result, &object);
            }
            Ok(())
        })
    }
}
