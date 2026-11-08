use alloc::vec;

use jvm::{Array, ClassInstanceRef, Jvm, Result as JvmResult, runtime::JavaLangString};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};

use wie_jvm_support::{WieJavaClassProto, WieJvmContext};
use wie_midp::classes::javax::microedition::media::Player;

// not in reference, but called by some apps..
// class org.kwis.msp.media.BaseClip
pub struct BaseClip;

impl BaseClip {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "org/kwis/msp/media/BaseClip",
            parent_class: Some("java/lang/Object"),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("<init>", "()V", Self::init, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("putData", "([BII)I", Self::put_data, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setBuffer", "([BI)Z", Self::set_buffer, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("clearData", "()V", Self::clear_data, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("availableDataSize", "()I", Self::available_data_size, MethodAccessFlags::PUBLIC),
            ],
            fields: vec![
                JavaFieldProto::new("buffer", "[B", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("dataLength", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("player", "Ljavax/microedition/media/Player;", FieldAccessFlags::PRIVATE),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }

    async fn init(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<()> {
        tracing::debug!("org.kwis.msp.media.BaseClip::<init>({this:?})");

        let _: () = jvm.invoke_special(&this, "java/lang/Object", "<init>", "()V", ()).await?;

        Ok(())
    }

    async fn available_data_size(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::warn!("stub org.kwis.msp.media.BaseClip::availableDataSize({this:?})");

        jvm.get_field(&this, "dataLength", "I").await
    }

    async fn put_data(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        buffer: ClassInstanceRef<Array<i8>>,
        offset: i32,
        length: i32,
    ) -> JvmResult<i32> {
        tracing::debug!("org.kwis.msp.media.Clip::putData({this:?}, {buffer:?}, {offset}, {length})");

        let capacity_buffer: ClassInstanceRef<Array<i8>> = jvm.get_field(&this, "buffer", "[B").await?;
        let source_length = jvm.array_length(&buffer).await?;
        if offset < 0 || length < 0 || offset as usize > source_length || length as usize > source_length - offset as usize {
            return Err(jvm.exception("java/lang/IndexOutOfBoundsException", "Invalid clip data range").await);
        }
        if capacity_buffer.is_null() {
            return Ok(0);
        }
        let used: i32 = jvm.get_field(&this, "dataLength", "I").await?;
        let capacity = jvm.array_length(&capacity_buffer).await?;
        let count = (length as usize).min(capacity - used as usize);
        if count == 0 {
            return Ok(0);
        }
        let bytes: alloc::vec::Vec<i8> = jvm.load_array(&buffer, offset as usize, count).await?;
        let mut destination = capacity_buffer;
        jvm.store_array(&mut destination, used as usize, bytes).await?;
        Self::close_player(jvm, &mut this).await?;
        jvm.put_field(&mut this, "dataLength", "I", used + count as i32).await?;
        Ok(count as i32)
    }

    pub async fn prepare_player<T>(jvm: &Jvm, mut this: ClassInstanceRef<T>) -> JvmResult<ClassInstanceRef<Player>> {
        let player: ClassInstanceRef<Player> = jvm.get_field(&this, "player", "Ljavax/microedition/media/Player;").await?;
        if !player.is_null() {
            return Ok(player);
        }
        let length: i32 = jvm.get_field(&this, "dataLength", "I").await?;
        if length == 0 {
            return Ok(None.into());
        }
        let buffer: ClassInstanceRef<Array<i8>> = jvm.get_field(&this, "buffer", "[B").await?;
        let stream = jvm.new_class("java/io/ByteArrayInputStream", "([BII)V", (buffer, 0, length)).await?;
        let kind = JavaLangString::from_rust_string(jvm, "application/vnd.smaf").await?;
        let player: ClassInstanceRef<Player> = jvm
            .invoke_static(
                "javax/microedition/media/Manager",
                "createPlayer",
                "(Ljava/io/InputStream;Ljava/lang/String;)Ljavax/microedition/media/Player;",
                (stream, kind),
            )
            .await?;
        jvm.put_field(&mut this, "player", "Ljavax/microedition/media/Player;", player.clone())
            .await?;
        Ok(player)
    }

    async fn close_player(jvm: &Jvm, this: &mut ClassInstanceRef<Self>) -> JvmResult<()> {
        let player: ClassInstanceRef<Player> = jvm.get_field(this, "player", "Ljavax/microedition/media/Player;").await?;
        if !player.is_null() {
            let _: () = jvm.invoke_virtual(&player, "javax/microedition/media/Player", "close", "()V", ()).await?;
            jvm.put_field(this, "player", "Ljavax/microedition/media/Player;", None).await?;
        }
        Ok(())
    }

    async fn clear_data(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>) -> JvmResult<()> {
        tracing::debug!("org.kwis.msp.media.BaseClip::clearData({this:?})");

        Self::close_player(jvm, &mut this).await?;
        jvm.put_field(&mut this, "dataLength", "I", 0).await?;
        Ok(())
    }

    async fn set_buffer(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        buffer: ClassInstanceRef<Array<i8>>,
        size: i32,
    ) -> JvmResult<bool> {
        tracing::debug!("org.kwis.msp.media.BaseClip::setBuffer({this:?}, {buffer:?}, {size})");

        let existing: ClassInstanceRef<Array<i8>> = jvm.get_field(&this, "buffer", "[B").await?;
        if !existing.is_null() {
            return Ok(false);
        }
        let length = jvm.array_length(&buffer).await?;
        if size < 0 || size as usize > length {
            return Err(jvm.exception("java/lang/IndexOutOfBoundsException", "Invalid clip data size").await);
        }
        let mut this = this;
        jvm.put_field(&mut this, "buffer", "[B", buffer).await?;
        jvm.put_field(&mut this, "dataLength", "I", size).await?;
        Ok(true)
    }
}
