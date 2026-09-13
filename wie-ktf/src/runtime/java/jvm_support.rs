mod array_class_definition;
mod array_class_instance;
mod class_definition;
mod class_instance;
#[cfg(test)]
mod class_loader_tests;
pub(crate) mod class_memory;
mod classes;
mod field;
mod jvm_implementation;
mod method;
mod name;
mod value;
mod vtable;

use alloc::boxed::Box;
use core::mem::{offset_of, size_of};
use jvm_implementation::KtfJvmImplementation;

use bytemuck::{Pod, Zeroable};

use jvm::{ClassDefinition, ClassInstance, ClassInstanceRef, Jvm, Result as JvmResult, runtime::JavaLangString};
use rustjava_runtime::classes::java::util::{Enumeration, jar::JarEntry};

use wie_backend::System;
use wie_core_arm::{Allocator, ArmCore};
use wie_jvm_support::JvmSupport;
use wie_midp::classes::javax::microedition::midlet::MIDlet;
use wie_util::{Result, WieError, read_generic, write_generic};

use self::{
    array_class_instance::JavaArrayClassInstance,
    classes::net::wie::{ClassLoaderContext, KtfClassLoader},
    name::JavaFullName,
};
use super::interface::register_java_interface_svc_handler;

pub use self::{
    array_class_definition::JavaArrayClassDefinition,
    class_definition::JavaClassDefinition,
    class_instance::JavaClassInstance,
    method::{JavaMethod, JavaMethodResult},
    vtable::JavaVtable,
};

pub type KtfJvmWord = u32;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct KtfJvmThreadContext {
    unk: [u32; 8],
    current_java_exception_handler: u32,
    native_return: NativeReturnSlot,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct NativeReturnSlot {
    kind: u32,
    words: [u32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct KtfJvmSupportContext {
    class_memory_cursor: u32,
    ptr_current_jvm_thread_context: u32,
    image_base: u32,
    image_size: u32,
    ptr_network_endpoint: u32,
    network_endpoint_length: u32,
}

const SUPPORT_CONTEXT_BASE: u32 = 0x7fff0000;

pub struct KtfJvmSupport;

impl KtfJvmSupport {
    pub(crate) fn set_network_endpoint(core: &mut ArmCore, address: u32) -> Result<()> {
        use wie_util::{ByteWrite, read_null_terminated_string_bytes};
        let mut bytes = read_null_terminated_string_bytes(core, address)?;
        bytes.push(0);
        let root = SUPPORT_CONTEXT_BASE + offset_of!(KtfJvmSupportContext, ptr_network_endpoint) as u32;
        let previous: u32 = read_generic(core, root)?;
        let length_root = SUPPORT_CONTEXT_BASE + offset_of!(KtfJvmSupportContext, network_endpoint_length) as u32;
        let previous_length: u32 = read_generic(core, length_root)?;
        let copy = Allocator::alloc(core, bytes.len() as u32)?;
        core.write_bytes(copy, &bytes)?;
        write_generic(core, root, copy)?;
        write_generic(core, length_root, bytes.len() as u32)?;
        if previous != 0 {
            Allocator::free(core, previous, previous_length)?;
        }
        Ok(())
    }
    pub fn set_native_image(core: &mut ArmCore, base: u32, size: u32) -> Result<()> {
        write_generic(core, SUPPORT_CONTEXT_BASE + offset_of!(KtfJvmSupportContext, image_base) as u32, base)?;
        write_generic(core, SUPPORT_CONTEXT_BASE + offset_of!(KtfJvmSupportContext, image_size) as u32, size)
    }

    pub async fn init(core: &mut ArmCore, system: &mut System, jar_name: Option<&str>) -> Result<(Jvm, Box<dyn ClassInstance>)> {
        let ptr_jvm_context = class_memory::init(core)?;

        let protos = [
            wie_wipi_java::get_protos().into(),
            wie_midp::get_protos().into(),
            alloc::vec![
                classes::wec::OEMDevice::as_proto(),
                classes::kfc::ChoiceText::as_proto(),
                classes::gform_component::GFormComponent::as_proto(),
                classes::gform::GForm::as_proto(),
                classes::gform_base::GFormBase::as_proto(),
                classes::gmenu_bar::GMenuBar::as_proto(),
                classes::gmenubar_form::GTextModeListener::as_proto(),
                classes::gmenubar_form::GMenubarForm::as_proto(),
                classes::gtext_field::GTextField::as_proto()
            ]
            .into(),
        ];
        let jvm_implementation = KtfJvmImplementation::new(core);
        let jvm = JvmSupport::new_jvm(system, jar_name, Box::new(protos), &[], jvm_implementation.clone()).await?;
        let presenter = JavaClassDefinition::new(
            core,
            &jvm,
            classes::screen_presenter::KtfScreenPresenter::as_proto(),
            Box::new(ClassLoaderContext {
                core: core.clone(),
                system: system.clone(),
            }),
            jvm_implementation.java_functions(),
        )
        .await?;
        jvm.register_class(Box::new(presenter), None)
            .await
            .map_err(|error| WieError::FatalError(alloc::format!("Cannot register native screen presenter: {error:?}")))?;
        let root_core = core.clone();
        jvm.set_native_root_provider(move || {
            let snapshot = (|| -> Result<_> {
                let context: KtfJvmSupportContext = read_generic(&root_core, SUPPORT_CONTEXT_BASE)?;
                root_core.native_heap_reference_candidates(context.image_base, context.image_size)
            })();
            match snapshot {
                Ok(roots) => Some(roots),
                Err(error) => {
                    tracing::error!("Cannot snapshot KTF native roots: {error}");
                    None
                }
            }
        });
        register_java_interface_svc_handler(core, &jvm)?;

        let system_class_loader: Box<dyn ClassInstance> = jvm
            .invoke_static("java/lang/ClassLoader", "getSystemClassLoader", "()Ljava/lang/ClassLoader;", [])
            .await
            .unwrap();

        // used in tests
        if jar_name.is_none() {
            return Ok((jvm, system_class_loader));
        }

        // find client.bin
        let jar_name_java = JavaLangString::from_rust_string(&jvm, jar_name.unwrap()).await.unwrap();
        let jar_file = jvm
            .new_class("java/util/jar/JarFile", "(Ljava/lang/String;)V", (jar_name_java,))
            .await
            .unwrap();
        let entries: ClassInstanceRef<Enumeration> = jvm
            .invoke_virtual(&jar_file, "java/util/jar/JarFile", "entries", "()Ljava/util/Enumeration;", [])
            .await
            .unwrap();

        let binary_name = loop {
            let has_more_elements: bool = jvm
                .invoke_virtual(&entries, "java/util/Enumeration", "hasMoreElements", "()Z", [])
                .await
                .unwrap();
            if !has_more_elements {
                return Err(WieError::FatalError("client.bin not found".into()));
            }

            let entry: ClassInstanceRef<JarEntry> = jvm
                .invoke_virtual(&entries, "java/util/Enumeration", "nextElement", "()Ljava/lang/Object;", [])
                .await
                .unwrap();
            let name = jvm
                .invoke_virtual(&entry, "java/util/jar/JarEntry", "getName", "()Ljava/lang/String;", [])
                .await
                .unwrap();
            let name_rust = JavaLangString::to_rust_string(&jvm, &name).await.unwrap();

            if name_rust.starts_with("client.bin") {
                break name;
            }
        };

        let class_loader_class = JavaClassDefinition::new(
            core,
            &jvm,
            KtfClassLoader::as_proto(),
            Box::new(ClassLoaderContext {
                core: core.clone(),
                system: system.clone(),
            }) as Box<_>,
            jvm_implementation.java_functions(),
        )
        .await?;

        jvm.register_class(Box::new(class_loader_class), None).await.unwrap();

        let class_loader = match jvm
            .new_class(
                "net/wie/KtfClassLoader",
                "(Ljava/lang/ClassLoader;Ljava/lang/String;II)V",
                (
                    system_class_loader,
                    binary_name,
                    ptr_jvm_context as i32,
                    (SUPPORT_CONTEXT_BASE + offset_of!(KtfJvmSupportContext, ptr_current_jvm_thread_context) as u32) as i32,
                ),
            )
            .await
        {
            Ok(loader) => loader,
            Err(error) => return Err(JvmSupport::to_wie_err(&jvm, error).await),
        };

        Ok((jvm, class_loader))
    }

    pub(crate) async fn mark_repaint_pending(jvm: &Jvm, framebuffer: Option<u32>) -> JvmResult<()> {
        let midlet: ClassInstanceRef<MIDlet> = jvm
            .get_static_field("javax/microedition/midlet/MIDlet", "currentMIDlet", "Ljavax/microedition/midlet/MIDlet;")
            .await?;
        if midlet.is_null() {
            return Ok(());
        }
        let mut display = MIDlet::display(jvm, &midlet).await?;
        if let Some(framebuffer) = framebuffer {
            let mut presenter: ClassInstanceRef<()> = jvm.get_field(&display, "nativePainter", "Ljava/lang/Runnable;").await?;
            if presenter.is_null() {
                presenter = jvm.instantiate_class("net/wie/KtfScreenPresenter").await?.into();
                jvm.put_field(&mut display, "nativePainter", "Ljava/lang/Runnable;", presenter.clone())
                    .await?;
            }
            jvm.put_field(&mut presenter, "framebuffer", "I", framebuffer as i32).await?;
            jvm.put_field(&mut display, "nativeRepaintPending", "Z", true).await?;
            let mut graphics: ClassInstanceRef<()> = jvm.get_field(&display, "screenGraphics", "Ljavax/microedition/lcdui/Graphics;").await?;
            jvm.put_field(&mut graphics, "presentationOwner", "Ljavax/microedition/lcdui/Display;", display.clone())
                .await?;
        }
        jvm.put_field(&mut display, "repaintPending", "Z", true).await
    }

    pub(crate) async fn disable_midp_paint(jvm: &Jvm) -> JvmResult<()> {
        let midlet: ClassInstanceRef<MIDlet> = jvm
            .get_static_field("javax/microedition/midlet/MIDlet", "currentMIDlet", "Ljavax/microedition/midlet/MIDlet;")
            .await?;
        if midlet.is_null() {
            return Ok(());
        }
        let display = MIDlet::display(jvm, &midlet).await?;

        let mut graphics: ClassInstanceRef<()> = jvm.get_field(&display, "screenGraphics", "Ljavax/microedition/lcdui/Graphics;").await?;
        jvm.put_field(&mut graphics, "presentationOwner", "Ljavax/microedition/lcdui/Display;", display.clone())
            .await?;

        jvm.invoke_virtual(&display, "javax/microedition/lcdui/Display", "disablePaint", "()V", ())
            .await
    }

    pub fn class_definition_raw(definition: &dyn ClassDefinition) -> Result<u32> {
        Ok(if let Some(x) = definition.as_any().downcast_ref::<JavaClassDefinition>() {
            x.ptr_raw
        } else {
            let class = definition.as_any().downcast_ref::<JavaArrayClassDefinition>().unwrap();

            class.class.ptr_raw
        })
    }

    pub fn class_from_raw(core: &ArmCore, ptr_class: u32) -> JavaClassDefinition {
        JavaClassDefinition::from_raw(ptr_class, core)
    }

    pub fn read_name(core: &ArmCore, ptr_name: u32) -> Result<JavaFullName> {
        JavaFullName::from_ptr(core, ptr_name)
    }

    #[allow(clippy::borrowed_box)]
    pub fn class_instance_raw(instance: &Box<dyn ClassInstance>) -> u32 {
        if let Some(x) = instance.as_any().downcast_ref::<JavaClassInstance>() {
            x.ptr_raw
        } else {
            let instance = instance.as_any().downcast_ref::<JavaArrayClassInstance>().unwrap();

            instance.class_instance.ptr_raw
        }
    }

    pub fn current_java_exception_handler(core: &mut ArmCore) -> Result<u32> {
        let ptr_thread_context = Self::current_thread_context(core)?;
        let thread_context: KtfJvmThreadContext = read_generic(core, ptr_thread_context)?;

        Ok(thread_context.current_java_exception_handler)
    }

    pub fn set_current_thread_context(core: &mut ArmCore, ptr_thread_context: u32) -> Result<()> {
        write_generic(
            core,
            SUPPORT_CONTEXT_BASE + offset_of!(KtfJvmSupportContext, ptr_current_jvm_thread_context) as u32,
            ptr_thread_context,
        )
    }

    pub(crate) fn begin_native_return(core: &mut ArmCore) -> Result<Option<(u32, NativeReturnSlot)>> {
        let thread = Self::current_thread_context(core)?;
        if thread == 0 {
            return Ok(None);
        }
        let address = thread + offset_of!(KtfJvmThreadContext, native_return) as u32;
        let previous = read_generic(core, address)?;
        // Guest JNI functions publish a type at +0x24 and result words at
        // +0x28/+0x2c. Register-return adapters leave this marker untouched.
        write_generic(
            core,
            address,
            NativeReturnSlot {
                kind: u32::MAX,
                words: [0; 2],
            },
        )?;
        Ok(Some((address, previous)))
    }

    pub(crate) fn end_native_return(core: &mut ArmCore, saved: Option<(u32, NativeReturnSlot)>) -> Result<Option<[u32; 2]>> {
        let Some((address, previous)) = saved else {
            return Ok(None);
        };
        let returned: NativeReturnSlot = read_generic(core, address)?;
        // Restore the enclosing invocation even when the callee failed or made
        // nested JNI calls. Exception-handler fields are deliberately separate.
        write_generic(core, address, previous)?;
        Ok((returned.kind != u32::MAX).then_some(returned.words))
    }

    pub fn current_thread_context(core: &ArmCore) -> Result<u32> {
        let context_data: KtfJvmSupportContext = read_generic(core, SUPPORT_CONTEXT_BASE)?;

        Ok(context_data.ptr_current_jvm_thread_context)
    }
}

#[cfg(test)]
mod test {
    use alloc::{boxed::Box, sync::Arc, vec, vec::Vec};
    use core::{
        mem::{offset_of, size_of},
        sync::atomic::{AtomicBool, Ordering},
    };

    use bytemuck::Zeroable;
    use jvm::{ClassInstanceRef, JavaValue, Jvm, runtime::JavaLangString};
    use jvm_types::{ClassAccessFlags, MethodAccessFlags};
    use wipi_types::ktf::java::JavaMethodDefinition as RawJavaMethod;

    use wie_backend::{DefaultTaskRunner, System};
    use wie_core_arm::{Allocator, ArmCore};
    use wie_jvm_support::native::encode_method_arguments;
    use wie_midp::classes::javax::microedition::{lcdui::Display as MidpDisplay, midlet::MIDlet};
    use wie_util::{Result, WieError, read_generic, write_generic};

    use super::{
        JavaArrayClassInstance, JavaClassDefinition, JavaMethod, KtfJvmSupport, KtfJvmSupportContext, KtfJvmThreadContext, SUPPORT_CONTEXT_BASE,
        value::JavaValueCodec,
    };

    use test_utils::{TestClock, TestPlatform};

    #[derive(Clone)]
    struct RepaintContext {
        core: ArmCore,
        repaint: u32,
    }

    async fn repaint_canvas(
        jvm: &Jvm,
        context: &mut RepaintContext,
        mut this: ClassInstanceRef<()>,
        _graphics: ClassInstanceRef<()>,
    ) -> jvm::Result<()> {
        let paints: i32 = jvm.get_field(&this, "paints", "I").await?;
        jvm.put_field(&mut this, "paints", "I", paints + 1).await?;
        if jvm.get_field::<bool>(&this, "requeue", "Z").await? {
            jvm.put_field(&mut this, "requeue", "Z", false).await?;
            context.core.run_function::<u32>(context.repaint, &[0, 0, 0, 240, 320]).await.unwrap();
            assert_eq!(
                jvm.get_field::<i32>(&this, "paints", "I").await?,
                paints + 1,
                "native repaint must not recurse into paint"
            );
        }
        Ok(())
    }

    #[test]
    fn native_repaint_queues_callback_before_first_flush() -> Result<()> {
        use jvm_class_proto::{JavaClassProto, JavaFieldProto, JavaMethodProto};
        use jvm_types::FieldAccessFlags;
        use wie_backend::Event;
        use wie_jvm_support::JvmSupport;

        use crate::runtime::svc_ids::{WIPICGraphicsMethodId, WIPICTableId};

        let mut system = System::new(Box::new(TestPlatform::new()), "", "", DefaultTaskRunner);
        let task_system = system.clone();
        let done = Arc::new(AtomicBool::new(false));
        let finished = done.clone();
        system.spawn(async move || {
            // Keep the same native method registry for both runtime classes and
            // the synthetic Canvas, whose state and repaint count live in guest fields.
            let mut core = ArmCore::new(false, None)?;
            Allocator::init(&mut core)?;
            let mut registers = core.save_context();
            registers.sp = Allocator::alloc(&mut core, 0x1000)? + 0x1000;
            core.restore_context(&registers);
            let thread = Allocator::alloc(&mut core, size_of::<KtfJvmThreadContext>() as u32)?;
            write_generic(&mut core, thread, KtfJvmThreadContext::zeroed())?;
            KtfJvmSupport::set_current_thread_context(&mut core, thread)?;
            let implementation = super::KtfJvmImplementation::new(&mut core);
            let functions = implementation.java_functions();
            let jvm = JvmSupport::new_jvm(
                &task_system,
                None,
                Box::new([wie_wipi_java::get_protos().into(), wie_midp::get_protos().into()]),
                &[],
                implementation,
            )
            .await?;
            crate::runtime::wipi_c::register_wipic_svc_handler(&mut core, &task_system, &jvm)?;
            let repaint = core.make_svc_stub(
                crate::runtime::SVC_CATEGORY_WIPIC,
                WIPICTableId::Graphics.function_id(WIPICGraphicsMethodId::Repaint),
            )?;
            let proto = JavaClassProto {
                name: "test/NativeRepaintCanvas",
                parent_class: Some("javax/microedition/lcdui/Canvas"),
                interfaces: vec![],
                methods: vec![JavaMethodProto::new(
                    "paint",
                    "(Ljavax/microedition/lcdui/Graphics;)V",
                    repaint_canvas,
                    MethodAccessFlags::PROTECTED,
                )],
                fields: vec![
                    JavaFieldProto::new("paints", "I", FieldAccessFlags::PRIVATE),
                    JavaFieldProto::new("requeue", "Z", FieldAccessFlags::PRIVATE),
                ],
                access_flags: ClassAccessFlags::PUBLIC,
            };
            let context = Box::new(RepaintContext { core: core.clone(), repaint });
            let canvas_class = JavaClassDefinition::new(&mut core, &jvm, proto, context, functions).await?;
            jvm.register_class(Box::new(canvas_class), None).await.unwrap();
            let midlet: ClassInstanceRef<MIDlet> = jvm.new_class("net/wie/WIPIMIDlet", "()V", ()).await.unwrap().into();
            let display = MIDlet::display(&jvm, &midlet).await.unwrap();
            let mut canvas: ClassInstanceRef<()> = jvm.instantiate_class("test/NativeRepaintCanvas").await.unwrap().into();
            let _: () = jvm
                .invoke_special(&canvas, "javax/microedition/lcdui/Canvas", "<init>", "()V", ())
                .await
                .unwrap();
            let _: () = jvm
                .invoke_virtual(
                    &display,
                    "javax/microedition/lcdui/Display",
                    "setCurrent",
                    "(Ljavax/microedition/lcdui/Displayable;)V",
                    (canvas.clone(),),
                )
                .await
                .unwrap();
            let _: () = jvm
                .invoke_virtual(&display, "javax/microedition/lcdui/Display", "serviceRepaints", "()V", ())
                .await
                .unwrap();
            jvm.put_field(&mut canvas, "paints", "I", 0).await.unwrap();
            assert!(!jvm.get_field::<bool>(&display, "repaintPending", "Z").await.unwrap());
            assert!(!jvm.get_field::<bool>(&display, "paintDisabled", "Z").await.unwrap());

            let queue: ClassInstanceRef<()> = jvm
                .invoke_static("net/wie/EventQueue", "getEventQueue", "()Lnet/wie/EventQueue;", ())
                .await
                .unwrap();
            let event: ClassInstanceRef<()> = jvm.instantiate_array("I", 4).await.unwrap().into();
            for _ in 0..2 {
                core.run_function::<u32>(repaint, &[0, 0, 0, 240, 320]).await?;
            }
            assert!(jvm.get_field::<bool>(&display, "repaintPending", "Z").await.unwrap());
            assert!(!jvm.get_field::<bool>(&display, "paintDisabled", "Z").await.unwrap());
            assert_eq!(jvm.get_field::<i32>(&canvas, "paints", "I").await.unwrap(), 0);

            // TestScreen does not deliver frontend notifications, so supply the
            // actual backend Redraw and use the production queue conversion/dispatch.
            task_system.event_queue().push(Event::Redraw);
            let _: () = jvm
                .invoke_virtual(&queue, "net/wie/EventQueue", "getNextEvent", "([I)V", (event.clone(),))
                .await
                .unwrap();
            assert_eq!(jvm.load_array::<i32>(&event, 0, 1).await.unwrap(), [41]);
            let _: () = jvm
                .invoke_virtual(&queue, "net/wie/EventQueue", "dispatchEvent", "([I)V", (event.clone(),))
                .await
                .unwrap();
            assert_eq!(jvm.get_field::<i32>(&canvas, "paints", "I").await.unwrap(), 1);
            assert!(!jvm.get_field::<bool>(&display, "repaintPending", "Z").await.unwrap());
            let _: () = jvm
                .invoke_virtual(&queue, "net/wie/EventQueue", "dispatchEvent", "([I)V", (event.clone(),))
                .await
                .unwrap();
            assert_eq!(
                jvm.get_field::<i32>(&canvas, "paints", "I").await.unwrap(),
                1,
                "late redraw must not repaint a serviced request"
            );

            jvm.put_field(&mut canvas, "requeue", "Z", true).await.unwrap();
            core.run_function::<u32>(repaint, &[0, 0, 0, 240, 320]).await?;
            let _: () = jvm
                .invoke_virtual(&queue, "net/wie/EventQueue", "dispatchEvent", "([I)V", (event.clone(),))
                .await
                .unwrap();
            assert_eq!(jvm.get_field::<i32>(&canvas, "paints", "I").await.unwrap(), 2);
            assert!(
                jvm.get_field::<bool>(&display, "repaintPending", "Z").await.unwrap(),
                "request made during paint belongs to the next cycle"
            );
            let _: () = jvm
                .invoke_virtual(&queue, "net/wie/EventQueue", "dispatchEvent", "([I)V", (event.clone(),))
                .await
                .unwrap();
            assert_eq!(jvm.get_field::<i32>(&canvas, "paints", "I").await.unwrap(), 3);
            assert!(!jvm.get_field::<bool>(&display, "repaintPending", "Z").await.unwrap());
            finished.store(true, Ordering::Relaxed);
            Ok(())
        });
        while !done.load(Ordering::Relaxed) {
            system.tick()?;
        }
        Ok(())
    }

    #[test]
    fn native_repaint_presents_current_guest_framebuffer_after_empty_callback() -> Result<()> {
        use spin::Mutex;
        use wie_wipi_c::WIPICContext;
        use wipi_types::wipic::{WIPICFramebuffer, WIPICIndirectPtr};

        use crate::runtime::{
            svc_ids::{WIPICGraphicsMethodId, WIPICTableId},
            wipi_c::KtfWIPICContext,
        };

        let pixels = Arc::new(Mutex::new(Vec::new()));
        let captured = pixels.clone();
        let platform = TestPlatform::new().with_paint_handler(move |image| {
            let pixel = image.get_pixel(0, 0);
            captured.lock().push((pixel.r, pixel.g, pixel.b));
        });
        let mut system = System::new(Box::new(platform), "", "", DefaultTaskRunner);
        let mut task_system = system.clone();
        let done = Arc::new(AtomicBool::new(false));
        let finished = done.clone();
        system.spawn(async move || {
            let (jvm, mut core) = init_jvm(&mut task_system).await?;
            let midlet: ClassInstanceRef<MIDlet> = jvm.new_class("net/wie/WIPIMIDlet", "()V", ()).await.unwrap().into();
            let display = MIDlet::display(&jvm, &midlet).await.unwrap();
            let canvas: ClassInstanceRef<()> = jvm.new_class("net/wie/CardCanvas", "()V", ()).await.unwrap().into();
            let _: () = jvm
                .invoke_virtual(
                    &display,
                    "javax/microedition/lcdui/Display",
                    "setCurrent",
                    "(Ljavax/microedition/lcdui/Displayable;)V",
                    (canvas,),
                )
                .await
                .unwrap();
            let _: () = jvm
                .invoke_virtual(&display, "javax/microedition/lcdui/Display", "serviceRepaints", "()V", ())
                .await
                .unwrap();
            assert_eq!(pixels.lock().len(), 1);
            pixels.lock().clear();

            crate::runtime::wipi_c::register_wipic_svc_handler(&mut core, &task_system, &jvm)?;
            let get_framebuffer = core.make_svc_stub(
                crate::runtime::SVC_CATEGORY_WIPIC,
                WIPICTableId::Graphics.function_id(WIPICGraphicsMethodId::GetScreenFramebuffer),
            )?;
            let repaint = core.make_svc_stub(
                crate::runtime::SVC_CATEGORY_WIPIC,
                WIPICTableId::Graphics.function_id(WIPICGraphicsMethodId::Repaint),
            )?;
            let flush = core.make_svc_stub(
                crate::runtime::SVC_CATEGORY_WIPIC,
                WIPICTableId::Graphics.function_id(WIPICGraphicsMethodId::FlushLcd),
            )?;
            let framebuffer = core.run_function::<u32>(get_framebuffer, &[0]).await?;
            let mut context = KtfWIPICContext::new(core.clone(), task_system.clone(), jvm.clone());
            let shape: WIPICFramebuffer = read_generic(&context, context.data_ptr(WIPICIndirectPtr(framebuffer))?)?;
            assert_eq!(shape.bpp, 16);
            let pixel_address = context.data_ptr(shape.buf)?;
            write_generic(&mut context, pixel_address, 0xf800u16)?;

            // Drop the request's temporary Java roots before collection. The
            // presenter must remain reachable from the current Display field.
            let saved_context = core.save_context();
            jvm.push_native_frame();
            core.run_function::<u32>(repaint, &[0, 0, 0, shape.width, shape.height]).await?;
            let presenter: ClassInstanceRef<()> = jvm.get_field(&display, "nativePainter", "Ljava/lang/Runnable;").await.unwrap();
            assert!(!presenter.is_null());
            let presenter_raw = KtfJvmSupport::class_instance_raw(&presenter.into());
            jvm.pop_frame();
            core.restore_context(&saved_context);
            jvm.collect_garbage().unwrap();
            assert!(Allocator::is_allocated(&core, presenter_raw, 8)?);
            assert!(pixels.lock().is_empty(), "Repaint must defer presentation");
            assert!(jvm.get_field::<bool>(&display, "nativeRepaintPending", "Z").await.unwrap());

            // The presenter reads the guest buffer after the callback, rather
            // than retaining the red pixel that existed when Repaint was called.
            write_generic(&mut context, pixel_address, 0x07e0u16)?;
            let _: () = jvm
                .invoke_virtual(&display, "javax/microedition/lcdui/Display", "serviceRepaints", "()V", ())
                .await
                .unwrap();
            assert_eq!(&*pixels.lock(), &[(0, 255, 0)]);
            assert!(!jvm.get_field::<bool>(&display, "nativeRepaintPending", "Z").await.unwrap());
            assert!(jvm.get_field::<bool>(&display, "paintDisabled", "Z").await.unwrap());
            let _: () = jvm
                .invoke_virtual(&display, "javax/microedition/lcdui/Display", "serviceRepaints", "()V", ())
                .await
                .unwrap();
            assert_eq!(pixels.lock().len(), 1, "a serviced request must not present twice");
            // A paint that resumes after this native presentation must not
            // overwrite it with the untouched Java backing image.
            let _: () = jvm
                .invoke_virtual(&display, "javax/microedition/lcdui/Display", "handlePaintEvent", "()V", ())
                .await
                .unwrap();
            assert_eq!(&*pixels.lock(), &[(0, 255, 0)]);

            write_generic(&mut context, pixel_address, 0x001fu16)?;
            core.run_function::<u32>(flush, &[0, framebuffer, 0, 0, shape.width, shape.height])
                .await?;
            assert_eq!(&*pixels.lock(), &[(0, 255, 0), (0, 0, 255)]);
            assert!(jvm.get_field::<bool>(&display, "paintDisabled", "Z").await.unwrap());
            pixels.lock().clear();
            write_generic(&mut context, pixel_address, 0xf800u16)?;
            core.run_function::<u32>(repaint, &[0, 0, 0, shape.width, shape.height]).await?;
            assert!(pixels.lock().is_empty());
            let _: () = jvm
                .invoke_virtual(&display, "javax/microedition/lcdui/Display", "serviceRepaints", "()V", ())
                .await
                .unwrap();
            assert_eq!(&*pixels.lock(), &[(255, 0, 0)]);
            assert!(jvm.get_field::<bool>(&display, "paintDisabled", "Z").await.unwrap());
            pixels.lock().clear();
            write_generic(&mut context, pixel_address, 0x001fu16)?;
            // After native ownership is established, a Java repaint alone must
            // read current guest pixels; no extra native Flush/Repaint is needed.
            let _: () = jvm
                .invoke_virtual(&display, "javax/microedition/lcdui/Display", "repaint", "(IIII)V", (0, 0, 1, 1))
                .await
                .unwrap();
            assert!(pixels.lock().is_empty());
            let _: () = jvm
                .invoke_virtual(&display, "javax/microedition/lcdui/Display", "serviceRepaints", "()V", ())
                .await
                .unwrap();
            assert_eq!(&*pixels.lock(), &[(0, 0, 255)]);
            let _: () = jvm
                .invoke_virtual(&display, "javax/microedition/lcdui/Display", "handlePaintEvent", "()V", ())
                .await
                .unwrap();
            assert_eq!(&*pixels.lock(), &[(0, 0, 255)], "stale notification must not duplicate presentation");
            finished.store(true, Ordering::Relaxed);
            Ok(())
        });
        while !done.load(Ordering::Relaxed) {
            system.tick()?;
        }
        Ok(())
    }

    async fn init_jvm(system: &mut System) -> Result<(Jvm, ArmCore)> {
        let mut core = ArmCore::new(false, None)?;
        Allocator::init(&mut core)?;

        let mut context = core.save_context();
        let stack = Allocator::alloc(&mut core, 0x100)?;
        context.sp = stack + 0x100;
        core.restore_context(&context);

        let ptr_thread_context = Allocator::alloc(&mut core, size_of::<KtfJvmThreadContext>() as u32)?;
        write_generic(&mut core, ptr_thread_context, KtfJvmThreadContext::zeroed())?;
        KtfJvmSupport::set_current_thread_context(&mut core, ptr_thread_context)?;

        let (jvm, _) = KtfJvmSupport::init(&mut core, system, None).await?;

        Ok((jvm, core))
    }

    #[test]
    fn native_ktf_deferred_image_job_handles_missing_resource_and_releases_references() -> Result<()> {
        let mut system = System::new(Box::new(TestPlatform::new()), "", "", DefaultTaskRunner);
        let done = Arc::new(AtomicBool::new(false));
        let completed = done.clone();
        let mut task_system = system.clone();
        system.spawn(async move || {
            let (jvm, _) = init_jvm(&mut task_system).await?;
            let path = JavaLangString::from_rust_string(&jvm, "/missing-image-test.png").await.unwrap();
            let image: ClassInstanceRef<()> = jvm
                .invoke_static(
                    "org/kwis/msp/lcdui/Image",
                    "loadImage",
                    "(Ljava/lang/String;Lorg/kwis/msp/lcdui/ImageObserver;)Lorg/kwis/msp/lcdui/Image;",
                    (path, ClassInstanceRef::<()>::from(None)),
                )
                .await
                .unwrap();
            assert!(!image.is_null());
            assert_eq!(
                jvm.invoke_virtual::<_, i32>(&image, "org/kwis/msp/lcdui/Image", "getWidth", "()I", ())
                    .await
                    .unwrap(),
                0
            );
            let queue: ClassInstanceRef<()> = jvm
                .invoke_static("net/wie/EventQueue", "getEventQueue", "()Lnet/wie/EventQueue;", ())
                .await
                .unwrap();
            let pending: ClassInstanceRef<()> = jvm.get_field(&queue, "callSeriallyEvents", "Ljava/util/Vector;").await.unwrap();
            assert_eq!(
                jvm.invoke_virtual::<_, i32>(&pending, "java/util/Vector", "size", "()I", ())
                    .await
                    .unwrap(),
                1
            );
            let task: ClassInstanceRef<()> = jvm
                .invoke_virtual(&pending, "java/util/Vector", "remove", "(I)Ljava/lang/Object;", (0,))
                .await
                .unwrap();
            let _: () = jvm.invoke_virtual(&task, "java/lang/Runnable", "run", "()V", ()).await.unwrap();
            let head: ClassInstanceRef<()> = jvm
                .get_static_field("net/wie/ImageLoadTask", "head", "Lnet/wie/ImageLoadTask;")
                .await
                .unwrap();
            assert!(head.is_null());
            let held_image: ClassInstanceRef<()> = jvm.get_field(&task, "image", "Lorg/kwis/msp/lcdui/Image;").await.unwrap();
            assert!(held_image.is_null());
            assert_eq!(
                jvm.invoke_virtual::<_, i32>(&image, "org/kwis/msp/lcdui/Image", "getHeight", "()I", ())
                    .await
                    .unwrap(),
                0
            );
            completed.store(true, Ordering::Relaxed);
            Ok(())
        });
        while !done.load(Ordering::Relaxed) {
            system.tick()?;
        }
        Ok(())
    }

    #[test]
    fn native_ktf_form_title_constructor_preserves_stack_argument_and_guest_references() -> Result<()> {
        use alloc::collections::BTreeMap;
        use jvm_class_proto::JavaClassProto;
        let mut system = System::new(Box::new(TestPlatform::new()), "", "", DefaultTaskRunner);
        let done = Arc::new(AtomicBool::new(false));
        let completed = done.clone();
        let mut task_system = system.clone();
        system.spawn(async move || {
            let (jvm, mut core) = init_jvm(&mut task_system).await?;
            let proto: JavaClassProto<()> = JavaClassProto {
                name: "test/FormJlet",
                parent_class: Some("org/kwis/msp/lcdui/Jlet"),
                interfaces: vec![],
                methods: vec![],
                fields: vec![],
                access_flags: ClassAccessFlags::PUBLIC,
            };
            let class = super::JavaClassDefinition::new(&mut core, &jvm, proto, Box::new(()), Arc::new(spin::Mutex::new(BTreeMap::new()))).await?;
            jvm.register_class(Box::new(class), None).await.unwrap();
            let _ = jvm.new_class("net/wie/WIPIMIDlet", "()V", ()).await.unwrap();
            let jlet = jvm.instantiate_class("test/FormJlet").await.unwrap();
            let _: () = jvm.invoke_special(&jlet, "org/kwis/msp/lcdui/Jlet", "<init>", "()V", ()).await.unwrap();
            let title = JavaLangString::from_rust_string(&jvm, "제목").await.unwrap();
            let icon: ClassInstanceRef<()> = jvm
                .invoke_static("org/kwis/msp/lcdui/Image", "createImage", "(II)Lorg/kwis/msp/lcdui/Image;", (4, 4))
                .await
                .unwrap();
            let form = jvm
                .new_class(
                    "com/ktf/kfc/GFormBase",
                    "(Ljava/lang/String;Lorg/kwis/msp/lcdui/Image;I)V",
                    (title.clone(), icon.clone(), 0x12345678),
                )
                .await
                .unwrap();
            assert_eq!(
                jvm.invoke_virtual::<_, i32>(&form, "com/ktf/kfc/GForm", "getFormID", "()I", ())
                    .await
                    .unwrap(),
                0x12345678
            );
            let label: ClassInstanceRef<()> = jvm
                .invoke_virtual(&form, "org/kwis/msp/lwc/ShellComponent", "getTitle", "()Lorg/kwis/msp/lwc/Component;", ())
                .await
                .unwrap();
            let text: ClassInstanceRef<()> = jvm
                .invoke_virtual(&label, "org/kwis/msp/lwc/LabelComponent", "getLabel", "()Ljava/lang/String;", ())
                .await
                .unwrap();
            let image: ClassInstanceRef<()> = jvm
                .invoke_virtual(&label, "org/kwis/msp/lwc/LabelComponent", "getImage", "()Lorg/kwis/msp/lcdui/Image;", ())
                .await
                .unwrap();
            assert_eq!(text.identity(), title.identity());
            assert_eq!(image.identity(), icon.identity());
            let bar = jvm.new_class("org/kwis/msp/lwc/CommandBarComponent", "()V", ()).await.unwrap();
            let command = jvm
                .new_class(
                    "org/kwis/msp/lwc/Command",
                    "(Ljava/lang/String;Ljava/lang/Object;)V",
                    (title.clone(), form.clone()),
                )
                .await
                .unwrap();
            let index: i32 = jvm
                .invoke_virtual(
                    &bar,
                    "org/kwis/msp/lwc/CommandBarComponent",
                    "addCommand",
                    "(Lorg/kwis/msp/lwc/Command;)I",
                    (command.clone(),),
                )
                .await
                .unwrap();
            assert_eq!(index, 0);
            assert_eq!(
                jvm.invoke_virtual::<_, i32>(&bar, "org/kwis/msp/lwc/CommandBarComponent", "getActiveIndex", "()I", ())
                    .await
                    .unwrap(),
                -1
            );
            let _: () = jvm
                .invoke_virtual(
                    &form,
                    "org/kwis/msp/lwc/ShellComponent",
                    "setCommand",
                    "(Lorg/kwis/msp/lwc/Component;Z)V",
                    (bar.clone(), true),
                )
                .await
                .unwrap();
            let stored: ClassInstanceRef<()> = jvm
                .invoke_virtual(
                    &bar,
                    "org/kwis/msp/lwc/CommandBarComponent",
                    "getCommand",
                    "(I)Lorg/kwis/msp/lwc/Command;",
                    (0,),
                )
                .await
                .unwrap();
            assert_eq!(stored.identity(), command.identity());
            assert_eq!(
                jvm.get_static_field::<i32>("org/kwis/msp/lwc/CommandListener", "SELECT", "I")
                    .await
                    .unwrap(),
                2
            );
            assert_eq!(
                jvm.get_static_field::<i32>("org/kwis/msp/lwc/CommandListener", "FOCUS_CHANGE", "I")
                    .await
                    .unwrap(),
                1
            );
            let menu_form = jvm
                .new_class("com/ktf/kfc/GMenubarForm", "(Ljava/lang/String;)V", (title.clone(),))
                .await
                .unwrap();
            let _: () = jvm
                .invoke_virtual(&menu_form, "com/ktf/kfc/GMenubarForm", "setIMEButtonPos", "(I)V", (0,))
                .await
                .unwrap();
            let empty = JavaLangString::from_rust_string(&jvm, "").await.unwrap();
            let field = jvm
                .new_class(
                    "com/ktf/kfc/GTextField",
                    "(Lcom/ktf/kfc/GMenubarForm;Ljava/lang/String;I)V",
                    (menu_form.clone(), empty, 0),
                )
                .await
                .unwrap();
            let global_before: bool = jvm
                .get_static_field("javax/microedition/lcdui/Display", "koreanInput", "Z")
                .await
                .unwrap();
            let _: () = jvm
                .invoke_virtual(&field, "com/ktf/kfc/GTextField", "focusNotify", "(Z)V", (true,))
                .await
                .unwrap();
            let menu: ClassInstanceRef<()> = jvm
                .invoke_virtual(&menu_form, "com/ktf/kfc/GMenubarForm", "getGMenuBar", "()Lcom/ktf/kfc/GMenuBar;", ())
                .await
                .unwrap();
            let owner: ClassInstanceRef<()> = jvm
                .invoke_virtual(
                    &menu,
                    "com/ktf/kfc/GMenuBar",
                    "getCommandListener",
                    "()Lorg/kwis/msp/lwc/CommandListener;",
                    (),
                )
                .await
                .unwrap();
            assert_eq!(owner.identity(), menu_form.identity());
            for expected in ["A", "Aa", "Aa2"] {
                let mode_command: ClassInstanceRef<()> = jvm
                    .invoke_virtual(&menu, "com/ktf/kfc/GMenuBar", "itemAt", "(I)Lorg/kwis/msp/lwc/Command;", (0,))
                    .await
                    .unwrap();
                let _: () = jvm
                    .invoke_virtual(
                        &menu_form,
                        "com/ktf/kfc/GMenubarForm",
                        "commandAction",
                        "(Lorg/kwis/msp/lwc/Command;ILjava/lang/Object;)V",
                        (mode_command, 2, menu_form.clone()),
                    )
                    .await
                    .unwrap();
                for kind in [1, 2] {
                    assert!(
                        jvm.invoke_virtual::<_, bool>(&field, "com/ktf/kfc/GTextField", "keyNotify", "(II)Z", (kind, 50))
                            .await
                            .unwrap()
                    );
                }
                let value = jvm
                    .invoke_virtual(&field, "org/kwis/msp/lwc/TextComponent", "getString", "()Ljava/lang/String;", ())
                    .await
                    .unwrap();
                assert_eq!(JavaLangString::to_rust_string(&jvm, &value).await.unwrap(), expected);
            }
            assert_eq!(
                jvm.get_static_field::<bool>("javax/microedition/lcdui/Display", "koreanInput", "Z")
                    .await
                    .unwrap(),
                global_before
            );
            let _: () = jvm
                .invoke_virtual(&field, "com/ktf/kfc/GTextField", "focusNotify", "(Z)V", (true,))
                .await
                .unwrap();
            let mode_command: ClassInstanceRef<()> = jvm
                .invoke_virtual(&menu, "com/ktf/kfc/GMenuBar", "itemAt", "(I)Lorg/kwis/msp/lwc/Command;", (0,))
                .await
                .unwrap();
            let mode_name = jvm
                .invoke_virtual(&mode_command, "org/kwis/msp/lwc/Command", "getString", "()Ljava/lang/String;", ())
                .await
                .unwrap();
            assert_eq!(JavaLangString::to_rust_string(&jvm, &mode_name).await.unwrap(), "KO");
            // Route a complete physical soft-key sequence through the form:
            // release/typed must not cycle the mode a second time.
            for kind in [1, 2, 3] {
                assert!(
                    jvm.invoke_virtual::<_, bool>(&menu_form, "com/ktf/kfc/GMenubarForm", "keyNotify", "(II)Z", (kind, -6))
                        .await
                        .unwrap()
                );
            }
            assert_eq!(jvm.get_field::<i32>(&field, "wieMode", "I").await.unwrap(), 1);
            let _: () = jvm
                .invoke_virtual(&field, "com/ktf/kfc/GTextField", "focusNotify", "(Z)V", (false,))
                .await
                .unwrap();
            let focused: ClassInstanceRef<()> = jvm
                .get_field_in_class(&menu_form, "com/ktf/kfc/GMenubarForm", "m_cmpfocused", "Lorg/kwis/msp/lwc/Component;")
                .await
                .unwrap();
            assert!(focused.is_null());
            for kind in [1, 2, 3] {
                let _: bool = jvm
                    .invoke_virtual(&menu_form, "com/ktf/kfc/GMenubarForm", "keyNotify", "(II)Z", (kind, -6))
                    .await
                    .unwrap();
            }
            assert_eq!(jvm.get_field::<i32>(&field, "wieMode", "I").await.unwrap(), 1);
            completed.store(true, Ordering::Relaxed);
            Ok(())
        });
        while !done.load(Ordering::Relaxed) {
            system.tick()?;
        }
        Ok(())
    }

    #[test]
    fn native_card_fields_are_distinct_from_same_named_subclass_fields() -> Result<()> {
        use alloc::collections::BTreeMap;
        use jvm_class_proto::{JavaClassProto, JavaFieldProto};
        use jvm_types::FieldAccessFlags;
        let mut system = System::new(Box::new(TestPlatform::new()), "", "", DefaultTaskRunner);
        let done = Arc::new(AtomicBool::new(false));
        let completed = done.clone();
        let mut task_system = system.clone();
        system.spawn(async move || {
            let (jvm, mut core) = init_jvm(&mut task_system).await?;
            let proto: JavaClassProto<()> = JavaClassProto {
                name: "test/ShadowCard",
                parent_class: Some("org/kwis/msp/lcdui/Card"),
                interfaces: vec![],
                methods: vec![],
                fields: vec![
                    JavaFieldProto::new("x", "I", FieldAccessFlags::PROTECTED),
                    JavaFieldProto::new("y", "I", FieldAccessFlags::PROTECTED),
                    JavaFieldProto::new("w", "I", FieldAccessFlags::PROTECTED),
                    JavaFieldProto::new("h", "I", FieldAccessFlags::PROTECTED),
                    JavaFieldProto::new("display", "Lorg/kwis/msp/lcdui/Display;", FieldAccessFlags::PRIVATE),
                ],
                access_flags: ClassAccessFlags::PUBLIC,
            };
            // No methods are registered here; inherited bodies retain the real
            // KTF runtime's existing SVC table.
            let class = super::JavaClassDefinition::new(&mut core, &jvm, proto, Box::new(()), Arc::new(spin::Mutex::new(BTreeMap::new()))).await?;
            jvm.register_class(Box::new(class), None).await.unwrap();
            let display = jvm.instantiate_class("org/kwis/msp/lcdui/Display").await.unwrap();
            let mut card = jvm.instantiate_class("test/ShadowCard").await.unwrap();
            let _: () = jvm
                .invoke_special(
                    &card,
                    "org/kwis/msp/lcdui/Card",
                    "<init>",
                    "(Lorg/kwis/msp/lcdui/Display;IIIIZ)V",
                    (display.clone(), 0, 0, 176, 220, false),
                )
                .await
                .unwrap();
            for (field, value) in [("x", 176), ("y", 99), ("w", 220), ("h", 88)] {
                jvm.put_field(&mut card, field, "I", value).await.unwrap();
            }
            for (method, expected) in [("getX", 0), ("getY", 0), ("getWidth", 176), ("getHeight", 220)] {
                assert_eq!(
                    jvm.invoke_virtual::<_, i32>(&card, "org/kwis/msp/lcdui/Card", method, "()I", ())
                        .await
                        .unwrap(),
                    expected
                );
            }
            let base_display: ClassInstanceRef<()> = jvm
                .invoke_virtual(&card, "org/kwis/msp/lcdui/Card", "getDisplay", "()Lorg/kwis/msp/lcdui/Display;", ())
                .await
                .unwrap();
            assert_eq!(base_display.identity(), display.identity());
            assert!(
                jvm.get_field::<ClassInstanceRef<()>>(&card, "display", "Lorg/kwis/msp/lcdui/Display;")
                    .await
                    .unwrap()
                    .is_null()
            );
            let _: () = jvm
                .invoke_virtual(&card, "org/kwis/msp/lcdui/Card", "move", "(II)V", (3, 4))
                .await
                .unwrap();
            let _: () = jvm
                .invoke_virtual(&card, "org/kwis/msp/lcdui/Card", "resize", "(II)V", (50, 60))
                .await
                .unwrap();
            for (field, expected) in [("x", 176), ("y", 99), ("w", 220), ("h", 88)] {
                assert_eq!(jvm.get_field::<i32>(&card, field, "I").await.unwrap(), expected);
            }
            for (method, expected) in [("getX", 3), ("getY", 4), ("getWidth", 50), ("getHeight", 60)] {
                assert_eq!(
                    jvm.invoke_virtual::<_, i32>(&card, "org/kwis/msp/lcdui/Card", method, "()I", ())
                        .await
                        .unwrap(),
                    expected
                );
            }
            completed.store(true, Ordering::Relaxed);
            Ok(())
        });
        while !done.load(Ordering::Relaxed) {
            system.tick()?;
        }
        Ok(())
    }

    #[test]
    fn field_reads_observe_descriptor_changes_between_calls() -> Result<()> {
        use alloc::collections::BTreeMap;
        use jvm::{Field, JavaValue};
        use jvm_class_proto::{JavaClassProto, JavaFieldProto};
        use jvm_types::FieldAccessFlags;
        use wie_util::ByteWrite;
        use wipi_types::ktf::java::JavaFieldDefinition;
        let mut system = System::new(Box::new(TestPlatform::new()), "", "", DefaultTaskRunner);
        let done = Arc::new(AtomicBool::new(false));
        let completed = done.clone();
        let mut task_system = system.clone();
        system.spawn(async move || {
            let (jvm, mut core) = init_jvm(&mut task_system).await?;
            let proto: JavaClassProto<()> = JavaClassProto {
                name: "test/MutableFieldTypes",
                parent_class: Some("java/lang/Object"),
                interfaces: vec![],
                methods: vec![],
                fields: vec![
                    JavaFieldProto::new("word", "I", FieldAccessFlags::PUBLIC),
                    JavaFieldProto::new("wide", "J", FieldAccessFlags::PUBLIC),
                    JavaFieldProto::new("staticWord", "I", FieldAccessFlags::PUBLIC | FieldAccessFlags::STATIC),
                ],
                access_flags: ClassAccessFlags::PUBLIC,
            };
            let class = super::JavaClassDefinition::new(&mut core, &jvm, proto, Box::new(()), Arc::new(spin::Mutex::new(BTreeMap::new()))).await?;
            for (name, descriptor, is_static) in [
                ("word", "I", false),
                ("wide", "J", false),
                ("word", "J", false),
                ("missing", "I", false),
                ("staticWord", "I", true),
                ("staticWord", "I", false),
            ] {
                let expected = class
                    .fields()?
                    .into_iter()
                    .find(|field| {
                        field.matches_name(name, descriptor).unwrap() && field.access_flags().contains(FieldAccessFlags::STATIC) == is_static
                    })
                    .map(|field| field.ptr_raw);
                assert_eq!(class.field(name, descriptor, is_static)?.map(|field| field.ptr_raw), expected);
            }
            // A malformed table tail must still fault before an early match.
            use wipi_types::ktf::java::{JavaClass, JavaClassDescriptor};
            let raw_class: JavaClass = read_generic(&core, class.ptr_raw)?;
            let mut layout: JavaClassDescriptor = read_generic(&core, raw_class.ptr_descriptor)?;
            let original_table = layout.ptr_fields_or_element_type;
            let first_field: u32 = read_generic(&core, original_table)?;
            core.map(0x03000000, 0x10000)?;
            write_generic(&mut core, 0x0300fffc, first_field)?;
            layout.ptr_fields_or_element_type = 0x0300fffc;
            write_generic(&mut core, raw_class.ptr_descriptor, layout)?;
            assert!(class.fields().is_err());
            assert!(class.field("word", "I", false).is_err());
            layout.ptr_fields_or_element_type = original_table;
            write_generic(&mut core, raw_class.ptr_descriptor, layout)?;
            let word = class.field("word", "I", false)?.unwrap();
            let wide = class.field("wide", "J", false)?.unwrap();
            jvm.register_class(Box::new(class.clone()), None).await.unwrap();
            let mut instance = jvm.instantiate_class("test/MutableFieldTypes").await.unwrap();
            instance.put_field(&word, JavaValue::Int(0x3f800000)).unwrap();
            instance.put_field(&wide, JavaValue::Long(0x3ff0000000000000)).unwrap();
            assert!(matches!(instance.get_field(&word).unwrap(), JavaValue::Int(0x3f800000)));
            assert!(matches!(instance.get_field(&wide).unwrap(), JavaValue::Long(0x3ff0000000000000)));
            let raw_word: JavaFieldDefinition = read_generic(&core, word.ptr_raw)?;
            let raw_wide: JavaFieldDefinition = read_generic(&core, wide.ptr_raw)?;
            core.write_bytes(raw_word.ptr_name + 1, b"F")?;
            core.write_bytes(raw_wide.ptr_name + 1, b"D")?;
            assert!(class.field("word", "I", false)?.is_none());
            assert_eq!(class.field("word", "F", false)?.unwrap().ptr_raw, word.ptr_raw);
            assert!(matches!(instance.get_field(&word).unwrap(), JavaValue::Float(x) if x == 1.0));
            assert!(matches!(instance.get_field(&wide).unwrap(), JavaValue::Double(x) if x == 1.0));
            core.write_bytes(raw_word.ptr_name + 1, b"I")?;
            core.write_bytes(raw_wide.ptr_name + 1, b"J")?;
            assert!(matches!(instance.get_field(&word).unwrap(), JavaValue::Int(0x3f800000)));
            assert!(matches!(instance.get_field(&wide).unwrap(), JavaValue::Long(0x3ff0000000000000)));
            completed.store(true, Ordering::Relaxed);
            Ok(())
        });
        while !done.load(Ordering::Relaxed) {
            system.tick()?;
        }
        Ok(())
    }

    #[test]
    fn test_jvm_support() -> Result<()> {
        let mut system = System::new(Box::new(TestPlatform::new()), "", "", DefaultTaskRunner);

        let done = Arc::new(AtomicBool::new(false));

        let done_clone = done.clone();
        let mut system_clone = system.clone();
        system.spawn(async move || {
            let (jvm, mut core) = init_jvm(&mut system_clone).await?;

            let midlet: ClassInstanceRef<MIDlet> = jvm.new_class("net/wie/WIPIMIDlet", "()V", ()).await.unwrap().into();
            let display: ClassInstanceRef<MidpDisplay> = MIDlet::display(&jvm, &midlet).await.unwrap();
            let paint_disabled: bool = jvm.get_field(&display, "paintDisabled", "Z").await.unwrap();
            assert!(!paint_disabled);

            crate::runtime::wipi_c::register_wipic_svc_handler(&mut core, &system_clone, &jvm)?;
            use crate::runtime::svc_ids::{WIPICGraphicsMethodId, WIPICKernelMethodId, WIPICTableId};
            let access = core.make_svc_stub(
                crate::runtime::SVC_CATEGORY_WIPIC,
                WIPICTableId::Kernel.function_id(WIPICKernelMethodId::GetAccessLevel),
            )?;
            // The SDK query has no arguments. Residual register values must not
            // be interpreted as pointers. Missing metadata grants no optional access.
            assert_eq!(core.run_function::<u32>(access, &[u32::MAX, u32::MAX, u32::MAX, u32::MAX]).await?, 0);
            system_clone
                .filesystem()
                .add_virtual("__adf__", b"SLvl:7CBDAFBC\nSLvl2:00000FFF\n".to_vec());
            assert_eq!(core.run_function::<u32>(access, &[]).await?, 0x7cbdafbc);
            system_clone.filesystem().add_virtual("__adf__", b"SLvl:00000040\n".to_vec());
            assert_eq!(core.run_function::<u32>(access, &[]).await?, 0x40);

            // Exercise the real five-word SVC ABI, including the stack-passed capacity.
            use wie_util::{ByteRead, ByteWrite};
            let enumerate = core.make_svc_stub(
                crate::runtime::SVC_CATEGORY_WIPIC,
                WIPICTableId::Kernel.function_id(WIPICKernelMethodId::GetExecNames),
            )?;
            let output = Allocator::alloc(&mut core, 128)?;
            core.write_bytes(output, &[0x5a; 128])?;
            assert_eq!(core.run_function::<u32>(enumerate, &[0, 0, 0, output, 128]).await?, 0);
            let metadata = encoding_rs::EUC_KR
                .encode("Name:시험 게임\nVer:01.02\nVdr:제작사\nAID:TEST\n")
                .0
                .into_owned();
            system_clone.filesystem().add_virtual("__adf__", metadata);
            // Metadata alone is not an installed executable.
            assert_eq!(core.run_function::<u32>(enumerate, &[0, 0, 0, output, 128]).await?, 0);
            system_clone.filesystem().add_virtual("TEST.jar", vec![1]);
            assert_eq!(core.run_function::<u32>(enumerate, &[0, 0, 0, output, 14]).await?, (-18i32) as u32);
            let mut actual = [0u8; 128];
            core.read_bytes(output, &mut actual)?;
            assert_eq!(actual, [0x5a; 128]);
            assert_eq!(core.run_function::<u32>(enumerate, &[0, 0, 0, output, 15]).await?, 1);
            core.read_bytes(output, &mut actual)?;
            assert_eq!(&actual[..15], b"/TEST/TEST.jar\0");
            assert_eq!(&actual[15..], &[0x5a; 113]);
            let filters = Allocator::alloc(&mut core, 192)?;
            for (index, text) in ["TEST", "01.02", "제작사"].into_iter().enumerate() {
                let bytes = encoding_rs::EUC_KR.encode(text).0.into_owned();
                wie_util::write_null_terminated_string_bytes(&mut core, filters + index as u32 * 64, &bytes)?;
            }
            assert_eq!(
                core.run_function::<u32>(enumerate, &[filters, filters + 64, filters + 128, output, 128])
                    .await?,
                1
            );
            for index in 0..3 {
                let mut args = [0, 0, 0, output, 128];
                args[index] = filters + ((index + 1) % 3) as u32 * 64;
                core.write_bytes(output, &[0x5a; 128])?;
                assert_eq!(core.run_function::<u32>(enumerate, &args).await?, 0);
                core.read_bytes(output, &mut actual)?;
                assert_eq!(actual, [0x5a; 128]);
            }
            assert_eq!(core.run_function::<u32>(enumerate, &[0, 0, 0, output, u32::MAX]).await?, (-18i32) as u32);
            assert!(core.run_function::<u32>(enumerate, &[0xdead0000, 0, 0, output, 128]).await.is_err());
            assert!(core.run_function::<u32>(enumerate, &[0, 0, 0, 0xdead0000, 128]).await.is_err());

            // Carrier endpoint configuration owns a guest-memory copy, including
            // when the caller immediately reuses its input buffer.
            let endpoint = core.make_svc_stub(crate::runtime::SVC_CATEGORY_WIPIC, WIPICTableId::Net.function_id(34u16))?;
            let endpoint_input = Allocator::alloc(&mut core, 64)?;
            core.write_bytes(endpoint_input, b"gateway.example:27090\0")?;
            core.run_function::<u32>(endpoint, &[endpoint_input]).await?;
            let endpoint_root = SUPPORT_CONTEXT_BASE + offset_of!(KtfJvmSupportContext, ptr_network_endpoint) as u32;
            let endpoint_copy: u32 = read_generic(&mut core, endpoint_root)?;
            assert_ne!(endpoint_copy, endpoint_input);
            core.write_bytes(endpoint_input, b"other:1\0")?;
            assert_eq!(
                wie_util::read_null_terminated_string_bytes(&mut core, endpoint_copy)?,
                b"gateway.example:27090"
            );
            core.run_function::<u32>(endpoint, &[endpoint_input]).await?;
            let replacement: u32 = read_generic(&mut core, endpoint_root)?;
            assert_eq!(wie_util::read_null_terminated_string_bytes(&mut core, replacement)?, b"other:1");
            // Aliasing the stored value must remain safe during replacement.
            core.run_function::<u32>(endpoint, &[replacement]).await?;
            let replacement: u32 = read_generic(&mut core, endpoint_root)?;
            assert_eq!(wie_util::read_null_terminated_string_bytes(&mut core, replacement)?, b"other:1");
            assert!(core.run_function::<u32>(endpoint, &[0xdead0000]).await.is_err());
            assert_eq!(read_generic::<u32, _>(&mut core, endpoint_root)?, replacement);

            let framebuffer = core.make_svc_stub(
                crate::runtime::SVC_CATEGORY_WIPIC,
                WIPICTableId::Graphics.function_id(WIPICGraphicsMethodId::GetScreenFramebuffer),
            )?;
            let flush = core.make_svc_stub(
                crate::runtime::SVC_CATEGORY_WIPIC,
                WIPICTableId::Graphics.function_id(WIPICGraphicsMethodId::FlushLcd),
            )?;
            assert!(core.run_function::<u32>(flush, &[0, 0, 0, 0, 240, 320]).await.is_err());
            assert!(!jvm.get_field::<bool>(&display, "paintDisabled", "Z").await.unwrap());
            let framebuffer = core.run_function::<u32>(framebuffer, &[0]).await?;
            core.run_function::<u32>(flush, &[0, framebuffer, 0, 0, 240, 320]).await?;

            let paint_disabled: bool = jvm.get_field(&display, "paintDisabled", "Z").await.unwrap();
            assert!(paint_disabled);

            let screen: ClassInstanceRef<()> = jvm
                .get_field(&display, "screenGraphics", "Ljavax/microedition/lcdui/Graphics;")
                .await
                .unwrap();
            let image: ClassInstanceRef<()> = jvm
                .invoke_static(
                    "javax/microedition/lcdui/Image",
                    "createImage",
                    "(II)Ljavax/microedition/lcdui/Image;",
                    (2, 2),
                )
                .await
                .unwrap();
            let offscreen: ClassInstanceRef<()> = jvm
                .invoke_virtual(
                    &image,
                    "javax/microedition/lcdui/Image",
                    "getGraphics",
                    "()Ljavax/microedition/lcdui/Graphics;",
                    (),
                )
                .await
                .unwrap();
            let _: () = jvm
                .invoke_virtual(&offscreen, "javax/microedition/lcdui/Graphics", "fillRect", "(IIII)V", (0, 0, 1, 1))
                .await
                .unwrap();
            let _: () = jvm
                .invoke_virtual(&screen, "javax/microedition/lcdui/Graphics", "setClip", "(IIII)V", (0, 0, 2, 2))
                .await
                .unwrap();
            assert!(jvm.get_field::<bool>(&display, "paintDisabled", "Z").await.unwrap());
            let _: () = jvm
                .invoke_virtual(&screen, "javax/microedition/lcdui/Graphics", "fillRect", "(IIII)V", (0, 0, 1, 1))
                .await
                .unwrap();
            assert!(!jvm.get_field::<bool>(&display, "paintDisabled", "Z").await.unwrap());
            core.run_function::<u32>(flush, &[0, framebuffer, 0, 0, 240, 320]).await?;
            assert!(jvm.get_field::<bool>(&display, "paintDisabled", "Z").await.unwrap());

            let string1 = JavaLangString::from_rust_string(&jvm, "test1").await.unwrap();
            let string2 = JavaLangString::from_rust_string(&jvm, "test2").await.unwrap();

            let string3 = jvm
                .invoke_virtual(
                    &string1,
                    "java/lang/String",
                    "concat",
                    "(Ljava/lang/String;)Ljava/lang/String;",
                    [string2.into()],
                )
                .await
                .unwrap();

            assert_eq!(JavaLangString::to_rust_string(&jvm, &string3).await.unwrap(), "test1test2");

            let mut array = jvm.instantiate_array("S", 10).await.unwrap();
            jvm.store_array(&mut array, 0, (0..10i16).collect::<Vec<_>>()).await.unwrap();
            let temp: Vec<i16> = jvm.load_array(&array, 5, 4).await.unwrap();

            assert_eq!(temp, vec![5, 6, 7, 8]);

            // test 64bit parameter passing
            let date = jvm.new_class("java/util/Date", "(J)V", (0x12345678_abcdef01i64,)).await.unwrap();
            let time: i64 = jvm.invoke_virtual(&date, "java/util/Date", "getTime", "()J", ()).await.unwrap();

            assert_eq!(time, 0x12345678_abcdef01);

            let calendar = jvm.new_class("java/util/GregorianCalendar", "()V", ()).await.unwrap();
            assert!(jvm.is_instance(&*calendar, "java/util/Calendar"));
            let cloneable = jvm.resolve_class("java/lang/Cloneable").await.unwrap();
            assert!(cloneable.definition.access_flags().contains(ClassAccessFlags::INTERFACE));

            done_clone.store(true, Ordering::Relaxed);

            Ok(())
        });

        loop {
            system.tick()?;
            if done.load(Ordering::Relaxed) {
                break;
            }
        }

        Ok(())
    }

    #[test]
    fn test_native_method_entry_points() -> Result<()> {
        struct ReturnWords([u32; 2]);

        impl wie_core_arm::RunFunctionResult<ReturnWords> for ReturnWords {
            fn get(core: &ArmCore) -> Self {
                Self([core.read_param(0).unwrap(), core.read_param(1).unwrap()])
            }
        }

        let clock = TestClock::new();
        clock.set(0x12345678_9abcdef0);
        let mut system = System::new(Box::new(TestPlatform::with_clock(clock.clone())), "", "", DefaultTaskRunner);
        let done = Arc::new(AtomicBool::new(false));
        let done_clone = done.clone();
        let mut system_clone = system.clone();
        system.spawn(async move || {
            let (jvm, mut core) = init_jvm(&mut system_clone).await?;
            let runtime = jvm.new_class("java/lang/Runtime", "()V", ()).await.unwrap();
            let mut source = jvm.instantiate_array("I", 4).await.unwrap();
            let mut destination = jvm.instantiate_array("I", 4).await.unwrap();
            jvm.store_array(&mut source, 0, vec![11i32, 22, 33, 44]).await.unwrap();

            let class_name = JavaLangString::from_rust_string(&jvm, "java.lang.String").await.unwrap();
            let expected_class: ClassInstanceRef<()> = jvm
                .invoke_static(
                    "java/lang/Class",
                    "forName",
                    "(Ljava/lang/String;)Ljava/lang/Class;",
                    (class_name.clone(),),
                )
                .await
                .unwrap();
            let expected_class_word = encode_method_arguments(&JavaValueCodec::new(&core), &[expected_class.into()])[0];

            for (class_name, name, descriptor, is_static, args, expected) in [
                (
                    "java/lang/Runtime",
                    "totalMemory",
                    "()J",
                    false,
                    vec![JavaValue::from(runtime)],
                    vec![0x100000, 0],
                ),
                ("java/lang/System", "currentTimeMillis", "()J", true, vec![], vec![0x9abcdef0, 0x12345678]),
                (
                    "java/lang/Class",
                    "forName",
                    "(Ljava/lang/String;)Ljava/lang/Class;",
                    true,
                    vec![class_name.into()],
                    vec![expected_class_word],
                ),
                (
                    "java/lang/System",
                    "arraycopy",
                    "(Ljava/lang/Object;ILjava/lang/Object;II)V",
                    true,
                    vec![source.into(), 1.into(), destination.clone().into(), 0.into(), 2.into()],
                    vec![0],
                ),
            ] {
                let class = jvm.resolve_class(class_name).await.unwrap();
                let class = class.definition.as_any().downcast_ref::<JavaClassDefinition>().unwrap();
                let method = class.method(name, descriptor, is_static)?.unwrap();
                let raw: RawJavaMethod = read_generic(&core, method.ptr_raw)?;
                assert_eq!(
                    MethodAccessFlags::from_bits_truncate(raw.access_flags).contains(MethodAccessFlags::NATIVE),
                    name != "forName"
                );
                assert_ne!(raw.fn_body, 0);
                assert_ne!(raw.fn_body_native_or_exception_table, 0);
                assert_ne!(raw.fn_body, raw.fn_body_native_or_exception_table);

                for entry in 0..3 {
                    jvm.store_array(&mut destination, 0, vec![0i32; 4]).await.unwrap();
                    let codec = JavaValueCodec::new(&core);
                    let actual = if entry != 0 {
                        let words = encode_method_arguments(&codec, &args);
                        let buffer = Allocator::alloc(&mut core, (words.len().max(2) * 4) as u32)?;
                        for (i, word) in words.iter().enumerate() {
                            write_generic(&mut core, buffer + i as u32 * 4, *word)?;
                        }
                        let actual = if entry == 1 {
                            let result = core
                                .run_function::<ReturnWords>(raw.fn_body_native_or_exception_table, &[0, buffer])
                                .await?;
                            result.0
                        } else {
                            let trampoline = core.make_svc_stub(
                                crate::runtime::SVC_CATEGORY_JAVA_INTERFACE,
                                crate::runtime::svc_ids::JavaSvcId::CallNative,
                            )?;
                            let returned = core
                                .run_function::<u32>(trampoline, &[raw.fn_body_native_or_exception_table, buffer])
                                .await?;
                            assert_eq!(returned, buffer);
                            read_generic::<[u32; 2], _>(&core, buffer)?
                        };
                        Allocator::free(&mut core, buffer, (words.len().max(2) * 4) as u32)?;
                        actual[..expected.len()].to_vec()
                    } else {
                        let mut params = vec![0];
                        params.extend(encode_method_arguments(&codec, &args));
                        let result = core.run_function::<ReturnWords>(raw.fn_body, &params).await?;
                        result.0[..expected.len()].to_vec()
                    };
                    assert_eq!(actual, expected, "{name}, entry={entry}");
                    if name == "currentTimeMillis" && entry == 0 {
                        use crate::runtime::svc_ids::JavaSvcId;
                        for (svc, args) in [
                            (JavaSvcId::JavaJump1, vec![0, raw.fn_body]),
                            (JavaSvcId::JavaJump2, vec![0, 0, raw.fn_body]),
                            (JavaSvcId::JavaJump3, vec![0, 0, 0, raw.fn_body]),
                        ] {
                            let jump = core.make_svc_stub(crate::runtime::SVC_CATEGORY_JAVA_INTERFACE, svc)?;
                            assert_eq!(core.run_function::<ReturnWords>(jump, &args).await?.0.as_slice(), expected.as_slice());
                        }
                    }
                    if name == "arraycopy" {
                        assert_eq!(jvm.load_array::<i32>(&destination, 0, 4).await.unwrap(), vec![22, 33, 0, 0]);
                    }
                }
            }

            // A guest JNI function publishes its result in the thread context;
            // its CPU return registers may contain unrelated scratch values.
            assert_eq!(offset_of!(KtfJvmThreadContext, native_return), 0x24);
            assert_eq!(size_of::<KtfJvmThreadContext>(), 0x30);
            let thread = KtfJvmSupport::current_thread_context(&core)?;
            let slot = thread + 0x24;
            let prior = [2u32, 777, 0];
            write_generic(&mut core, slot, prior)?;
            let mut code = vec![
                0x03, 0x4a, // ldr r2, [pc, #12] (thread pointer)
                0x02, 0x23, // movs r3, #2 (integer result)
                0x53, 0x62, // str r3, [r2, #0x24]
                0x21, 0x23, // movs r3, #33
                0x93, 0x62, // str r3, [r2, #0x28]
                0xc7, 0x20, // movs r0, #199 (not the result)
                0x00, 0x21, // movs r1, #0
                0x70, 0x47, // bx lr
            ];
            code.extend_from_slice(&thread.to_le_bytes());
            core.load(&code, 0x100000, 0x1000)?;
            let trampoline = core.make_svc_stub(
                crate::runtime::SVC_CATEGORY_JAVA_INTERFACE,
                crate::runtime::svc_ids::JavaSvcId::CallNative,
            )?;
            let buffer = Allocator::alloc(&mut core, 8)?;
            write_generic(&mut core, buffer, [0u32; 2])?;
            core.run_function::<u32>(trampoline, &[0x100001, buffer]).await?;
            assert_eq!(read_generic::<[u32; 2], _>(&core, buffer)?, [33, 0]);
            assert_eq!(read_generic::<[u32; 3], _>(&core, slot)?, prior);

            // The Rust JVM entry must honor the same guest JNI convention.
            let class = jvm.resolve_class("java/lang/System").await.unwrap();
            let class = class.definition.as_any().downcast_ref::<JavaClassDefinition>().unwrap();
            let method = class.method("identityHashCode", "(Ljava/lang/Object;)I", true)?.unwrap();
            let original: RawJavaMethod = read_generic(&core, method.ptr_raw)?;
            let mut patched = original;
            patched.fn_body_native_or_exception_table = 0x100001;
            patched.access_flags |= MethodAccessFlags::NATIVE.bits();
            write_generic(&mut core, method.ptr_raw, patched)?;
            assert!(matches!(
                method.run(vec![JavaValue::Object(None)].into_boxed_slice()).await?,
                JavaValue::Int(33)
            ));
            write_generic(&mut core, method.ptr_raw, original)?;
            assert_eq!(read_generic::<[u32; 3], _>(&core, slot)?, prior);

            let outer = KtfJvmSupport::begin_native_return(&mut core)?;
            write_generic(&mut core, slot, [2u32, 123, 0])?;
            core.run_function::<u32>(trampoline, &[0x100001, buffer]).await?;
            assert_eq!(KtfJvmSupport::end_native_return(&mut core, outer)?, Some([123, 0]));
            assert_eq!(read_generic::<[u32; 3], _>(&core, slot)?, prior);

            // A fault must restore the enclosing slot and not publish a result.
            core.load(&[0x00, 0x22, 0x10, 0x68, 0x70, 0x47], 0x110000, 0x1000)?;
            write_generic(&mut core, buffer, [55u32, 66])?;
            assert!(core.run_function::<u32>(trampoline, &[0x110001, buffer]).await.is_err());
            assert_eq!(read_generic::<[u32; 2], _>(&core, buffer)?, [55, 66]);
            assert_eq!(read_generic::<[u32; 3], _>(&core, slot)?, prior);
            Allocator::free(&mut core, buffer, 8)?;

            // Exceptions must leave the packed argument/result buffer untouched.
            let class = jvm.resolve_class("java/lang/Class").await.unwrap();
            let class = class.definition.as_any().downcast_ref::<JavaClassDefinition>().unwrap();
            let method = class.method("forName", "(Ljava/lang/String;)Ljava/lang/Class;", true)?.unwrap();
            let raw: RawJavaMethod = read_generic(&core, method.ptr_raw)?;
            let missing = JavaLangString::from_rust_string(&jvm, "test.AbsentNativeClass").await.unwrap();
            let missing_word = encode_method_arguments(&JavaValueCodec::new(&core), &[missing.into()])[0];
            let buffer = Allocator::alloc(&mut core, 8)?;
            let before = [missing_word, 0x12345678];
            let trampoline = core.make_svc_stub(
                crate::runtime::SVC_CATEGORY_JAVA_INTERFACE,
                crate::runtime::svc_ids::JavaSvcId::CallNative,
            )?;
            for (entry, args) in [
                (raw.fn_body, vec![0, missing_word]),
                (raw.fn_body_native_or_exception_table, vec![0, buffer]),
                (trampoline, vec![raw.fn_body_native_or_exception_table, buffer]),
            ] {
                write_generic(&mut core, buffer, before)?;
                let error = core.run_function::<u32>(entry, &args).await.expect_err("missing class must throw");
                assert!(matches!(error, WieError::JavaException(_)), "{error:?}");
                let after: [u32; 2] = read_generic(&core, buffer)?;
                assert_eq!(after, before);
            }
            Allocator::free(&mut core, buffer, 8)?;

            crate::runtime::init::register_init_svc_handler(&mut core, &jvm)?;
            let throw = core.make_svc_stub(crate::runtime::SVC_CATEGORY_INIT, crate::runtime::svc_ids::InitSvcId::JavaThrowInstance)?;
            let exception = jvm.new_class("java/io/IOException", "()V", ()).await.unwrap();
            let exception_ptr = KtfJvmSupport::class_instance_raw(&exception);
            let error = core
                .run_function::<u32>(throw, &[exception_ptr, 0])
                .await
                .expect_err("athrow must propagate");
            assert!(matches!(error, WieError::JavaException(ptr) if ptr == exception_ptr));
            let error = core.run_function::<u32>(throw, &[0, 0]).await.expect_err("athrow null must throw NPE");
            let WieError::JavaException(ptr) = error else {
                panic!("{error:?}");
            };
            let exception = super::JavaClassInstance::from_raw(ptr, &core);
            assert_eq!(exception.class()?.name()?, "java/lang/NullPointerException");

            use wipi_types::ktf::java::{JavaExceptionHandler, JavaMethodExceptionTableEntry};
            let table = Allocator::alloc(&mut core, 4)?;
            let entry = Allocator::alloc(&mut core, size_of::<JavaMethodExceptionTableEntry>() as u32)?;
            write_generic(&mut core, table, entry)?;
            write_generic(
                &mut core,
                entry,
                JavaMethodExceptionTableEntry {
                    from_pc: 10,
                    to_pc: 20,
                    target: 15,
                    ptr_class: 0,
                },
            )?;
            let method_ptr = Allocator::alloc(&mut core, size_of::<RawJavaMethod>() as u32)?;
            let mut raw = RawJavaMethod::zeroed();
            raw.fn_body_native_or_exception_table = table;
            raw.exception_table_count = 1;
            write_generic(&mut core, method_ptr, raw)?;
            let functions = Allocator::alloc(&mut core, 8)?;
            write_generic(&mut core, functions, [0u32, 0x12345679])?;
            let handler_ptr = Allocator::alloc(&mut core, size_of::<JavaExceptionHandler>() as u32)?;
            let handler = JavaExceptionHandler {
                ptr_method: method_ptr,
                ptr_this: 0x1234,
                ptr_old_handler: 0,
                current_pc: 12,
                unk3: 0xdeadbeef,
                ptr_functions: functions,
                context: [0xabcdef; 11],
            };
            write_generic(&mut core, handler_ptr, handler)?;
            let thread_ptr = KtfJvmSupport::current_thread_context(&core)?;
            let mut thread: KtfJvmThreadContext = read_generic(&core, thread_ptr)?;
            thread.current_java_exception_handler = handler_ptr;
            write_generic(&mut core, thread_ptr, thread)?;
            let exception = jvm.new_class("java/io/IOException", "()V", ()).await.unwrap();
            let exception_ptr = KtfJvmSupport::class_instance_raw(&exception);
            let error = JavaMethod::handle_exception(&mut core, &jvm, exception)
                .await
                .err()
                .expect("matching handler must unwind");
            assert!(
                matches!(error, WieError::JavaExceptionUnwind { context_base, target: 15, next_pc: 0x12345679 } if context_base == handler_ptr + 24)
            );
            let after: JavaExceptionHandler = read_generic(&core, handler_ptr)?;
            assert_eq!(after.unk3, exception_ptr);
            assert_eq!(after.ptr_this, handler.ptr_this);
            assert_eq!(after.context, handler.context);
            thread.current_java_exception_handler = 0;
            write_generic(&mut core, thread_ptr, thread)?;

            done_clone.store(true, Ordering::Relaxed);
            clock.advance(16);
            Ok(())
        });

        while !done.load(Ordering::Relaxed) {
            system.tick()?;
        }
        Ok(())
    }

    #[test]
    fn test_exception_class_matches_raw_class_and_vtable() -> Result<()> {
        let mut system = System::new(Box::new(TestPlatform::new()), "", "", DefaultTaskRunner);

        let done = Arc::new(AtomicBool::new(false));

        let done_clone = done.clone();
        let mut system_clone = system.clone();
        system.spawn(async move || {
            let (jvm, mut core) = init_jvm(&mut system_clone).await?;

            let exception = jvm.new_class("java/lang/NullPointerException", "()V", ()).await.unwrap();
            let null_pointer_class = jvm
                .resolve_class("java/lang/NullPointerException")
                .await
                .unwrap()
                .definition
                .as_any()
                .downcast_ref::<JavaClassDefinition>()
                .unwrap()
                .clone();
            let runtime_exception_class = jvm
                .resolve_class("java/lang/RuntimeException")
                .await
                .unwrap()
                .definition
                .as_any()
                .downcast_ref::<JavaClassDefinition>()
                .unwrap()
                .clone();
            let illegal_argument_class = jvm
                .resolve_class("java/lang/IllegalArgumentException")
                .await
                .unwrap()
                .definition
                .as_any()
                .downcast_ref::<JavaClassDefinition>()
                .unwrap()
                .clone();

            assert!(JavaMethod::exception_class_matches(&core, &jvm, &*exception, 0)?);
            assert!(JavaMethod::exception_class_matches(&core, &jvm, &*exception, null_pointer_class.ptr_raw)?);
            assert!(JavaMethod::exception_class_matches(
                &core,
                &jvm,
                &*exception,
                null_pointer_class.ptr_vtable()?
            )?);
            assert!(JavaMethod::exception_class_matches(
                &core,
                &jvm,
                &*exception,
                runtime_exception_class.ptr_vtable()?
            )?);
            assert!(!JavaMethod::exception_class_matches(
                &core,
                &jvm,
                &*exception,
                illegal_argument_class.ptr_vtable()?
            )?);

            let ptr_exception = KtfJvmSupport::class_instance_raw(&exception);
            let result = JavaMethod::handle_exception(&mut core, &jvm, exception).await;
            assert!(matches!(result, Err(WieError::JavaException(ptr)) if ptr == ptr_exception));

            done_clone.store(true, Ordering::Relaxed);

            Ok(())
        });

        loop {
            system.tick()?;
            if done.load(Ordering::Relaxed) {
                break;
            }
        }

        Ok(())
    }

    #[test]
    fn test_long_array_store_load() -> Result<()> {
        let mut system = System::new(Box::new(TestPlatform::new()), "", "", DefaultTaskRunner);

        let done = Arc::new(AtomicBool::new(false));

        let done_clone = done.clone();
        let mut system_clone = system.clone();
        system.spawn(async move || {
            let (jvm, core) = init_jvm(&mut system_clone).await?;

            let values = vec![i64::MIN, -1, 0x12345678_9abcdef0, i64::MAX];

            let mut array = jvm.instantiate_array("J", 4).await.unwrap();
            jvm.store_array(&mut array, 0, values.clone()).await.unwrap();
            let loaded: Vec<i64> = jvm.load_array(&array, 0, 4).await.unwrap();

            assert_eq!(loaded, values);

            // guard against store/load flipping words symmetrically: check raw guest memory layout
            let array_instance = JavaArrayClassInstance::from_raw(KtfJvmSupport::class_instance_raw(&array), &core);
            let mut raw = [0u8; 8];
            array_instance.load_raw(16, &mut raw)?;
            assert_eq!(raw, 0x12345678_9abcdef0u64.to_le_bytes());

            done_clone.store(true, Ordering::Relaxed);

            Ok(())
        });

        loop {
            system.tick()?;
            if done.load(Ordering::Relaxed) {
                break;
            }
        }

        Ok(())
    }

    #[test]
    fn test_double_array_store_load() -> Result<()> {
        let mut system = System::new(Box::new(TestPlatform::new()), "", "", DefaultTaskRunner);

        let done = Arc::new(AtomicBool::new(false));

        let done_clone = done.clone();
        let mut system_clone = system.clone();
        system.spawn(async move || {
            let (jvm, _core) = init_jvm(&mut system_clone).await?;

            let values = vec![f64::MIN_POSITIVE, -1.5, f64::MAX];

            let mut array = jvm.instantiate_array("D", 3).await.unwrap();
            jvm.store_array(&mut array, 0, values.clone()).await.unwrap();
            let loaded: Vec<f64> = jvm.load_array(&array, 0, 3).await.unwrap();

            let to_bits = |x: &Vec<f64>| x.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
            assert_eq!(to_bits(&loaded), to_bits(&values));

            done_clone.store(true, Ordering::Relaxed);

            Ok(())
        });

        loop {
            system.tick()?;
            if done.load(Ordering::Relaxed) {
                break;
            }
        }

        Ok(())
    }
    #[test]
    fn native_static_reference_survives_gc_and_is_reclaimed_after_clear() -> Result<()> {
        let mut system = System::new(Box::new(TestPlatform::new()), "", "", DefaultTaskRunner);
        let mut system_clone = system.clone();
        let done = Arc::new(AtomicBool::new(false));
        let done_clone = done.clone();
        system.spawn(async move || {
            let (jvm, mut core) = init_jvm(&mut system_clone).await?;
            core.map(0x100000, 0x1000)?;
            KtfJvmSupport::set_native_image(&mut core, 0x100000, 0x1000)?;
            let saved_context = core.save_context();
            jvm.push_native_frame();
            let string = JavaLangString::from_rust_string(&jvm, "native static root").await.unwrap();
            let raw = KtfJvmSupport::class_instance_raw(&string.clone().into());
            write_generic(&mut core, 0x100100, raw)?;
            jvm.pop_frame();
            core.restore_context(&saved_context);
            jvm.collect_garbage().unwrap();
            assert!(Allocator::is_allocated(&core, raw, 8)?);
            jvm.push_native_frame();
            assert_eq!(JavaLangString::to_rust_string(&jvm, &string).await.unwrap(), "native static root");
            jvm.pop_frame();
            write_generic(&mut core, 0x100100, 0u32)?;
            core.restore_context(&saved_context);
            // Incomplete native memory snapshots must never cause live objects to be freed.
            KtfJvmSupport::set_native_image(&mut core, 0x200000, 4)?;
            assert_eq!(jvm.collect_garbage().unwrap(), 0);
            assert!(Allocator::is_allocated(&core, raw, 8)?);
            KtfJvmSupport::set_native_image(&mut core, 0x100000, 0x1000)?;
            jvm.collect_garbage().unwrap();
            assert!(!Allocator::is_allocated(&core, raw, 8)?);
            done_clone.store(true, Ordering::Relaxed);
            Ok(())
        });
        for _ in 0..1000 {
            system.tick()?;
            if done.load(Ordering::Relaxed) {
                break;
            }
        }
        assert!(done.load(Ordering::Relaxed));
        Ok(())
    }
}
