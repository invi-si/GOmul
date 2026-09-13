#![no_std]
extern crate alloc;

mod context;
mod jvm_implementation;
pub mod native;
mod runtime;

use alloc::{boxed::Box, format, string::ToString};

use jvm::{JavaError, Jvm, runtime::JavaLangString};
use rustjava_runtime::Runtime;

use wie_backend::System;
use wie_util::{Result, WieError};

pub use context::{WieJavaClassProto, WieJvmContext};
pub use jvm_implementation::{JvmImplementation, RustJavaJvmImplementation};
use runtime::JvmRuntime;

pub static WIE_RUSTJAR: &str = "wie.rustjar";

pub struct JvmSupport;

impl JvmSupport {
    pub async fn new_jvm<T>(
        system: &System,
        jar_name: Option<&str>,
        protos: Box<[Box<[WieJavaClassProto]>]>,
        properties: &[(&str, &str)],
        implementation: T,
    ) -> Result<Jvm>
    where
        T: JvmImplementation + Sync + Send + 'static,
    {
        let runtime = JvmRuntime::new(system.clone(), implementation, protos);

        let class_path = if let Some(x) = jar_name {
            if cfg!(windows) {
                format!("{WIE_RUSTJAR};{x}")
            } else {
                format!("{WIE_RUSTJAR}:{x}")
            }
        } else {
            WIE_RUSTJAR.to_string()
        };

        let properties = [
            ("file.encoding", "EUC-KR"),
            ("java.class.path", &class_path),
            //("rustjava.disable_explicit_gc", "true"),
        ]
        .iter()
        .chain(properties.iter())
        .copied()
        .collect();
        let jvm = Jvm::new(
            rustjava_runtime::get_bootstrap_class_loader(Box::new(runtime.clone())),
            move || runtime.current_task_id(),
            properties,
        )
        .await
        .map_err(|x| WieError::FatalError(format!("Failed to create JVM: {x}")))?;

        Ok(jvm)
    }

    pub async fn to_wie_err(jvm: &Jvm, err: JavaError) -> WieError {
        match err {
            JavaError::JavaException(x) => {
                // Reporting a guest exception may itself invoke broken/missing guest
                // methods. Never turn that secondary failure into a host panic.
                let rendered: jvm::Result<alloc::string::String> = async {
                    let string_writer = jvm.new_class("java/io/StringWriter", "()V", ()).await?;
                    let print_writer = jvm
                        .new_class("java/io/PrintWriter", "(Ljava/io/Writer;)V", (string_writer.clone(),))
                        .await?;
                    let _: () = jvm
                        .invoke_virtual(&x, "java/lang/Throwable", "printStackTrace", "(Ljava/io/PrintWriter;)V", (print_writer,))
                        .await?;
                    let trace = jvm
                        .invoke_virtual(&string_writer, "java/io/StringWriter", "toString", "()Ljava/lang/String;", ())
                        .await?;
                    JavaLangString::to_rust_string(jvm, &trace).await
                }
                .await;
                match rendered {
                    Ok(trace) => WieError::FatalError(format!("\n{trace}")),
                    Err(secondary) => {
                        let class = x.class_definition().name();
                        let message = match jvm
                            .get_field::<jvm::ClassInstanceRef<JavaLangString>>(&x, "detailMessage", "Ljava/lang/String;")
                            .await
                        {
                            Ok(message) if !message.is_null() => JavaLangString::to_rust_string(jvm, &message).await.unwrap_or_default(),
                            _ => alloc::string::String::new(),
                        };
                        WieError::FatalError(format!("{class}: {message} (stack trace unavailable: {secondary:?})"))
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod reporting_tests {
    use super::*;
    use alloc::vec;
    use jvm::ClassInstanceRef;
    use jvm_class_proto::JavaMethodProto;
    use test_utils::run_jvm_test;

    async fn fail_trace<C: Send>(jvm: &Jvm, _: &mut C, _: ClassInstanceRef<()>, _: ClassInstanceRef<()>) -> jvm::Result<()> {
        Err(jvm.exception("java/lang/RuntimeException", "secondary failure").await)
    }

    #[test]
    fn failed_exception_formatter_preserves_original_error_without_panicking() -> Result<()> {
        let proto = jvm_class_proto::JavaClassProto {
            name: "test/BrokenTrace",
            parent_class: Some("java/lang/Exception"),
            interfaces: vec![],
            fields: vec![],
            methods: vec![JavaMethodProto::new(
                "printStackTrace",
                "(Ljava/io/PrintWriter;)V",
                fail_trace::<_>,
                Default::default(),
            )],
            access_flags: Default::default(),
        };
        run_jvm_test(Box::new([vec![proto].into()]), |jvm| async move {
            let mut error = jvm.instantiate_class("test/BrokenTrace").await?;
            let text = JavaLangString::from_rust_string(&jvm, "original failure").await?;
            jvm.put_field(&mut error, "detailMessage", "Ljava/lang/String;", text).await?;
            let actual = JvmSupport::to_wie_err(&jvm, JavaError::JavaException(error)).await.to_string();
            assert!(actual.contains("test/BrokenTrace: original failure"));
            assert!(actual.contains("stack trace unavailable"));
            let empty = jvm.instantiate_class("test/BrokenTrace").await?;
            let actual = JvmSupport::to_wie_err(&jvm, JavaError::JavaException(empty)).await.to_string();
            assert!(actual.contains("test/BrokenTrace:"));
            Ok(())
        })
    }
}
