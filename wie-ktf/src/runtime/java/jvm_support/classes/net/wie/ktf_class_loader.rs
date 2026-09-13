use alloc::{boxed::Box, format, vec};

use bytemuck::cast_slice;
use jvm::{
    ClassInstanceRef, Jvm, Result as JvmResult,
    runtime::{JavaIoInputStream, JavaLangString},
};
use jvm_class_proto::{JavaClassProto, JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use rustjava_runtime::classes::java::lang::{Class, ClassLoader, String};

use wie_backend::System;
use wie_core_arm::{Allocator, ArmCore};
use wie_util::write_null_terminated_string_bytes;

use crate::runtime::{init::load_native, java::jvm_support::class_definition::JavaClassDefinition};

#[derive(Clone)]
pub struct ClassLoaderContext {
    pub core: ArmCore,
    pub system: System,
}

type ClassLoaderProto = JavaClassProto<ClassLoaderContext>;

// class net.wie.KtfClassLoader
pub struct KtfClassLoader;

impl KtfClassLoader {
    pub fn as_proto() -> ClassLoaderProto {
        ClassLoaderProto {
            name: "net/wie/KtfClassLoader",
            parent_class: Some("java/lang/ClassLoader"),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new(
                    "<init>",
                    "(Ljava/lang/ClassLoader;Ljava/lang/String;II)V",
                    Self::init,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new(
                    "loadClass",
                    "(Ljava/lang/String;)Ljava/lang/Class;",
                    Self::load_class,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new(
                    "findClass",
                    "(Ljava/lang/String;)Ljava/lang/Class;",
                    Self::find_class,
                    MethodAccessFlags::PROTECTED,
                ),
            ],
            fields: vec![
                JavaFieldProto::new("fnGetClass", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("nativeStrings", "Ljava/util/Vector;", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new(
                    "instance",
                    "Lnet/wie/KtfClassLoader;",
                    FieldAccessFlags::PRIVATE | FieldAccessFlags::STATIC,
                ),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }

    async fn init(
        jvm: &Jvm,
        context: &mut ClassLoaderContext,
        mut this: ClassInstanceRef<Self>,
        parent: ClassInstanceRef<ClassLoader>,
        binary_name: ClassInstanceRef<String>,
        ptr_jvm_context: i32,
        ptr_current_jvm_thread_context: i32,
    ) -> JvmResult<()> {
        tracing::debug!("net.wie.KtfClassLoader::<init>({this:?}, {parent:?}, {binary_name:?})");

        let _: () = jvm
            .invoke_special(&this, "java/lang/ClassLoader", "<init>", "(Ljava/lang/ClassLoader;)V", (parent,))
            .await?;

        let native_strings = jvm.new_class("java/util/Vector", "()V", ()).await?;
        jvm.put_field(&mut this, "nativeStrings", "Ljava/util/Vector;", native_strings).await?;

        jvm.put_static_field("net/wie/KtfClassLoader", "instance", "Lnet/wie/KtfClassLoader;", this.clone())
            .await?;

        // load client.bin
        let name_rust = JavaLangString::to_rust_string(jvm, &binary_name).await?;
        let data_stream = jvm
            .invoke_virtual(
                &this,
                "net/wie/KtfClassLoader",
                "getResourceAsStream",
                "(Ljava/lang/String;)Ljava/io/InputStream;",
                (binary_name,),
            )
            .await?;
        let data = JavaIoInputStream::read_until_end(jvm, &data_stream).await?;

        // load binary
        let native_functions = match load_native(
            &mut context.core,
            &mut context.system,
            jvm,
            &name_rust,
            cast_slice(&data),
            ptr_jvm_context as _,
            ptr_current_jvm_thread_context as _,
        )
        .await
        {
            Ok(functions) => functions,
            Err(error) => return Err(jvm.exception("net/wie/WieError", &format!("Native initialization failed: {error}")).await),
        };

        jvm.put_field(&mut this, "fnGetClass", "I", native_functions.fn_get_class as i32).await?;

        Ok(())
    }

    async fn load_class(
        jvm: &Jvm,
        context: &mut ClassLoaderContext,
        this: ClassInstanceRef<Self>,
        name: ClassInstanceRef<String>,
    ) -> JvmResult<ClassInstanceRef<Class>> {
        let internal_name = JavaLangString::to_rust_string(jvm, &name).await?.replace('.', "/");
        if jvm.has_class(&internal_name) {
            return Ok(jvm.resolve_class(&internal_name).await?.java_class().into());
        }

        // A KTF package can retain classfiles alongside their compiled definitions.
        // Use the image's exported class when present so native calls, inherited
        // fields and object identity all refer to the same guest-backed definition.
        // Arrays still belong to the JVM's ordinary array construction path.
        if !internal_name.starts_with('[') {
            let class = Self::find_class(jvm, context, this.clone(), name.clone()).await?;
            if !class.is_null() {
                tracing::debug!("KTF loaded exported class {internal_name}");
                return Ok(class);
            }
        }

        jvm.invoke_special(
            &this,
            "java/lang/ClassLoader",
            "loadClass",
            "(Ljava/lang/String;)Ljava/lang/Class;",
            (name,),
        )
        .await
    }

    async fn find_class(
        jvm: &Jvm,
        context: &mut ClassLoaderContext,
        this: ClassInstanceRef<Self>,
        name: ClassInstanceRef<String>,
    ) -> JvmResult<ClassInstanceRef<Class>> {
        tracing::debug!("net.wie.KtfClassLoader::findClass({this:?}, {name:?})");

        let fn_get_class: i32 = jvm.get_field(&this, "fnGetClass", "I").await?;

        if fn_get_class == 0 {
            return Ok(None.into());
        }

        // find from client.bin
        let name = JavaLangString::to_rust_string(jvm, &name).await?.replace('.', "/");

        let ptr_name_size = (name.len() + 1) as u32;
        let ptr_name = Allocator::alloc(&mut context.core, ptr_name_size).unwrap();
        write_null_terminated_string_bytes(&mut context.core, ptr_name, name.as_bytes()).unwrap();

        let ptr_raw = context.core.run_function(fn_get_class as _, &[ptr_name]).await.unwrap();
        Allocator::free(&mut context.core, ptr_name, ptr_name_size).unwrap();

        if ptr_raw != 0 {
            let class = JavaClassDefinition::from_raw(ptr_raw, &context.core);
            jvm.register_class(Box::new(class), Some(this.into())).await?;

            Ok(jvm.resolve_class(&name).await?.java_class().into())
        } else {
            Ok(None.into())
        }
    }
}
