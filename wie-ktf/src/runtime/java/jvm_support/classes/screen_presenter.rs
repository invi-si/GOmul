use alloc::{string::ToString, vec};

use jvm::{ClassInstanceRef, Jvm, Result as JvmResult};
use jvm_class_proto::{JavaClassProto, JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use wie_util::read_generic;
use wie_wipi_c::{WIPICContext, api::graphics};
use wipi_types::wipic::{WIPICFramebuffer, WIPICIndirectPtr};

use super::net::wie::ClassLoaderContext;
use crate::runtime::wipi_c::KtfWIPICContext;

/// Host adapter for the guest-owned C screen buffer. Display invokes it only
/// after a requested native paint callback, unless that callback presents
/// explicitly or draws through Java Graphics instead.
pub struct KtfScreenPresenter;

impl KtfScreenPresenter {
    pub fn as_proto() -> JavaClassProto<ClassLoaderContext> {
        JavaClassProto {
            name: "net/wie/KtfScreenPresenter",
            parent_class: Some("java/lang/Object"),
            interfaces: vec!["java/lang/Runnable"],
            methods: vec![JavaMethodProto::new("run", "()V", Self::run, MethodAccessFlags::PUBLIC)],
            fields: vec![JavaFieldProto::new("framebuffer", "I", FieldAccessFlags::PRIVATE)],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }

    async fn run(jvm: &Jvm, context: &mut ClassLoaderContext, this: ClassInstanceRef<Self>) -> JvmResult<()> {
        let framebuffer: i32 = jvm.get_field(&this, "framebuffer", "I").await?;
        let mut context = KtfWIPICContext::new(context.core.clone(), context.system.clone(), jvm.clone());
        let result: wie_util::Result<()> = async {
            let framebuffer = WIPICIndirectPtr(framebuffer as u32);
            let shape: WIPICFramebuffer = read_generic(&context, context.data_ptr(framebuffer)?)?;
            graphics::flush_lcd(&mut context, 0, framebuffer, 0, 0, shape.width, shape.height).await
        }
        .await;
        match result {
            // Match an explicit native flush. In particular, a nested repaint
            // must not let its outer Java paint overwrite the presented buffer.
            Ok(()) => super::super::KtfJvmSupport::disable_midp_paint(jvm).await,
            Err(error) => Err(jvm.exception("net/wie/WieError", &error.to_string()).await),
        }
    }
}
