use crate::classes::net::wie::WIPIKeyCode;
use alloc::{vec, vec::Vec};
#[cfg(test)]
use jvm::runtime::JavaLangString;
use jvm::{Array, ClassInstanceRef, Jvm, Result as JvmResult};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use rustjava_runtime::classes::java::lang::String;
use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

pub struct TextComponent;
const NAME: &str = "org/kwis/msp/lwc/TextComponent";
const COMPONENT: &str = "org/kwis/msp/lwc/Component";
const FONT: &str = "org/kwis/msp/lcdui/Font";
const GRAPHICS: &str = "org/kwis/msp/lcdui/Graphics";
type Chars = ClassInstanceRef<Array<u16>>;
impl TextComponent {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: NAME,
            parent_class: Some(COMPONENT),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("<init>", "()V", Self::init, MethodAccessFlags::PROTECTED),
                JavaMethodProto::new("setString", "(Ljava/lang/String;)V", Self::set_string, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getString", "()Ljava/lang/String;", Self::get_string, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setMaxLength", "(I)V", Self::set_max_length, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getMaxLength", "()I", Self::max_length, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getConstraint", "()I", Self::constraint, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getFont", "()Lorg/kwis/msp/lcdui/Font;", Self::font, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setFont", "(Lorg/kwis/msp/lcdui/Font;)V", Self::set_font, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("insert", "([CIII)V", Self::insert, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("delete", "(II)V", Self::delete, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("keyNotify", "(II)Z", Self::key, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("focusNotify", "(Z)V", Self::focus, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("paintContent", "(Lorg/kwis/msp/lcdui/Graphics;)V", Self::paint, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("calcPreferredSize", "(I)V", Self::preferred, MethodAccessFlags::PROTECTED),
            ],
            fields: vec![
                JavaFieldProto::new("m_td", "[C", FieldAccessFlags::PROTECTED),
                JavaFieldProto::new("charCount", "I", FieldAccessFlags::PROTECTED),
                JavaFieldProto::new("constraint", "I", FieldAccessFlags::PROTECTED),
                JavaFieldProto::new("maxLength", "I", FieldAccessFlags::PROTECTED),
                JavaFieldProto::new("m_cPos", "I", FieldAccessFlags::PROTECTED),
                JavaFieldProto::new("f", "Lorg/kwis/msp/lcdui/Font;", FieldAccessFlags::PROTECTED),
                JavaFieldProto::new("imHandler", "Lorg/kwis/msp/lcdui/InputMethodHandler;", FieldAccessFlags::PROTECTED),
                JavaFieldProto::new("iMode", "I", FieldAccessFlags::PROTECTED),
                JavaFieldProto::new("wieMultiline", "Z", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("wieEditor", "Lnet/wie/LwcTextEditor;", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("wieSelectDown", "Z", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("wieNumeric", "Z", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("wieInputModeOverride", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("wieUppercase", "Z", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("wieLastKey", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("wieLastTime", "J", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("wieTap", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("wieHangulKeys", "[C", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("wieHangulStart", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("wieHangulEpoch", "I", FieldAccessFlags::PRIVATE),
            ],
            access_flags: ClassAccessFlags::PUBLIC | ClassAccessFlags::ABSTRACT,
        }
    }
    async fn init(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>) -> JvmResult<()> {
        let _: () = jvm.invoke_special(&this, COMPONENT, "<init>", "()V", ()).await?;
        let handler = jvm.new_class("org/kwis/msp/lcdui/InputMethodHandler", "(I)V", (0,)).await?;
        jvm.put_field(&mut this, "imHandler", "Lorg/kwis/msp/lcdui/InputMethodHandler;", handler)
            .await?;
        let font: ClassInstanceRef<()> = jvm.invoke_static(FONT, "getDefaultFont", "()Lorg/kwis/msp/lcdui/Font;", ()).await?;
        jvm.put_field(&mut this, "f", "Lorg/kwis/msp/lcdui/Font;", font).await?;
        jvm.put_field(&mut this, "maxLength", "I", -1).await?;
        jvm.put_field(&mut this, "wieInput", "Z", true).await?;
        let data = jvm.instantiate_array("C", 0).await?;
        jvm.put_field(&mut this, "m_td", "[C", data).await
    }
    async fn data(jvm: &Jvm, this: &ClassInstanceRef<Self>) -> JvmResult<Vec<u16>> {
        let a: Chars = jvm.get_field(this, "m_td", "[C").await?;
        let count: i32 = jvm.get_field(this, "charCount", "I").await?;
        jvm.load_array(&a, 0, count as usize).await
    }
    fn allowed(constraint: i32, data: &[u16]) -> bool {
        data.iter().all(|&c| match constraint {
            1 => (48..=57).contains(&c) || c == 32 || c == 45,
            2 | 5 => (48..=57).contains(&c),
            3 | 4 => (32..=126).contains(&c),
            _ => true,
        })
    }
    async fn replace(jvm: &Jvm, mut this: ClassInstanceRef<Self>, data: Vec<u16>, cursor: usize) -> JvmResult<()> {
        jvm.put_field(&mut this, "wieHangulKeys", "[C", ClassInstanceRef::<()>::new(None)).await?;
        let mut a = jvm.instantiate_array("C", data.len()).await?;
        let len = data.len();
        jvm.store_array(&mut a, 0, data).await?;
        jvm.put_field(&mut this, "m_td", "[C", a).await?;
        jvm.put_field(&mut this, "charCount", "I", len as i32).await?;
        jvm.put_field(&mut this, "m_cPos", "I", cursor.min(len) as i32).await?;
        let _: () = jvm.invoke_virtual(&this, COMPONENT, "invalidate", "()V", ()).await?;
        jvm.invoke_virtual(&this, COMPONENT, "repaint", "()V", ()).await
    }
    async fn set_string(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, text: ClassInstanceRef<String>) -> JvmResult<()> {
        let mut data = if text.is_null() {
            vec![]
        } else {
            let a: Chars = jvm.invoke_virtual(&text, "java/lang/String", "toCharArray", "()[C", ()).await?;
            jvm.load_array(&a, 0, jvm.array_length(&a).await?).await?
        };
        let constraint: i32 = jvm.get_field(&this, "constraint", "I").await?;
        if !Self::allowed(constraint, &data) {
            return Err(jvm.exception("java/lang/IllegalArgumentException", "Text violates constraint").await);
        }
        let max: i32 = jvm.get_field(&this, "maxLength", "I").await?;
        if max >= 0 {
            data.truncate(max as usize);
        }
        jvm.put_field(&mut this, "wieLastKey", "I", 0).await?;
        let cursor = data.len();
        Self::replace(jvm, this, data, cursor).await
    }
    async fn get_string(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<ClassInstanceRef<String>> {
        let a: Chars = jvm.get_field(&this, "m_td", "[C").await?;
        let count: i32 = jvm.get_field(&this, "charCount", "I").await?;
        Ok(jvm.new_class("java/lang/String", "([CII)V", (a, 0, count)).await?.into())
    }
    async fn set_max_length(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, max: i32) -> JvmResult<()> {
        if max == 0 || max < -1 {
            return Err(jvm.exception("java/lang/IllegalArgumentException", "Invalid maximum text length").await);
        }
        let mut data = Self::data(jvm, &this).await?;
        if max >= 0 {
            data.truncate(max as usize);
        }
        let cursor: i32 = jvm.get_field(&this, "m_cPos", "I").await?;
        jvm.put_field(&mut this, "maxLength", "I", max).await?;
        jvm.put_field(&mut this, "wieLastKey", "I", 0).await?;
        Self::replace(jvm, this, data, cursor.max(0) as usize).await
    }
    async fn max_length(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        jvm.get_field(&this, "maxLength", "I").await
    }
    async fn constraint(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        jvm.get_field(&this, "constraint", "I").await
    }
    async fn font(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<ClassInstanceRef<()>> {
        jvm.get_field(&this, "f", "Lorg/kwis/msp/lcdui/Font;").await
    }
    async fn set_font(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, font: ClassInstanceRef<()>) -> JvmResult<()> {
        if font.is_null() {
            return Err(jvm.exception("java/lang/NullPointerException", "font").await);
        }
        jvm.put_field(&mut this, "f", "Lorg/kwis/msp/lcdui/Font;", font).await?;
        let _: () = jvm.invoke_virtual(&this, COMPONENT, "invalidate", "()V", ()).await?;
        jvm.invoke_virtual(&this, COMPONENT, "repaint", "()V", ()).await
    }
    async fn insert(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        a: Chars,
        offset: i32,
        len: i32,
        index: i32,
    ) -> JvmResult<()> {
        if a.is_null() {
            return Err(jvm.exception("java/lang/NullPointerException", "text").await);
        }
        let mut data = Self::data(jvm, &this).await?;
        let size = jvm.array_length(&a).await?;
        if offset < 0 || len < 0 || (offset as usize).saturating_add(len as usize) > size || index < 0 || index as usize > data.len() {
            return Err(jvm.exception("java/lang/IndexOutOfBoundsException", "text range").await);
        }
        let added: Vec<u16> = jvm.load_array(&a, offset as usize, len as usize).await?;
        let max: i32 = jvm.get_field(&this, "maxLength", "I").await?;
        let constraint: i32 = jvm.get_field(&this, "constraint", "I").await?;
        if !Self::allowed(constraint, &added) || (max >= 0 && data.len() + added.len() > max as usize) {
            return Err(jvm.exception("java/lang/IllegalArgumentException", "Text constraint or length").await);
        }
        data.splice(index as usize..index as usize, added);
        jvm.put_field(&mut this, "wieLastKey", "I", 0).await?;
        Self::replace(jvm, this, data, index as usize + len as usize).await
    }
    async fn delete(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, index: i32, len: i32) -> JvmResult<()> {
        let mut data = Self::data(jvm, &this).await?;
        if index < 0 || len < 0 || index as usize > data.len() {
            return Err(jvm.exception("java/lang/IndexOutOfBoundsException", "text range").await);
        }
        if len as usize > data.len() - index as usize {
            return Err(jvm.exception("java/lang/IllegalArgumentException", "text deletion length").await);
        }
        let cursor: i32 = jvm.get_field(&this, "m_cPos", "I").await?;
        data.drain(index as usize..index as usize + len as usize);
        let cursor = if cursor <= index { cursor } else { cursor - len.min(cursor - index) };
        jvm.put_field(&mut this, "wieLastKey", "I", 0).await?;
        Self::replace(jvm, this, data, cursor.max(0) as usize).await
    }
    async fn focus(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, focused: bool) -> JvmResult<()> {
        jvm.put_field(&mut this, "wieHangulKeys", "[C", ClassInstanceRef::<()>::new(None)).await?;
        jvm.put_field(&mut this, "wieLastKey", "I", 0).await?;
        jvm.put_field(&mut this, "wieSelectDown", "Z", false).await?;
        jvm.invoke_special(&this, COMPONENT, "focusNotify", "(Z)V", (focused,)).await
    }
    async fn preferred(jvm: &Jvm, ctx: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, width: i32) -> JvmResult<()> {
        let font = Self::font(jvm, ctx, this.clone()).await?;
        let text = Self::get_string(jvm, ctx, this.clone()).await?;
        let tw: i32 = jvm.invoke_virtual(&font, FONT, "stringWidth", "(Ljava/lang/String;)I", (text,)).await?;
        let fh: i32 = jvm.invoke_virtual(&font, FONT, "getHeight", "()I", ()).await?;
        let multiline: bool = jvm.get_field(&this, "wieMultiline", "Z").await?;
        let w = if multiline && width > 0 { width } else { (tw + 6).max(fh + 6) };
        let rows = if multiline {
            (tw.max(1) + (w - 6).max(1) - 1) / (w - 6).max(1)
        } else {
            1
        };
        jvm.put_field(&mut this, "prefW", "I", w).await?;
        jvm.put_field(&mut this, "prefH", "I", rows.max(1).saturating_mul(fh).saturating_add(4))
            .await
    }
    async fn paint(jvm: &Jvm, ctx: &mut WieJvmContext, this: ClassInstanceRef<Self>, g: ClassInstanceRef<()>) -> JvmResult<()> {
        let _: () = jvm
            .invoke_special(&this, COMPONENT, "paintContent", "(Lorg/kwis/msp/lcdui/Graphics;)V", (g.clone(),))
            .await?;
        let w: i32 = jvm.get_field(&this, "w", "I").await?;
        let h: i32 = jvm.get_field(&this, "h", "I").await?;
        let focus: bool = jvm.get_field(&this, "wieFocused", "Z").await?;
        Self::draw(jvm, ctx, this, g, w, h, focus).await
    }
    pub(super) async fn draw(
        jvm: &Jvm,
        ctx: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        g: ClassInstanceRef<()>,
        w: i32,
        h: i32,
        cursor_visible: bool,
    ) -> JvmResult<()> {
        if w <= 0 || h <= 0 {
            return Ok(());
        }
        let font = Self::font(jvm, ctx, this.clone()).await?;
        let fh: i32 = jvm.invoke_virtual(&font, FONT, "getHeight", "()I", ()).await?;
        let _: () = jvm
            .invoke_virtual(&g, GRAPHICS, "setFont", "(Lorg/kwis/msp/lcdui/Font;)V", (font.clone(),))
            .await?;
        let _: () = jvm.invoke_virtual(&g, GRAPHICS, "setColor", "(I)V", (0xffffff,)).await?;
        let _: () = jvm.invoke_virtual(&g, GRAPHICS, "fillRect", "(IIII)V", (0, 0, w, h)).await?;
        let fg: i32 = jvm.get_field(&this, "fg", "I").await?;
        let _: () = jvm.invoke_virtual(&g, GRAPHICS, "setColor", "(I)V", (fg,)).await?;
        let _: () = jvm.invoke_virtual(&g, GRAPHICS, "drawRect", "(IIII)V", (0, 0, w - 1, h - 1)).await?;
        let mut data = Self::data(jvm, &this).await?;
        let constraint: i32 = jvm.get_field(&this, "constraint", "I").await?;
        if constraint == 2 {
            data.fill(42);
        }
        let mut a = jvm.instantiate_array("C", data.len()).await?;
        jvm.store_array(&mut a, 0, data.clone()).await?;
        let multiline: bool = jvm.get_field(&this, "wieMultiline", "Z").await?;
        let cursor: i32 = jvm.get_field(&this, "m_cPos", "I").await?;
        let mut x = 3;
        let mut y = 2;
        for i in 0..=data.len() {
            let cw: i32 = if i == data.len() {
                0
            } else {
                jvm.invoke_virtual(&font, FONT, "charsWidth", "([CII)I", (a.clone(), i as i32, 1)).await?
            };
            if multiline && x > 3 && x + cw > w - 3 {
                x = 3;
                y += fh;
            }
            if cursor_visible && cursor == i as i32 && y < h - 1 {
                let _: () = jvm
                    .invoke_virtual(&g, GRAPHICS, "drawLine", "(IIII)V", (x, y, x, (y + fh - 1).min(h - 2)))
                    .await?;
            }
            if i < data.len() && y < h - 1 {
                if data[i] == 10 && multiline {
                    x = 3;
                    y += fh;
                    continue;
                }
                let _: () = jvm
                    .invoke_virtual(&g, GRAPHICS, "drawChars", "([CIIIII)V", (a.clone(), i as i32, 1, x, y, 0))
                    .await?;
                x += cw;
            }
        }
        Ok(())
    }
    async fn key(jvm: &Jvm, ctx: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, kind: i32, code: i32) -> JvmResult<bool> {
        if code == WIPIKeyCode::FIRE as i32 {
            if kind == 1 {
                jvm.put_field(&mut this, "wieSelectDown", "Z", true).await?;
            }
            if kind == 2 {
                let down: bool = jvm.get_field(&this, "wieSelectDown", "Z").await?;
                jvm.put_field(&mut this, "wieSelectDown", "Z", false).await?;
                if down {
                    super::text_editor::TextEditor::open(jvm, this).await?;
                }
            }
            return Ok(true);
        }
        Self::edit_key(jvm, ctx, this, kind, code).await
    }
    pub(super) async fn korean(jvm: &Jvm, this: &ClassInstanceRef<Self>) -> JvmResult<bool> {
        let enabled: bool = jvm.get_static_field("javax/microedition/lcdui/Display", "koreanInput", "Z").await?;
        let local: i32 = jvm.get_field(this, "wieInputModeOverride", "I").await?;
        let enabled = if local == 0 { enabled } else { local == 1 };
        let constraint: i32 = jvm.get_field(this, "constraint", "I").await?;
        Ok(enabled && !matches!(constraint, 1..=5))
    }
    async fn hangul_key(jvm: &Jvm, mut this: ClassInstanceRef<Self>, code: i32, kind: i32) -> JvmResult<bool> {
        let data = Self::data(jvm, &this).await?;
        let cursor = jvm.get_field::<i32>(&this, "m_cPos", "I").await?.clamp(0, data.len() as i32) as usize;
        let epoch: i32 = jvm.get_static_field("javax/microedition/lcdui/Display", "inputModeEpoch", "I").await?;
        let keys: Chars = jvm.get_field(&this, "wieHangulKeys", "[C").await?;
        let mut tokens = if keys.is_null() {
            vec![]
        } else {
            jvm.load_array::<u16>(&keys, 0, jvm.array_length(&keys).await?).await?
        };
        let mut start = jvm.get_field::<i32>(&this, "wieHangulStart", "I").await?.max(0) as usize;
        let rendered = super::hangul::compose(&tokens);
        if tokens.is_empty()
            || jvm.get_field::<i32>(&this, "wieHangulEpoch", "I").await? != epoch
            || start + rendered.len() != cursor
            || data.get(start..cursor) != Some(rendered.as_slice())
        {
            tokens.clear();
            start = cursor;
        }
        let now: i64 = jvm.invoke_static("java/lang/System", "currentTimeMillis", "()J", ()).await?;
        let last: i32 = jvm.get_field(&this, "wieLastKey", "I").await?;
        let time: i64 = jvm.get_field(&this, "wieLastTime", "J").await?;
        let digit = (48..=57).contains(&code);
        if code == 35 {
            jvm.put_field(&mut this, "wieLastKey", "I", 0).await?;
            Self::replace(jvm, this, data, cursor).await?;
            return Ok(true);
        }
        if digit {
            super::hangul::push(
                &mut tokens,
                (code - 48) as u16,
                kind == 1 && last == code && now >= time && now - time < 700,
            );
        } else if tokens.is_empty() {
            let mut data = data;
            if cursor > 0 {
                data.remove(cursor - 1);
            }
            jvm.put_field(&mut this, "wieLastKey", "I", 0).await?;
            Self::replace(jvm, this, data, cursor.saturating_sub(1)).await?;
            return Ok(true);
        } else {
            tokens.pop();
        }
        let rendered = super::hangul::compose(&tokens);
        let mut next = data[..start].to_vec();
        next.extend_from_slice(&rendered);
        next.extend_from_slice(&data[cursor..]);
        let max: i32 = jvm.get_field(&this, "maxLength", "I").await?;
        if max >= 0 && next.len() > max as usize {
            return Ok(true);
        }
        let next_cursor = start + rendered.len();
        Self::replace(jvm, this.clone(), next, next_cursor).await?;
        let mut keys = jvm.instantiate_array("C", tokens.len()).await?;
        jvm.store_array(&mut keys, 0, tokens).await?;
        jvm.put_field(&mut this, "wieHangulKeys", "[C", keys).await?;
        jvm.put_field(&mut this, "wieHangulStart", "I", start as i32).await?;
        jvm.put_field(&mut this, "wieHangulEpoch", "I", epoch).await?;
        jvm.put_field(&mut this, "wieLastKey", "I", if digit { code } else { 0 }).await?;
        jvm.put_field(&mut this, "wieLastTime", "J", now).await?;
        Ok(true)
    }
    pub(super) async fn edit_key(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, kind: i32, code: i32) -> JvmResult<bool> {
        let typed = kind == 4 && (32..=65535).contains(&code);
        let digit = (48..=57).contains(&code);
        let direction = code == -3 || code == -4;
        let clear = code == WIPIKeyCode::CLEAR as i32 || code == WIPIKeyCode::RIGHT_SOFT_KEY as i32;
        let mode = code == WIPIKeyCode::LEFT_SOFT_KEY as i32;
        if !typed && !digit && !direction && !clear && !mode && code != 35 && code != 42 {
            return Ok(false);
        }
        if kind != 1 && kind != 3 && !typed {
            return Ok(true);
        }
        let korean = Self::korean(jvm, &this).await?;
        if korean && !typed && (digit || clear || code == 35) {
            return Self::hangul_key(jvm, this, code, kind).await;
        }
        let mut data = Self::data(jvm, &this).await?;
        let mut cursor = jvm.get_field::<i32>(&this, "m_cPos", "I").await?.clamp(0, data.len() as i32) as usize;
        let last: i32 = jvm.get_field(&this, "wieLastKey", "I").await?;
        jvm.put_field(&mut this, "wieLastKey", "I", 0).await?;
        if mode && korean {
            // Completing composition does not send a game soft-key action.
        } else if mode {
            let numeric: bool = jvm.get_field(&this, "wieNumeric", "Z").await?;
            jvm.put_field(&mut this, "wieNumeric", "Z", !numeric).await?;
        } else if direction {
            cursor = if code == -3 {
                cursor.saturating_sub(1)
            } else {
                (cursor + 1).min(data.len())
            };
        } else if clear {
            if cursor > 0 {
                cursor -= 1;
                data.remove(cursor);
            }
        } else {
            let constraint: i32 = jvm.get_field(&this, "constraint", "I").await?;
            let numeric: bool = jvm.get_field(&this, "wieNumeric", "Z").await?;
            let now: i64 = jvm.invoke_static("java/lang/System", "currentTimeMillis", "()J", ()).await?;
            let last_time: i64 = jvm.get_field(&this, "wieLastTime", "J").await?;
            let alphabetic = digit && !typed && !numeric && !matches!(constraint, 1 | 2 | 5);
            let table = [" 0", ".,?!1", "abc2", "def3", "ghi4", "jkl5", "mno6", "pqrs7", "tuv8", "wxyz9"];
            let cycling = alphabetic && kind == 1 && last == code && cursor > 0 && now >= last_time && now - last_time < 700;
            let tap = if cycling {
                jvm.get_field::<i32>(&this, "wieTap", "I").await?.saturating_add(1)
            } else {
                0
            };
            let character = if alphabetic {
                let chars = table[(code - 48) as usize].as_bytes();
                chars[tap as usize % chars.len()] as u16
            } else {
                if korean && code == 42 { 32 } else { code as u16 }
            };
            let max: i32 = jvm.get_field(&this, "maxLength", "I").await?;
            let character = if alphabetic && jvm.get_field::<bool>(&this, "wieUppercase", "Z").await? {
                (character as u8).to_ascii_uppercase() as u16
            } else {
                character
            };
            if Self::allowed(constraint, &[character]) && (cycling || max < 0 || data.len() < max as usize) {
                if cycling {
                    data[cursor - 1] = character;
                } else {
                    data.insert(cursor, character);
                    cursor += 1;
                }
                if alphabetic {
                    jvm.put_field(&mut this, "wieLastKey", "I", code).await?;
                }
                jvm.put_field(&mut this, "wieLastTime", "J", now).await?;
                jvm.put_field(&mut this, "wieTap", "I", tap).await?;
            }
        }
        Self::replace(jvm, this, data, cursor).await?;
        Ok(true)
    }
}
#[cfg(test)]
mod tests {
    use alloc::boxed::Box;
    use jvm::{ClassInstanceRef, runtime::JavaLangString};
    use rustjava_runtime::classes::java::lang::String;
    use test_utils::run_jvm_test;
    use wie_util::Result;

    #[test]
    fn inherited_input_mode_is_zero_initialized_and_instance_local() -> Result<()> {
        run_jvm_test(Box::new([wie_midp::get_protos().into(), crate::get_protos().into()]), |jvm| async move {
            let name = "org/kwis/msp/lwc/TextBoxComponent";
            let text = JavaLangString::from_rust_string(&jvm, "입력").await?;
            let mut a = jvm.new_class(name, "(Ljava/lang/String;II)V", (text, 0, 80)).await?;
            let b = jvm
                .new_class(
                    "org/kwis/msp/lwc/TextFieldComponent",
                    "(Ljava/lang/String;I)V",
                    (ClassInstanceRef::<String>::new(None), 0),
                )
                .await?;
            assert_eq!(jvm.get_field::<i32>(&a, "iMode", "I").await?, 0);
            assert_eq!(jvm.get_field::<i32>(&b, "iMode", "I").await?, 0);
            for mode in [1, 2, -1, i32::MAX] {
                jvm.put_field(&mut a, "iMode", "I", mode).await?;
                assert_eq!(jvm.get_field::<i32>(&a, "iMode", "I").await?, mode);
                assert_eq!(jvm.get_field::<i32>(&b, "iMode", "I").await?, 0);
            }
            assert_eq!(jvm.get_field::<i32>(&a, "constraint", "I").await?, 0);
            assert_eq!(jvm.get_field::<i32>(&a, "boxHeight", "I").await?, 80);
            let value: ClassInstanceRef<String> = jvm.invoke_virtual(&a, name, "getString", "()Ljava/lang/String;", ()).await?;
            assert_eq!(JavaLangString::to_rust_string(&jvm, &value).await?, "입력");
            Ok(())
        })
    }
}

#[cfg(test)]
mod editing_tests {
    use super::*;
    use alloc::boxed::Box;
    use test_utils::run_jvm_test;
    const FIELD: &str = "org/kwis/msp/lwc/TextFieldComponent";
    async fn value(jvm: &Jvm, field: &ClassInstanceRef<()>) -> JvmResult<alloc::string::String> {
        let text: ClassInstanceRef<JavaLangString> = jvm.invoke_virtual(field, NAME, "getString", "()Ljava/lang/String;", ()).await?;
        JavaLangString::to_rust_string(jvm, &text).await
    }
    #[test]
    fn text_limits_utf16_ranges_and_constraints_preserve_state_on_rejection() -> wie_util::Result<()> {
        run_jvm_test(Box::new([wie_midp::get_protos().into(), crate::get_protos().into()]), |jvm| async move {
            let text = JavaLangString::from_rust_string(&jvm, "가나다라").await?;
            let field: ClassInstanceRef<()> = jvm.new_class(FIELD, "(Ljava/lang/String;I)V", (text, 0)).await?.into();
            assert_eq!(value(&jvm, &field).await?, "가나다라");
            let _: () = jvm.invoke_virtual(&field, NAME, "setMaxLength", "(I)V", (2,)).await?;
            assert_eq!(value(&jvm, &field).await?, "가나");
            for n in [0, -2] {
                assert!(jvm.invoke_virtual::<_, ()>(&field, NAME, "setMaxLength", "(I)V", (n,)).await.is_err());
            }
            assert_eq!(jvm.invoke_virtual::<_, i32>(&field, NAME, "getMaxLength", "()I", ()).await?, 2);
            let mut chars = jvm.instantiate_array("C", 4).await?;
            jvm.store_array(&mut chars, 0, vec![65u16, 0xd83d, 0xde00, 66]).await?;
            assert!(
                jvm.invoke_virtual::<_, ()>(&field, FIELD, "insert", "([CIII)V", (chars.clone(), 1, 2, 1))
                    .await
                    .is_err()
            );
            assert_eq!(value(&jvm, &field).await?, "가나");
            let _: () = jvm.invoke_virtual(&field, NAME, "setMaxLength", "(I)V", (-1,)).await?;
            let _: () = jvm.invoke_virtual(&field, FIELD, "insert", "([CIII)V", (chars.clone(), 1, 2, 1)).await?;
            assert_eq!(value(&jvm, &field).await?, "가😀나");
            assert_eq!(jvm.get_field::<i32>(&field, "charCount", "I").await?, 4);
            for (off, len, at) in [(-1, 1, 0), (3, 2, 0), (0, 1, -1), (0, 1, 5)] {
                assert!(
                    jvm.invoke_virtual::<_, ()>(&field, FIELD, "insert", "([CIII)V", (chars.clone(), off, len, at))
                        .await
                        .is_err()
                );
            }
            let _: () = jvm.invoke_virtual(&field, NAME, "delete", "(II)V", (1, 2)).await?;
            assert_eq!(value(&jvm, &field).await?, "가나");
            assert!(jvm.invoke_virtual::<_, ()>(&field, NAME, "delete", "(II)V", (1, 2)).await.is_err());
            assert_eq!(value(&jvm, &field).await?, "가나");
            for constraint in [1, 2, 5] {
                let numeric: ClassInstanceRef<()> = jvm
                    .new_class(FIELD, "(Ljava/lang/String;I)V", (ClassInstanceRef::<()>::new(None), constraint))
                    .await?
                    .into();
                let bad = JavaLangString::from_rust_string(&jvm, "가").await?;
                assert!(
                    jvm.invoke_virtual::<_, ()>(&numeric, NAME, "setString", "(Ljava/lang/String;)V", (bad,))
                        .await
                        .is_err()
                );
                assert_eq!(value(&jvm, &numeric).await?, "");
                for code in [49, 50] {
                    for kind in [1, 2] {
                        let _: bool = jvm.invoke_virtual(&numeric, NAME, "keyNotify", "(II)Z", (kind, code)).await?;
                    }
                }
                assert_eq!(value(&jvm, &numeric).await?, "12");
            }
            assert_eq!(value(&jvm, &field).await?, "가나");
            Ok(())
        })
    }
    #[test]
    fn hangul_field_composition_limits_commit_and_numeric_constraints() -> wie_util::Result<()> {
        run_jvm_test(
            Box::new([wie_midp::get_protos().into(), super::super::shell_component::test_support::protos()]),
            |jvm| async move {
                super::super::shell_component::test_support::init(&jvm).await?;
                let queue: ClassInstanceRef<()> = jvm
                    .invoke_static("net/wie/EventQueue", "getEventQueue", "()Lnet/wie/EventQueue;", ())
                    .await?;
                let mut event = jvm.instantiate_array("I", 4).await?;
                jvm.store_array(&mut event, 0, [42, 1, 0, 0]).await?;
                let _: () = jvm
                    .invoke_virtual(&queue, "net/wie/EventQueue", "dispatchEvent", "([I)V", (event,))
                    .await?;
                assert!(
                    jvm.get_static_field::<bool>("javax/microedition/lcdui/Display", "koreanInput", "Z")
                        .await?
                );
                let field: ClassInstanceRef<()> = jvm
                    .new_class(FIELD, "(Ljava/lang/String;I)V", (ClassInstanceRef::<()>::new(None), 0))
                    .await?
                    .into();
                for code in [56, 56, 49, 50, 53, 52, 51, 53, 53] {
                    for kind in [1, 2] {
                        let _: bool = jvm.invoke_virtual(&field, NAME, "keyNotify", "(II)Z", (kind, code)).await?;
                    }
                }
                assert_eq!(value(&jvm, &field).await?, "한글");
                let _: bool = jvm.invoke_virtual(&field, NAME, "keyNotify", "(II)Z", (1, 35)).await?;
                let _: bool = jvm.invoke_virtual(&field, NAME, "keyNotify", "(II)Z", (1, 42)).await?;
                assert_eq!(value(&jvm, &field).await?, "한글 ");
                let _: bool = jvm.invoke_virtual(&field, NAME, "keyNotify", "(II)Z", (1, -16)).await?;
                assert_eq!(value(&jvm, &field).await?, "한글");
                let limited: ClassInstanceRef<()> = jvm
                    .new_class(FIELD, "(Ljava/lang/String;I)V", (ClassInstanceRef::<()>::new(None), 0))
                    .await?
                    .into();
                let _: () = jvm.invoke_virtual(&limited, NAME, "setMaxLength", "(I)V", (1,)).await?;
                for code in [52, 49, 50, 53] {
                    let _: bool = jvm.invoke_virtual(&limited, NAME, "keyNotify", "(II)Z", (1, code)).await?;
                }
                assert_eq!(value(&jvm, &limited).await?, "간");
                let _: bool = jvm.invoke_virtual(&limited, NAME, "keyNotify", "(II)Z", (1, 49)).await?;
                assert_eq!(value(&jvm, &limited).await?, "간", "splitting to two syllables must respect length limit");
                let _: bool = jvm.invoke_virtual(&limited, NAME, "keyNotify", "(II)Z", (1, -16)).await?;
                assert_eq!(value(&jvm, &limited).await?, "가");
                let _: bool = jvm.invoke_virtual(&limited, NAME, "keyNotify", "(II)Z", (1, -16)).await?;
                assert_eq!(value(&jvm, &limited).await?, "기");
                let numeric: ClassInstanceRef<()> = jvm
                    .new_class(FIELD, "(Ljava/lang/String;I)V", (ClassInstanceRef::<()>::new(None), 2))
                    .await?
                    .into();
                for code in [49, 50] {
                    let _: bool = jvm.invoke_virtual(&numeric, NAME, "keyNotify", "(II)Z", (1, code)).await?;
                }
                assert_eq!(value(&jvm, &numeric).await?, "12");
                // Programmatic assignment commits the old preedit, even if text is equal.
                let string = JavaLangString::from_rust_string(&jvm, "기").await?;
                let _: () = jvm
                    .invoke_virtual(&limited, NAME, "setString", "(Ljava/lang/String;)V", (string,))
                    .await?;
                let _: bool = jvm.invoke_virtual(&limited, NAME, "keyNotify", "(II)Z", (1, 50)).await?;
                assert_eq!(value(&jvm, &limited).await?, "기");
                Ok(())
            },
        )
    }
    #[test]
    fn paired_keys_multitap_focus_and_editor_roundtrip() -> wie_util::Result<()> {
        run_jvm_test(
            Box::new([wie_midp::get_protos().into(), super::super::shell_component::test_support::protos()]),
            |jvm| async move {
                super::super::shell_component::test_support::init(&jvm).await?;
                let field: ClassInstanceRef<()> = jvm
                    .new_class(FIELD, "(Ljava/lang/String;I)V", (ClassInstanceRef::<()>::new(None), 0))
                    .await?
                    .into();
                let _: () = jvm.invoke_virtual(&field, NAME, "setMaxLength", "(I)V", (2,)).await?;
                for kind in [1, 2, 1, 2] {
                    assert!(jvm.invoke_virtual::<_, bool>(&field, NAME, "keyNotify", "(II)Z", (kind, 50)).await?);
                }
                assert_eq!(value(&jvm, &field).await?, "b");
                for code in [-4, 50, 51] {
                    for kind in [1, 2] {
                        let _: bool = jvm.invoke_virtual(&field, NAME, "keyNotify", "(II)Z", (kind, code)).await?;
                    }
                }
                assert_eq!(value(&jvm, &field).await?, "ba");
                let _: bool = jvm.invoke_virtual(&field, NAME, "keyNotify", "(II)Z", (1, -16)).await?;
                assert_eq!(value(&jvm, &field).await?, "b");
                assert!(!jvm.invoke_virtual::<_, bool>(&field, NAME, "keyNotify", "(II)Z", (1, -2)).await?);
                let _: bool = jvm.invoke_virtual(&field, NAME, "keyNotify", "(II)Z", (2, -5)).await?;
                let editor: ClassInstanceRef<()> = jvm.get_field(&field, "wieEditor", "Lnet/wie/LwcTextEditor;").await?;
                assert!(editor.is_null());
                for kind in [1, 2] {
                    let _: bool = jvm.invoke_virtual(&field, NAME, "keyNotify", "(II)Z", (kind, -5)).await?;
                }
                let editor: ClassInstanceRef<()> = jvm.get_field(&field, "wieEditor", "Lnet/wie/LwcTextEditor;").await?;
                assert!(!editor.is_null());
                // Release that opened the editor cannot immediately close it.
                assert!(
                    !jvm.invoke_virtual::<_, bool>(&editor, "org/kwis/msp/lcdui/Card", "keyNotify", "(II)Z", (2, -5))
                        .await?
                );
                for code in [-6, 57] {
                    for kind in [1, 2] {
                        assert!(
                            !jvm.invoke_virtual::<_, bool>(&editor, "org/kwis/msp/lcdui/Card", "keyNotify", "(II)Z", (kind, code))
                                .await?
                        );
                    }
                }
                assert_eq!(value(&jvm, &field).await?, "b9");
                for kind in [1, 2] {
                    let _: bool = jvm
                        .invoke_virtual(&editor, "org/kwis/msp/lcdui/Card", "keyNotify", "(II)Z", (kind, -5))
                        .await?;
                }
                let closed: ClassInstanceRef<()> = jvm.get_field(&field, "wieEditor", "Lnet/wie/LwcTextEditor;").await?;
                assert!(closed.is_null());
                assert_eq!(value(&jvm, &field).await?, "b9");
                let _: () = jvm.invoke_virtual(&field, NAME, "focusNotify", "(Z)V", (false,)).await?;
                assert_eq!(jvm.get_field::<i32>(&field, "wieLastKey", "I").await?, 0);
                Ok(())
            },
        )
    }
    #[test]
    fn fields_paint_visible_content_and_mask_passwords_without_changing_value() -> wie_util::Result<()> {
        run_jvm_test(Box::new([wie_midp::get_protos().into(), crate::get_protos().into()]), |jvm| async move {
            let mut images = vec![];
            for (text, constraint) in [("12", 2), ("**", 0), ("", 0)] {
                let string = JavaLangString::from_rust_string(&jvm, text).await?;
                let mut field: ClassInstanceRef<()> = jvm.new_class(FIELD, "(Ljava/lang/String;I)V", (string, constraint)).await?.into();
                jvm.put_field(&mut field, "w", "I", 64).await?;
                jvm.put_field(&mut field, "h", "I", 32).await?;
                assert!(jvm.invoke_virtual::<_, i32>(&field, NAME, "getPreferredHeight", "()I", ()).await? > 0);
                let image: ClassInstanceRef<()> = jvm
                    .invoke_static("org/kwis/msp/lcdui/Image", "createImage", "(II)Lorg/kwis/msp/lcdui/Image;", (64, 32))
                    .await?;
                let g: ClassInstanceRef<()> = jvm
                    .invoke_virtual(&image, "org/kwis/msp/lcdui/Image", "getGraphics", "()Lorg/kwis/msp/lcdui/Graphics;", ())
                    .await?;
                let _: () = jvm
                    .invoke_virtual(&field, NAME, "paintContent", "(Lorg/kwis/msp/lcdui/Graphics;)V", (g.clone(),))
                    .await?;
                let mut pixels = vec![];
                for y in 0..32 {
                    for x in 0..64 {
                        pixels.push(jvm.invoke_virtual::<_, i32>(&g, GRAPHICS, "getPixel", "(II)I", (x, y)).await?);
                    }
                }
                images.push(pixels);
                assert_eq!(value(&jvm, &field).await?, text);
            }
            assert_eq!(images[0], images[1]);
            assert_ne!(images[0], images[2]);
            assert!(images[0].contains(&0) && images[0].contains(&0xffffff));
            Ok(())
        })
    }
}
