use alloc::vec;

use jvm::{ClassInstanceRef, Jvm, Result as JvmResult};
use jvm_class_proto::JavaMethodProto;
use jvm_types::{ClassAccessFlags, MethodAccessFlags};

use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

// class org.kwis.msp.lwc.AnnunciatorComponent
pub struct AnnunciatorComponent;

impl AnnunciatorComponent {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "org/kwis/msp/lwc/AnnunciatorComponent",
            parent_class: Some("org/kwis/msp/lwc/ShellComponent"),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("<init>", "(Z)V", Self::init, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("show", "()V", Self::show, MethodAccessFlags::PUBLIC),
            ],
            fields: vec![],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }

    async fn init(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<AnnunciatorComponent>, a0: bool) -> JvmResult<()> {
        tracing::warn!("stub org.kwis.msp.lwc.AnnunciatorComponent::<init>({this:?}, {a0})");

        let _: () = jvm.invoke_special(&this, "org/kwis/msp/lwc/ShellComponent", "<init>", "()V", ()).await?;

        // This is a device status strip, not a full-screen shell. Games subtract
        // its height from their Card height even before any component is shown.
        // Match the emulated phone display profiles used for annunciator geometry.
        let width: i32 = jvm.get_field(&this, "w", "I").await?;
        let screen_height: i32 = jvm.get_field(&this, "h", "I").await?;
        let height = match width {
            120 => 14,
            176 | 220 => 20,
            _ => 24,
        };
        jvm.put_field(&mut this, "h", "I", height.min(screen_height)).await?;
        Ok(())
    }

    async fn show(_: &Jvm, _: &mut WieJvmContext) -> JvmResult<()> {
        tracing::warn!("stub org.kwis.msp.lwc.AnnunciatorComponent::show()");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;
    use test_utils::run_jvm_test;

    #[test]
    fn status_strip_leaves_positive_card_area_before_and_after_show() -> wie_util::Result<()> {
        run_jvm_test(
            Box::new([
                wie_midp::get_protos().into(),
                crate::classes::org::kwis::msp::lwc::shell_component::test_support::protos(),
            ]),
            |jvm| async move {
                crate::classes::org::kwis::msp::lwc::shell_component::test_support::init(&jvm).await?;
                let display: ClassInstanceRef<()> = jvm
                    .invoke_static("org/kwis/msp/lcdui/Display", "getDefaultDisplay", "()Lorg/kwis/msp/lcdui/Display;", ())
                    .await?;
                let width: i32 = jvm.invoke_virtual(&display, "org/kwis/msp/lcdui/Display", "getWidth", "()I", ()).await?;
                let height: i32 = jvm.invoke_virtual(&display, "org/kwis/msp/lcdui/Display", "getHeight", "()I", ()).await?;
                for transparent in [false, true] {
                    let strip = jvm.new_class("org/kwis/msp/lwc/AnnunciatorComponent", "(Z)V", (transparent,)).await?;
                    let strip_height: i32 = jvm.invoke_virtual(&strip, "org/kwis/msp/lwc/Component", "getHeight", "()I", ()).await?;
                    assert!(strip_height > 0 && strip_height < height);
                    assert_eq!(
                        jvm.invoke_virtual::<_, i32>(&strip, "org/kwis/msp/lwc/Component", "getWidth", "()I", ())
                            .await?,
                        width
                    );
                    let _: () = jvm
                        .invoke_virtual(&strip, "org/kwis/msp/lwc/AnnunciatorComponent", "show", "()V", ())
                        .await?;
                    assert_eq!(
                        jvm.invoke_virtual::<_, i32>(&strip, "org/kwis/msp/lwc/Component", "getHeight", "()I", ())
                            .await?,
                        strip_height
                    );
                    let card = jvm.instantiate_class("net/wie/LwcCard").await?;
                    let _: () = jvm
                        .invoke_special(
                            &card,
                            "org/kwis/msp/lcdui/Card",
                            "<init>",
                            "(IIII)V",
                            (0, strip_height, width, height - strip_height),
                        )
                        .await?;
                    assert_eq!(
                        jvm.invoke_virtual::<_, i32>(&card, "org/kwis/msp/lcdui/Card", "getHeight", "()I", ())
                            .await?,
                        height - strip_height
                    );
                }
                // Ordinary shells still fill the display.
                let shell = jvm.new_class("org/kwis/msp/lwc/ShellComponent", "()V", ()).await?;
                assert_eq!(
                    jvm.invoke_virtual::<_, i32>(&shell, "org/kwis/msp/lwc/Component", "getHeight", "()I", ())
                        .await?,
                    height
                );
                Ok(())
            },
        )
    }
}
