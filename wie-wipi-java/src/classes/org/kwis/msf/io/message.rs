use alloc::vec;
use jvm::{Array, ClassInstanceRef, Jvm, Result as JvmResult};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use rustjava_runtime::classes::java::{lang::String, util::Date};
use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

/// WIPI message buffer; all state is stored in Java instance fields.
pub struct Message;
impl Message {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "org/kwis/msf/io/Message",
            parent_class: Some("java/lang/Object"),
            interfaces: vec![],
            access_flags: ClassAccessFlags::PUBLIC,
            fields: vec![
                JavaFieldProto::new("data", "[B", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("address", "Ljava/lang/String;", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("addressInt", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("date", "Ljava/util/Date;", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("index", "B", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("classification", "B", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("teleServiceID", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("offset", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("length", "I", FieldAccessFlags::PRIVATE),
            ],
            methods: vec![
                JavaMethodProto::new("<init>", "([B)V", Self::init_buffer, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("<init>", "(Ljava/lang/String;[B)V", Self::init_address, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("<init>", "(Ljava/lang/String;[BII)V", Self::init_slice, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getData", "()[B", Self::get_data, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getAddress", "()Ljava/lang/String;", Self::get_address, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setAddress", "(Ljava/lang/String;)V", Self::set_address, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getAddressInt", "()I", Self::get_addressint, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setAddressInt", "(I)V", Self::set_addressint, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getDate", "()Ljava/util/Date;", Self::get_date, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setDate", "(Ljava/util/Date;)V", Self::set_date, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getIndex", "()B", Self::get_index, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setIndex", "(B)V", Self::set_index, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getClassification", "()B", Self::get_classification, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setClassification", "(B)V", Self::set_classification, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getTeleServiceID", "()I", Self::get_teleserviceid, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setTeleServiceID", "(I)V", Self::set_teleserviceid, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getOffset", "()I", Self::get_offset, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setOffset", "(I)I", Self::set_offset, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getLength", "()I", Self::get_length, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setLength", "(I)I", Self::set_length, MethodAccessFlags::PUBLIC),
            ],
        }
    }
    async fn init_buffer(jvm: &Jvm, context: &mut WieJvmContext, this: ClassInstanceRef<Self>, data: ClassInstanceRef<Array<i8>>) -> JvmResult<()> {
        Self::init_address(jvm, context, this, ClassInstanceRef::new(None), data).await
    }
    async fn init_address(
        jvm: &Jvm,
        context: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        address: ClassInstanceRef<String>,
        data: ClassInstanceRef<Array<i8>>,
    ) -> JvmResult<()> {
        let length = jvm.array_length(&data).await? as i32;
        Self::init_slice(jvm, context, this, address, data, 0, length).await
    }
    async fn init_slice(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        address: ClassInstanceRef<String>,
        data: ClassInstanceRef<Array<i8>>,
        offset: i32,
        length: i32,
    ) -> JvmResult<()> {
        let size = jvm.array_length(&data).await? as i32;
        if offset < 0 || length < 0 || i64::from(offset) + i64::from(length) > i64::from(size) {
            return Err(jvm.exception("java/lang/IndexOutOfBoundsException", "Invalid message range").await);
        }
        let _: () = jvm.invoke_special(&this, "java/lang/Object", "<init>", "()V", ()).await?;
        jvm.put_field(&mut this, "data", "[B", data).await?;
        jvm.put_field(&mut this, "address", "Ljava/lang/String;", address).await?;
        jvm.put_field(&mut this, "addressInt", "I", -1i32).await?;
        jvm.put_field(&mut this, "offset", "I", offset).await?;
        jvm.put_field(&mut this, "length", "I", length).await
    }
    async fn get_data(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<ClassInstanceRef<Array<i8>>> {
        jvm.get_field(&this, "data", "[B").await
    }
    async fn get_address(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<ClassInstanceRef<String>> {
        jvm.get_field(&this, "address", "Ljava/lang/String;").await
    }
    async fn set_address(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, value: ClassInstanceRef<String>) -> JvmResult<()> {
        jvm.put_field(&mut this, "addressInt", "I", -1i32).await?;
        jvm.put_field(&mut this, "address", "Ljava/lang/String;", value).await
    }
    async fn get_addressint(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        jvm.get_field(&this, "addressInt", "I").await
    }
    async fn set_addressint(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, value: i32) -> JvmResult<()> {
        jvm.put_field(&mut this, "addressInt", "I", value).await
    }
    async fn get_date(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<ClassInstanceRef<Date>> {
        jvm.get_field(&this, "date", "Ljava/util/Date;").await
    }
    async fn set_date(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, value: ClassInstanceRef<Date>) -> JvmResult<()> {
        jvm.put_field(&mut this, "date", "Ljava/util/Date;", value).await
    }
    async fn get_index(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i8> {
        jvm.get_field(&this, "index", "B").await
    }
    async fn set_index(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, value: i8) -> JvmResult<()> {
        jvm.put_field(&mut this, "index", "B", value).await
    }
    async fn get_classification(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i8> {
        jvm.get_field(&this, "classification", "B").await
    }
    async fn set_classification(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, value: i8) -> JvmResult<()> {
        jvm.put_field(&mut this, "classification", "B", value).await
    }
    async fn get_teleserviceid(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        jvm.get_field(&this, "teleServiceID", "I").await
    }
    async fn set_teleserviceid(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, value: i32) -> JvmResult<()> {
        jvm.put_field(&mut this, "teleServiceID", "I", value).await
    }
    async fn get_offset(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        jvm.get_field(&this, "offset", "I").await
    }
    async fn set_offset(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, value: i32) -> JvmResult<i32> {
        let data: ClassInstanceRef<Array<i8>> = jvm.get_field(&this, "data", "[B").await?;
        let size = jvm.array_length(&data).await? as i32;
        let other: i32 = jvm.get_field(&this, "length", "I").await?;
        if value < 0 || i64::from(value) + i64::from(other) > i64::from(size) || value >= size {
            return Ok(-1);
        }
        jvm.put_field(&mut this, "offset", "I", value).await?;
        Ok(value)
    }
    async fn get_length(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        jvm.get_field(&this, "length", "I").await
    }
    async fn set_length(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, value: i32) -> JvmResult<i32> {
        let data: ClassInstanceRef<Array<i8>> = jvm.get_field(&this, "data", "[B").await?;
        let size = jvm.array_length(&data).await? as i32;
        let other: i32 = jvm.get_field(&this, "offset", "I").await?;
        if value < 0 || i64::from(value) + i64::from(other) > i64::from(size) {
            return Ok(-1);
        }
        jvm.put_field(&mut this, "length", "I", value).await?;
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::Message;
    use alloc::boxed::Box;
    use jvm::{Array, ClassInstanceRef, runtime::JavaLangString};
    use test_utils::run_jvm_test;
    use wie_util::Result;

    #[test]
    fn message_buffer_ranges_and_address_metadata() -> Result<()> {
        run_jvm_test(Box::new([crate::get_protos().into()]), |jvm| async move {
            let mut data = jvm.instantiate_array("B", 8).await?;
            jvm.store_array(&mut data, 0, [42i8]).await?;
            let address = JavaLangString::from_rust_string(&jvm, "destination").await?;
            let message: ClassInstanceRef<Message> = jvm
                .new_class(
                    "org/kwis/msf/io/Message",
                    "(Ljava/lang/String;[BII)V",
                    (address.clone(), data.clone(), 2, 4),
                )
                .await?
                .into();
            let returned: ClassInstanceRef<Array<i8>> = jvm.invoke_virtual(&message, "org/kwis/msf/io/Message", "getData", "()[B", ()).await?;
            // The buffer is shared, not copied.
            jvm.store_array(&mut data, 0, [99i8]).await?;
            assert_eq!(jvm.load_array::<i8>(&returned, 0, 1).await?, [99i8]);
            for (value, expected) in [(7, -1), (-1, -1), (i32::MAX, -1), (6, 6)] {
                let result: i32 = jvm
                    .invoke_virtual(&message, "org/kwis/msf/io/Message", "setLength", "(I)I", (value,))
                    .await?;
                assert_eq!(result, expected);
            }
            let result: i32 = jvm.invoke_virtual(&message, "org/kwis/msf/io/Message", "setOffset", "(I)I", (3,)).await?;
            assert_eq!(result, -1);
            let offset: i32 = jvm.invoke_virtual(&message, "org/kwis/msf/io/Message", "getOffset", "()I", ()).await?;
            assert_eq!(offset, 2);
            let _: () = jvm
                .invoke_virtual(&message, "org/kwis/msf/io/Message", "setAddressInt", "(I)V", (123,))
                .await?;
            let _: () = jvm
                .invoke_virtual(&message, "org/kwis/msf/io/Message", "setAddress", "(Ljava/lang/String;)V", (address,))
                .await?;
            let integer: i32 = jvm
                .invoke_virtual(&message, "org/kwis/msf/io/Message", "getAddressInt", "()I", ())
                .await?;
            assert_eq!(integer, -1);
            let _: () = jvm
                .invoke_virtual(&message, "org/kwis/msf/io/Message", "setIndex", "(B)V", (-7i8,))
                .await?;
            let index: i8 = jvm.invoke_virtual(&message, "org/kwis/msf/io/Message", "getIndex", "()B", ()).await?;
            assert_eq!(index, -7);
            let received: ClassInstanceRef<Message> = jvm.new_class("org/kwis/msf/io/Message", "([B)V", (data,)).await?.into();
            let length: i32 = jvm.invoke_virtual(&received, "org/kwis/msf/io/Message", "getLength", "()I", ()).await?;
            assert_eq!(length, 8);
            Ok(())
        })
    }
}
