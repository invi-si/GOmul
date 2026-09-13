use alloc::vec;

use jvm::{ClassInstanceRef, Jvm, Result, runtime::JavaLangString};
use jvm_class_proto::JavaMethodProto;
use jvm_types::{ClassAccessFlags, MethodAccessFlags};
use rustjava_runtime::classes::java::{io::InputStream, lang::String};

use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

use crate::classes::javax::microedition::media::Player;

// class javax.microedition.media.Manager
pub struct Manager;

impl Manager {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "javax/microedition/media/Manager",
            parent_class: Some("java/lang/Object"),
            interfaces: vec![],
            methods: vec![JavaMethodProto::new(
                "createPlayer",
                "(Ljava/io/InputStream;Ljava/lang/String;)Ljavax/microedition/media/Player;",
                Self::create_player,
                MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC,
            )],
            fields: vec![],
            access_flags: ClassAccessFlags::PUBLIC | ClassAccessFlags::FINAL,
        }
    }

    async fn create_player(
        jvm: &Jvm,
        _context: &mut WieJvmContext,
        stream: ClassInstanceRef<InputStream>,
        r#type: ClassInstanceRef<String>,
    ) -> Result<ClassInstanceRef<Player>> {
        tracing::debug!("javax.microedition.media.Manager::createPlayer({stream:?}, {type:?})");

        if stream.is_null() {
            return Err(jvm.exception("java/lang/IllegalArgumentException", "Media stream is null").await);
        }
        // A null content type requests detection. The available SMAF loader
        // validates its input and reports MediaException if it cannot decode it.
        let is_smaf = r#type.is_null() || JavaLangString::to_rust_string(jvm, &r#type).await? == "application/vnd.smaf";
        if is_smaf {
            Ok(jvm.new_class("net/wie/SmafPlayer", "(Ljava/io/InputStream;)V", (stream,)).await?.into())
        } else {
            Err(jvm.exception("javax/microedition/media/MediaException", "Unsupported media type").await)
        }
    }
}
