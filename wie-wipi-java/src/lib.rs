#![no_std]

extern crate alloc;

pub mod classes;

use wie_jvm_support::WieJavaClassProto;

pub fn get_protos() -> [WieJavaClassProto; 61] {
    [
        crate::classes::net::wie::ImageLoadTask::as_proto(),
        crate::classes::org::kwis::msp::lwc::TextEditor::as_proto(),
        crate::classes::java::io::UnavailableException::as_proto(),
        crate::classes::org::kwis::msf::io::Message::as_proto(),
        crate::classes::org::kwis::msf::io::Network::as_proto(),
        crate::classes::org::kwis::msf::io::SchemeNotFoundException::as_proto(),
        crate::classes::org::kwis::msf::io::Socket::as_proto(),
        crate::classes::org::kwis::msf::io::URL::as_proto(),
        crate::classes::org::kwis::msp::db::DataBase::as_proto(),
        crate::classes::org::kwis::msp::db::DataComparator::as_proto(),
        crate::classes::org::kwis::msp::db::DataFilter::as_proto(),
        crate::classes::org::kwis::msp::db::DataBaseException::as_proto(),
        crate::classes::org::kwis::msp::db::DataBaseRecordException::as_proto(),
        crate::classes::org::kwis::msp::handset::BackLight::as_proto(),
        crate::classes::org::kwis::msp::handset::LED::as_proto(),
        crate::classes::org::kwis::msp::media::MediaUnsupportedException::as_proto(),
        crate::classes::org::kwis::msp::lwc::ActionListener::as_proto(),
        crate::classes::org::kwis::msp::lwc::GrabKeyListener::as_proto(),
        crate::classes::org::kwis::msp::handset::HandsetProperty::as_proto(),
        crate::classes::org::kwis::msp::io::File::as_proto(),
        crate::classes::org::kwis::msp::io::FileSystem::as_proto(),
        crate::classes::org::kwis::msp::lcdui::Card::as_proto(),
        crate::classes::org::kwis::msp::lcdui::Display::as_proto(),
        crate::classes::org::kwis::msp::lcdui::EventQueue::as_proto(),
        crate::classes::org::kwis::msp::lcdui::Font::as_proto(),
        crate::classes::org::kwis::msp::lcdui::Graphics::as_proto(),
        crate::classes::org::kwis::msp::lcdui::Image::as_proto(),
        crate::classes::org::kwis::msp::lcdui::ImageObserver::as_proto(),
        crate::classes::org::kwis::msp::lcdui::InputMethodHandler::as_proto(),
        crate::classes::org::kwis::msp::lcdui::InputMethodListener::as_proto(),
        crate::classes::org::kwis::msp::lcdui::Main::as_proto(),
        crate::classes::org::kwis::msp::lcdui::Jlet::as_proto(),
        crate::classes::org::kwis::msp::lcdui::JletEventListener::as_proto(),
        crate::classes::org::kwis::msp::lcdui::JletWrapper::as_proto(),
        crate::classes::org::kwis::msp::lwc::LwcCard::as_proto(),
        crate::classes::org::kwis::msp::lwc::Component::as_proto(),
        crate::classes::org::kwis::msp::lwc::ProgressComponent::as_proto(),
        crate::classes::org::kwis::msp::lwc::ChangeListener::as_proto(),
        crate::classes::org::kwis::msp::lwc::LabelComponent::as_proto(),
        crate::classes::org::kwis::msp::lwc::Command::as_proto(),
        crate::classes::org::kwis::msp::lwc::CommandListener::as_proto(),
        crate::classes::org::kwis::msp::lwc::CommandBarComponent::as_proto(),
        crate::classes::org::kwis::msp::lwc::ButtonComponent::as_proto(),
        crate::classes::org::kwis::msp::lwc::ContainerComponent::as_proto(),
        crate::classes::org::kwis::msp::lwc::EventListener::as_proto(),
        crate::classes::org::kwis::msp::lwc::ShellComponent::as_proto(),
        crate::classes::org::kwis::msp::lwc::DialogComponent::as_proto(),
        crate::classes::org::kwis::msp::lwc::FormComponent::as_proto(),
        crate::classes::org::kwis::msp::lwc::AnnunciatorComponent::as_proto(),
        crate::classes::org::kwis::msp::lwc::TextComponent::as_proto(),
        crate::classes::org::kwis::msp::lwc::TextBoxComponent::as_proto(),
        crate::classes::org::kwis::msp::lwc::TextFieldComponent::as_proto(),
        crate::classes::org::kwis::msp::media::BaseClip::as_proto(),
        crate::classes::org::kwis::msp::media::Clip::as_proto(),
        crate::classes::org::kwis::msp::media::Player::as_proto(),
        crate::classes::org::kwis::msp::media::PlayListener::as_proto(),
        crate::classes::org::kwis::msp::media::Vibrator::as_proto(),
        crate::classes::org::kwis::msp::media::Volume::as_proto(),
        crate::classes::net::wie::CardCanvas::as_proto(),
        crate::classes::net::wie::WIPIFileOutputStream::as_proto(),
        crate::classes::net::wie::WIPIMIDlet::as_proto(),
    ]
}

#[cfg(test)]
mod ktf_audit_tests {
    use alloc::boxed::Box;
    use jvm::ClassInstanceRef;
    use jvm::runtime::JavaLangString;
    use rustjava_runtime::classes::java::lang::String;
    use test_utils::run_jvm_test;
    use wie_util::Result;

    #[test]
    fn documented_optional_classes_resolve_with_correct_contracts() -> Result<()> {
        run_jvm_test(Box::new([wie_midp::get_protos().into(), super::get_protos().into()]), |jvm| async move {
            let count: i32 = jvm.invoke_static("org/kwis/msp/handset/LED", "getCount", "()I", ()).await?;
            assert_eq!(count, 0);
            for mask in [0, 3, -1] {
                let _: () = jvm.invoke_static("org/kwis/msp/handset/LED", "set", "(I)V", (mask,)).await?;
                let bits: i32 = jvm.invoke_static("org/kwis/msp/handset/LED", "get", "()I", ()).await?;
                assert_eq!(bits, 0);
            }
            for exception_name in ["org/kwis/msp/media/MediaUnsupportedException", "java/io/UnavailableException"] {
                let empty = jvm.new_class(exception_name, "()V", ()).await?;
                assert!(jvm.is_instance(&*empty, "java/lang/RuntimeException"));
                let message: ClassInstanceRef<String> = jvm
                    .invoke_virtual(&empty, "java/lang/Throwable", "getMessage", "()Ljava/lang/String;", ())
                    .await?;
                assert!(message.is_null());
                let text = JavaLangString::from_rust_string(&jvm, "unsupported position").await?;
                let exception = jvm.new_class(exception_name, "(Ljava/lang/String;)V", (text,)).await?;
                let message: ClassInstanceRef<String> = jvm
                    .invoke_virtual(&exception, "java/lang/Throwable", "getMessage", "()Ljava/lang/String;", ())
                    .await?;
                assert_eq!(JavaLangString::to_rust_string(&jvm, &message).await?, "unsupported position");
            }
            let listener = jvm.resolve_class("org/kwis/msp/lwc/ActionListener").await?;
            assert!(listener.definition.access_flags().contains(jvm_types::ClassAccessFlags::INTERFACE));
            assert!(
                listener
                    .definition
                    .method("action", "(Lorg/kwis/msp/lwc/Component;Ljava/lang/Object;)V", false)
                    .is_some()
            );
            let input = jvm.resolve_class("org/kwis/msp/lcdui/InputMethodListener").await?;
            assert!(input.definition.access_flags().contains(jvm_types::ClassAccessFlags::INTERFACE));
            assert!(input.definition.method("notifyTextChanged", "([CII)V", false).is_some());
            let grab = jvm.resolve_class("org/kwis/msp/lwc/GrabKeyListener").await?;
            assert!(grab.definition.access_flags().contains(jvm_types::ClassAccessFlags::INTERFACE));
            assert!(grab.definition.method("grabKeyNotify", "(IILjava/lang/Object;)Z", false).is_some());
            Ok(())
        })
    }
}
