use alloc::{boxed::Box, sync::Arc, vec};
use core::{
    mem::size_of,
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
};

use bytemuck::Zeroable;
use jvm::{ClassInstanceRef, Jvm, Result as JvmResult, runtime::JavaLangString};
use jvm_class_proto::{JavaClassProto, JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use rustjava_runtime::classes::java::lang::{Class, String};
use test_utils::TestPlatform;
use wie_backend::{DefaultTaskRunner, System};
use wie_core_arm::{Allocator, ArmCore};
use wie_jvm_support::{JvmSupport, native::NativeJavaValueCodec};
use wie_util::{Result, read_null_terminated_string_bytes, write_generic};

use super::{
    JavaClassDefinition, KtfJvmSupport, KtfJvmThreadContext,
    classes::net::wie::{ClassLoaderContext, KtfClassLoader},
    jvm_implementation::KtfJvmImplementation,
    value::JavaValueCodec,
};

#[derive(Clone)]
struct ParentContext {
    core: ArmCore,
    shadow: u32,
    fallback: u32,
    calls: Arc<AtomicU32>,
}

async fn parent_load(
    jvm: &Jvm,
    context: &mut ParentContext,
    this: ClassInstanceRef<()>,
    name: ClassInstanceRef<String>,
) -> JvmResult<ClassInstanceRef<Class>> {
    context.calls.fetch_add(1, Ordering::Relaxed);
    let name = JavaLangString::to_rust_string(jvm, &name).await?.replace('.', "/");
    for ptr in [context.shadow, context.fallback] {
        let class = JavaClassDefinition::from_raw(ptr, &context.core);
        if class.name().unwrap() == name {
            return Ok(jvm.register_class(Box::new(class), Some(this.into())).await?.into());
        }
    }
    Ok(None.into())
}

async fn exported_class(core: &mut ArmCore, context: &mut (u32, Arc<AtomicU32>), ptr_name: u32) -> Result<u32> {
    context.1.fetch_add(1, Ordering::Relaxed);
    let name = read_null_terminated_string_bytes(core, ptr_name)?;
    Ok(if name == b"test/Exported" { context.0 } else { 0 })
}

#[test]
fn native_exports_precede_parent_copies_and_preserve_guest_identity() -> Result<()> {
    let mut system = System::new(Box::new(TestPlatform::new()), "", "", DefaultTaskRunner);
    let task_system = system.clone();
    let done = Arc::new(AtomicBool::new(false));
    let task_done = done.clone();
    system.spawn(async move || {
        let mut core = ArmCore::new(false, None)?;
        Allocator::init(&mut core)?;
        let mut cpu = core.save_context();
        cpu.sp = Allocator::alloc(&mut core, 0x1000)? + 0x1000;
        core.restore_context(&cpu);
        let thread = Allocator::alloc(&mut core, size_of::<KtfJvmThreadContext>() as u32)?;
        write_generic(&mut core, thread, KtfJvmThreadContext::zeroed())?;
        KtfJvmSupport::set_current_thread_context(&mut core, thread)?;
        let implementation = KtfJvmImplementation::new(&mut core);
        let functions = implementation.java_functions();
        let protos = [wie_wipi_java::get_protos().into(), wie_midp::get_protos().into()];
        let jvm = JvmSupport::new_jvm(&task_system, None, Box::new(protos), &[], implementation).await?;

        // Two sources export the same name, as in a JAR retaining the compiled
        // image's original class. Distinct native definitions make precedence
        // observable without embedding a proprietary game or copying its code.
        let mut definitions = vec![];
        for name in ["test/Exported", "test/Exported", "test/Fallback"] {
            let proto: JavaClassProto<()> = JavaClassProto {
                name,
                parent_class: Some("java/util/Vector"),
                interfaces: vec![],
                methods: vec![],
                fields: vec![JavaFieldProto::new("link", "Ljava/lang/Object;", FieldAccessFlags::PUBLIC)],
                access_flags: ClassAccessFlags::PUBLIC,
            };
            definitions.push(JavaClassDefinition::new(&mut core, &jvm, proto, Box::new(()), functions.clone()).await?);
        }
        let native = definitions[0].ptr_raw;
        let parent_calls = Arc::new(AtomicU32::new(0));
        let parent_proto = JavaClassProto {
            name: "test/ParentLoader",
            parent_class: Some("java/lang/ClassLoader"),
            interfaces: vec![],
            methods: vec![JavaMethodProto::new(
                "loadClass",
                "(Ljava/lang/String;)Ljava/lang/Class;",
                parent_load,
                MethodAccessFlags::PUBLIC,
            )],
            fields: vec![],
            access_flags: ClassAccessFlags::PUBLIC,
        };
        let parent_context = ParentContext {
            core: core.clone(),
            shadow: definitions[1].ptr_raw,
            fallback: definitions[2].ptr_raw,
            calls: parent_calls.clone(),
        };
        let parent_class = JavaClassDefinition::new(&mut core, &jvm, parent_proto, Box::new(parent_context), functions.clone()).await?;
        jvm.register_class(Box::new(parent_class), None).await.unwrap();
        let parent = jvm.instantiate_class("test/ParentLoader").await.unwrap();
        let loader_context = Box::new(ClassLoaderContext {
            core: core.clone(),
            system: task_system.clone(),
        });
        let loader_class = JavaClassDefinition::new(&mut core, &jvm, KtfClassLoader::as_proto(), loader_context, functions).await?;
        jvm.register_class(Box::new(loader_class), None).await.unwrap();
        let mut loader = jvm.instantiate_class("net/wie/KtfClassLoader").await.unwrap();
        jvm.put_field(&mut loader, "parent", "Ljava/lang/ClassLoader;", parent).await.unwrap();
        let lookups = Arc::new(AtomicU32::new(0));
        core.register_svc_handler(20, exported_class, &(native, lookups.clone()))?;
        let lookup = core.make_svc_stub(20, 0u32)?;
        jvm.put_field(&mut loader, "fnGetClass", "I", lookup as i32).await.unwrap();
        let name = JavaLangString::from_rust_string(&jvm, "test.Exported").await.unwrap();
        let loaded: ClassInstanceRef<Class> = jvm
            .invoke_virtual(
                &loader,
                "java/lang/ClassLoader",
                "loadClass",
                "(Ljava/lang/String;)Ljava/lang/Class;",
                (name.clone(),),
            )
            .await
            .unwrap();
        assert!(!loaded.is_null());
        assert_eq!(parent_calls.load(Ordering::Relaxed), 0);
        let definition = jvm.resolve_class("test/Exported").await.unwrap().definition;
        assert_eq!(definition.as_any().downcast_ref::<JavaClassDefinition>().unwrap().ptr_raw, native);
        let before = lookups.load(Ordering::Relaxed);
        let again: ClassInstanceRef<Class> = jvm
            .invoke_virtual(
                &loader,
                "java/lang/ClassLoader",
                "loadClass",
                "(Ljava/lang/String;)Ljava/lang/Class;",
                (name,),
            )
            .await
            .unwrap();
        assert_eq!(loaded.identity(), again.identity());
        assert_eq!(lookups.load(Ordering::Relaxed), before);

        // Inherited fields/methods and references use one guest-memory identity.
        let mut object = jvm.instantiate_class("test/Exported").await.unwrap();
        let _: () = jvm.invoke_special(&object, "java/util/Vector", "<init>", "()V", ()).await.unwrap();
        let value = JavaLangString::from_rust_string(&jvm, "retained").await.unwrap();
        jvm.put_field(&mut object, "link", "Ljava/lang/Object;", value.clone()).await.unwrap();
        let _: () = jvm
            .invoke_virtual(&object, "java/util/Vector", "addElement", "(Ljava/lang/Object;)V", (value.clone(),))
            .await
            .unwrap();
        let codec = JavaValueCodec::new(&core);
        let roundtrip = codec.object_from_raw(codec.object_to_raw(&*object));
        assert_eq!(object.identity(), roundtrip.identity());
        let size: i32 = jvm.invoke_virtual(&roundtrip, "java/util/Vector", "size", "()I", ()).await.unwrap();
        assert_eq!(size, 1);
        let element: ClassInstanceRef<String> = jvm
            .invoke_virtual(&roundtrip, "java/util/Vector", "elementAt", "(I)Ljava/lang/Object;", (0,))
            .await
            .unwrap();
        let link: ClassInstanceRef<String> = jvm.get_field(&roundtrip, "link", "Ljava/lang/Object;").await.unwrap();
        assert_eq!(value.identity(), element.identity());
        assert_eq!(value.identity(), link.identity());
        let reference: ClassInstanceRef<()> = object.clone().into();
        let root = jvm.new_global_ref(&reference);
        jvm.collect_garbage().unwrap();
        assert_eq!(JavaLangString::to_rust_string(&jvm, &link).await.unwrap(), "retained");
        drop(root);

        // A real parent fallback remains available when the image has no export.
        let name = JavaLangString::from_rust_string(&jvm, "test/Fallback").await.unwrap();
        let fallback: ClassInstanceRef<Class> = jvm
            .invoke_virtual(
                &loader,
                "java/lang/ClassLoader",
                "loadClass",
                "(Ljava/lang/String;)Ljava/lang/Class;",
                (name,),
            )
            .await
            .unwrap();
        assert!(!fallback.is_null());
        assert_eq!(parent_calls.load(Ordering::Relaxed), 1);
        let name = JavaLangString::from_rust_string(&jvm, "[Ltest/Exported;").await.unwrap();
        let before = lookups.load(Ordering::Relaxed);
        let array: ClassInstanceRef<Class> = jvm
            .invoke_virtual(
                &loader,
                "java/lang/ClassLoader",
                "loadClass",
                "(Ljava/lang/String;)Ljava/lang/Class;",
                (name,),
            )
            .await
            .unwrap();
        assert!(!array.is_null());
        assert_eq!(lookups.load(Ordering::Relaxed), before);
        task_done.store(true, Ordering::Relaxed);
        Ok(())
    });
    while !done.load(Ordering::Relaxed) {
        system.tick()?;
    }
    Ok(())
}
