use super::TextComponent;
use alloc::vec;
use jvm::{ClassInstanceRef, Jvm, Result, runtime::JavaLangString};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

const NAME: &str = "net/wie/LwcTextEditor";
const TEXT: &str = "org/kwis/msp/lwc/TextComponent";
const CARD: &str = "org/kwis/msp/lcdui/Card";
const DISPLAY: &str = "org/kwis/msp/lcdui/Display";
const GRAPHICS: &str = "org/kwis/msp/lcdui/Graphics";

/// Full-screen editing stays in guest fields and the existing Card stack, so it
/// uses the same key delivery, recording, save/load, and event semantics as games.
pub struct TextEditor;
impl TextEditor {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: NAME,
            parent_class: Some(CARD),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("<init>", "(Lorg/kwis/msp/lwc/TextComponent;)V", Self::init, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("paint", "(Lorg/kwis/msp/lcdui/Graphics;)V", Self::paint, MethodAccessFlags::PROTECTED),
                JavaMethodProto::new("keyNotify", "(II)Z", Self::key, MethodAccessFlags::PROTECTED),
            ],
            fields: vec![
                JavaFieldProto::new("owner", "Lorg/kwis/msp/lwc/TextComponent;", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("selectDown", "Z", FieldAccessFlags::PRIVATE),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }
    async fn init(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, owner: ClassInstanceRef<TextComponent>) -> Result<()> {
        let _: () = jvm.invoke_special(&this, CARD, "<init>", "()V", ()).await?;
        jvm.put_field(&mut this, "owner", "Lorg/kwis/msp/lwc/TextComponent;", owner).await
    }
    pub(super) async fn open(jvm: &Jvm, mut owner: ClassInstanceRef<TextComponent>) -> Result<()> {
        let existing: ClassInstanceRef<()> = jvm.get_field(&owner, "wieEditor", "Lnet/wie/LwcTextEditor;").await?;
        if !existing.is_null() {
            return Ok(());
        }
        let card = jvm.new_class(NAME, "(Lorg/kwis/msp/lwc/TextComponent;)V", (owner.clone(),)).await?;
        let display: ClassInstanceRef<()> = jvm
            .invoke_virtual(&card, CARD, "getDisplay", "()Lorg/kwis/msp/lcdui/Display;", ())
            .await?;
        jvm.put_field(&mut owner, "wieLastKey", "I", 0).await?;
        jvm.put_field(&mut owner, "wieHangulKeys", "[C", ClassInstanceRef::<()>::new(None))
            .await?;
        jvm.put_field(&mut owner, "wieEditor", "Lnet/wie/LwcTextEditor;", card.clone()).await?;
        jvm.invoke_virtual(&display, DISPLAY, "pushCard", "(Lorg/kwis/msp/lcdui/Card;)V", (card,))
            .await
    }
    async fn key(jvm: &Jvm, ctx: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, kind: i32, code: i32) -> Result<bool> {
        let mut owner: ClassInstanceRef<TextComponent> = jvm.get_field(&this, "owner", "Lorg/kwis/msp/lwc/TextComponent;").await?;
        if code == -5 {
            if kind == 1 {
                jvm.put_field(&mut this, "selectDown", "Z", true).await?;
            }
            if kind == 2 && jvm.get_field::<bool>(&this, "selectDown", "Z").await? {
                jvm.put_field(&mut this, "selectDown", "Z", false).await?;
                let display: ClassInstanceRef<()> = jvm
                    .invoke_virtual(&this, CARD, "getDisplay", "()Lorg/kwis/msp/lcdui/Display;", ())
                    .await?;
                let _: bool = jvm
                    .invoke_virtual(&display, DISPLAY, "removeCard", "(Lorg/kwis/msp/lcdui/Card;)Z", (this.clone(),))
                    .await?;
                jvm.put_field(&mut owner, "wieEditor", "Lnet/wie/LwcTextEditor;", ClassInstanceRef::<()>::new(None))
                    .await?;
                jvm.put_field(&mut owner, "wieLastKey", "I", 0).await?;
                jvm.put_field(&mut owner, "wieHangulKeys", "[C", ClassInstanceRef::<()>::new(None))
                    .await?;
                let _: () = jvm.invoke_virtual(&owner, TEXT, "repaint", "()V", ()).await?;
            }
        } else {
            TextComponent::edit_key(jvm, ctx, owner, kind, code).await?;
            let _: () = jvm.invoke_virtual(&this, CARD, "repaint", "()V", ()).await?;
        }
        // Card uses false to consume input. Do not leak editor keys to the game.
        Ok(false)
    }
    async fn paint(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>, g: ClassInstanceRef<()>) -> Result<()> {
        let owner: ClassInstanceRef<TextComponent> = jvm.get_field(&this, "owner", "Lorg/kwis/msp/lwc/TextComponent;").await?;
        let w: i32 = jvm.invoke_virtual(&this, CARD, "getWidth", "()I", ()).await?;
        let h: i32 = jvm.invoke_virtual(&this, CARD, "getHeight", "()I", ()).await?;
        TextComponent::draw(jvm, ctx, owner.clone(), g.clone(), w, h, true).await?;
        let numeric: bool = jvm.get_field(&owner, "wieNumeric", "Z").await?;
        let constraint: i32 = jvm.get_field(&owner, "constraint", "I").await?;
        let label = if TextComponent::korean(jvm, &owner).await? {
            "한글  #:확정 *:공백 OK:완료"
        } else if numeric || matches!(constraint, 1 | 2 | 5) {
            "123   OK: Done"
        } else {
            "abc   OK: Done"
        };
        let text = JavaLangString::from_rust_string(jvm, label).await?;
        let _: () = jvm.invoke_virtual(&g, GRAPHICS, "setColor", "(I)V", (0xeeeeee,)).await?;
        let _: () = jvm
            .invoke_virtual(&g, GRAPHICS, "fillRect", "(IIII)V", (0, (h - 18).max(0), w, 18))
            .await?;
        let _: () = jvm.invoke_virtual(&g, GRAPHICS, "setColor", "(I)V", (0,)).await?;
        jvm.invoke_virtual(&g, GRAPHICS, "drawString", "(Ljava/lang/String;III)V", (text, 3, (h - 16).max(0), 0))
            .await
    }
}
