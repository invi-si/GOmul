// WIPI 1.2.1 section 5.1.11. All mutable component data lives in guest memory.
use crate::context::WIPICContext;
use alloc::vec;
use wie_util::{Result, read_generic, read_null_terminated_string_bytes, write_generic};
use wipi_types::wipic::{WIPICIndirectPtr, WIPICWord};

const CONTEXT: u32 = 0x55494341;
const COMPONENT: u32 = 0x55494343;
const TEXT: u32 = 1;
const DATE: u32 = 2;
// tag, class, application, x, y, w, h, enabled, text buffer, max bytes, length, timestamp
const WORDS: usize = 12;
fn load(context: &dyn WIPICContext, cc: u32) -> Result<[u32; WORDS]> {
    let value: [u32; WORDS] = read_generic(context, context.data_ptr(WIPICIndirectPtr(cc))?)?;
    if value[0] != COMPONENT {
        return Err(wie_util::WieError::FatalError("Invalid WIPI component handle".into()));
    }
    Ok(value)
}
fn store(context: &mut dyn WIPICContext, cc: u32, value: [u32; WORDS]) -> Result<()> {
    write_generic(context, context.data_ptr(WIPICIndirectPtr(cc))?, value)
}
pub async fn create_application_context(context: &mut dyn WIPICContext) -> Result<WIPICIndirectPtr> {
    let pac = context.alloc(4)?;
    write_generic(context, context.data_ptr(pac)?, CONTEXT)?;
    Ok(pac)
}
pub async fn get_class(context: &mut dyn WIPICContext, psz: u32) -> Result<WIPICIndirectPtr> {
    let name = read_null_terminated_string_bytes(context, psz)?;
    Ok(WIPICIndirectPtr(match name.as_slice() {
        b"TextComponent" => TEXT,
        b"DateTimeComponent" => DATE,
        _ => (-1i32) as u32,
    }))
}
pub async fn create(context: &mut dyn WIPICContext, pac: u32, cls: u32) -> Result<WIPICIndirectPtr> {
    if ![TEXT, DATE].contains(&cls) || pac == 0 {
        return Ok(WIPICIndirectPtr(u32::MAX));
    }
    let tag: u32 = read_generic(context, context.data_ptr(WIPICIndirectPtr(pac))?)?;
    if tag != CONTEXT {
        return Ok(WIPICIndirectPtr(u32::MAX));
    }
    let mut value = [0; WORDS];
    value[0] = COMPONENT;
    value[1] = cls;
    value[2] = pac;
    if cls == TEXT {
        let buffer = context.alloc(256)?;
        value[8] = buffer.0;
        value[9] = 256;
    } else {
        value[11] = (context.system().platform().now().raw() / 1000) as u32;
    }
    let cc = context.alloc((WORDS * 4) as u32)?;
    store(context, cc.0, value)?;
    Ok(cc)
}
pub async fn destroy(context: &mut dyn WIPICContext, cc: u32) -> Result<()> {
    let value = load(context, cc)?;
    if value[8] != 0 {
        context.free(WIPICIndirectPtr(value[8]))?;
    }
    store(context, cc, [0; WORDS])?;
    context.free(WIPICIndirectPtr(cc))
}
pub async fn configure(context: &mut dyn WIPICContext, cc: u32, x: i32, y: i32, w: i32, h: i32, mask: u32) -> Result<()> {
    if w <= 0 || h <= 0 {
        return Ok(());
    }
    let mut value = load(context, cc)?;
    if mask & 1 != 0 {
        value[3] = x as u32;
        value[4] = y as u32;
    }
    if mask & 2 != 0 {
        value[5] = w as u32;
        value[6] = h as u32;
    }
    store(context, cc, value)
}
pub async fn get_geometry(context: &mut dyn WIPICContext, cc: u32, x: u32, y: u32, w: u32, h: u32) -> Result<()> {
    let value = load(context, cc)?;
    for (ptr, coordinate) in [x, y, w, h].into_iter().zip(&value[3..7]) {
        if ptr != 0 {
            write_generic(context, ptr, *coordinate)?;
        }
    }
    Ok(())
}
pub async fn set_enable(context: &mut dyn WIPICContext, cc: u32, enabled: i32) -> Result<()> {
    let mut value = load(context, cc)?;
    value[7] = u32::from(enabled != 0);
    store(context, cc, value)
}
pub async fn set_max_text_size(context: &mut dyn WIPICContext, cc: u32, max: i32) -> Result<i32> {
    let mut value = load(context, cc)?;
    if value[1] != TEXT || max < 0 {
        return Ok(-1);
    }
    let old = value[9];
    let length = value[10].min(max as u32);
    let mut bytes = vec![0; length as usize];
    context.read_bytes(context.data_ptr(WIPICIndirectPtr(value[8]))?, &mut bytes)?;
    let buffer = context.alloc((max as u32).max(1))?;
    context.write_bytes(context.data_ptr(buffer)?, &bytes)?;
    let previous = value[8];
    value[8] = buffer.0;
    value[9] = max as u32;
    value[10] = length;
    store(context, cc, value)?;
    context.free(WIPICIndirectPtr(previous))?;
    Ok(old as i32)
}
pub async fn insert_text(context: &mut dyn WIPICContext, cc: u32, idx: i32, psz: u32, len: i32) -> Result<i32> {
    let mut value = load(context, cc)?;
    if value[1] != TEXT || len <= 0 {
        return Ok(0);
    }
    let count = (len as u32).min(value[9] - value[10]);
    let idx = (idx.max(0) as u32).min(value[10]);
    let buffer = context.data_ptr(WIPICIndirectPtr(value[8]))?;
    let mut tail = vec![0; (value[10] - idx) as usize];
    context.read_bytes(buffer + idx, &mut tail)?;
    let mut insert = vec![0; count as usize];
    context.read_bytes(psz, &mut insert)?;
    context.write_bytes(buffer + idx, &insert)?;
    context.write_bytes(buffer + idx + count, &tail)?;
    value[10] += count;
    store(context, cc, value)?;
    Ok(count as i32)
}
pub async fn delete_text(context: &mut dyn WIPICContext, cc: u32, idx: i32, len: i32) -> Result<()> {
    let mut value = load(context, cc)?;
    if value[1] != TEXT || idx < 0 || idx as u32 >= value[10] || len < -1 {
        return Ok(());
    }
    let count = if len == -1 {
        value[10] - idx as u32
    } else {
        (len as u32).min(value[10] - idx as u32)
    };
    let buffer = context.data_ptr(WIPICIndirectPtr(value[8]))?;
    let mut tail = vec![0; (value[10] - idx as u32 - count) as usize];
    context.read_bytes(buffer + idx as u32 + count, &mut tail)?;
    context.write_bytes(buffer + idx as u32, &tail)?;
    value[10] -= count;
    store(context, cc, value)
}
pub async fn get_max_text_size(context: &mut dyn WIPICContext, cc: u32) -> Result<i32> {
    let value = load(context, cc)?;
    Ok(if value[1] == TEXT { value[9] as i32 } else { 0 })
}
pub async fn get_text_size(context: &mut dyn WIPICContext, cc: u32) -> Result<i32> {
    let value = load(context, cc)?;
    Ok(if value[1] == TEXT { value[10] as i32 } else { 0 })
}
pub async fn get_text(context: &mut dyn WIPICContext, cc: u32, idx: i32, out: u32, len: i32) -> Result<i32> {
    let value = load(context, cc)?;
    if value[1] != TEXT || len <= 0 {
        return Ok(0);
    }
    let idx = (idx.max(0) as u32).min(value[10]);
    let count = (len as u32).min(value[10] - idx);
    let mut bytes = vec![0; count as usize];
    context.read_bytes(context.data_ptr(WIPICIndirectPtr(value[8]))? + idx, &mut bytes)?;
    context.write_bytes(out, &bytes)?;
    Ok(count as i32)
}
pub async fn get_time(context: &mut dyn WIPICContext, cc: u32, out: u32) -> Result<()> {
    let value = load(context, cc)?;
    if value[1] != DATE || out == 0 {
        return Ok(());
    }
    let timestamp = value[11] as i64;
    let (year, month, day, hour, minute, second) = unix_seconds_to_utc(timestamp);
    let days = timestamp.div_euclid(86400);
    let jan1 = days
        - (1..month)
            .map(|m| match m {
                2 => {
                    if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                        29
                    } else {
                        28
                    }
                }
                4 | 6 | 9 | 11 => 30,
                _ => 31,
            })
            .sum::<i32>() as i64
        - day as i64
        + 1;
    write_generic(
        context,
        out,
        [
            second,
            minute,
            hour,
            day,
            month - 1,
            year - 1900,
            ((days + 4) % 7) as i32,
            (days - jan1) as i32,
            0i32,
        ],
    )
}
fn unix_seconds_to_utc(timestamp: i64) -> (i32, i32, i32, i32, i32, i32) {
    let days = timestamp.div_euclid(86_400);
    let seconds_of_day = timestamp.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = (seconds_of_day / 3600) as i32;
    let minute = ((seconds_of_day % 3600) / 60) as i32;
    let second = (seconds_of_day % 60) as i32;

    (year, month, day, hour, minute, second)
}

fn civil_from_days(days: i64) -> (i32, i32, i32) {
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_param = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_param + 2) / 5 + 1;
    let month = month_param + if month_param < 10 { 3 } else { -9 };
    year += if month <= 2 { 1 } else { 0 };

    (year as i32, month as i32, day as i32)
}

pub async fn get_menu_item(_context: &mut dyn WIPICContext, cc: WIPICWord, idx: u32, psz: WIPICWord, buflen: i32, img: WIPICWord) -> Result<i32> {
    tracing::warn!("stub MC_uicGetMenuItem({cc:#x}, {idx}, {psz:#x}, {buflen}, {img:#x})");

    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::test::TestContext;
    use wie_util::ByteWrite;

    async fn text(context: &mut TestContext) -> u32 {
        context.write_bytes(0x100, b"TextComponent\0").unwrap();
        let pac = create_application_context(context).await.unwrap();
        let cls = get_class(context, 0x100).await.unwrap();
        create(context, pac.0, cls.0).await.unwrap().0
    }
    #[futures_test::test]
    async fn geometry_masks_invalid_dimensions_and_optional_outputs() {
        let mut c = TestContext::new();
        let cc = text(&mut c).await;
        configure(&mut c, cc, 90, 248, 60, 17, 3).await.unwrap();
        configure(&mut c, cc, -5, 8, 4, 4, 1).await.unwrap();
        configure(&mut c, cc, 0, 0, 70, 20, 2).await.unwrap();
        configure(&mut c, cc, 1, 1, 0, 20, 3).await.unwrap();
        get_geometry(&mut c, cc, 0x200, 0x204, 0x208, 0x20c).await.unwrap();
        assert_eq!(read_generic::<[i32; 4], _>(&c, 0x200).unwrap(), [-5, 8, 70, 20]);
        get_geometry(&mut c, cc, 0, 0, 0, 0).await.unwrap();
        assert_eq!(load(&c, cc).unwrap()[7], 0);
        set_enable(&mut c, cc, 1).await.unwrap();
        assert_eq!(load(&c, cc).unwrap()[7], 1);
    }
    #[futures_test::test]
    async fn text_is_independent_copied_bounded_and_byte_addressed() {
        let mut c = TestContext::new();
        let a = text(&mut c).await;
        let b = text(&mut c).await;
        c.write_bytes(0x300, &[0xb0, 0xa1, b'A', b'B']).unwrap();
        assert_eq!(set_max_text_size(&mut c, a, 5).await.unwrap(), 256);
        assert_eq!(insert_text(&mut c, a, -1, 0x300, 4).await.unwrap(), 4);
        assert_eq!(insert_text(&mut c, a, 2, 0x302, 2).await.unwrap(), 1);
        c.write_bytes(0x300, &[0; 4]).unwrap();
        assert_eq!(get_text(&mut c, a, 0, 0x400, 20).await.unwrap(), 5);
        assert_eq!(read_generic::<[u8; 5], _>(&c, 0x400).unwrap(), [0xb0, 0xa1, b'A', b'A', b'B']);
        assert_eq!(get_text_size(&mut c, b).await.unwrap(), 0);
        assert_eq!(set_max_text_size(&mut c, a, 3).await.unwrap(), 5);
        assert_eq!(get_text_size(&mut c, a).await.unwrap(), 3);
        delete_text(&mut c, a, 2, -1).await.unwrap();
        assert_eq!(get_text_size(&mut c, a).await.unwrap(), 2);
        delete_text(&mut c, a, -1, 1).await.unwrap();
        delete_text(&mut c, a, 0, -2).await.unwrap();
        assert_eq!(get_text_size(&mut c, a).await.unwrap(), 2);
        assert_eq!(get_text(&mut c, a, 99, 0x400, 4).await.unwrap(), 0);
        destroy(&mut c, a).await.unwrap();
        assert!(load(&c, a).is_err());
        assert_eq!(get_max_text_size(&mut c, b).await.unwrap(), 256);
    }
    #[futures_test::test]
    async fn unsupported_class_and_wrong_component_do_not_fake_data() {
        let mut c = TestContext::new();
        c.write_bytes(0x100, b"MissingComponent\0").unwrap();
        assert_eq!(get_class(&mut c, 0x100).await.unwrap().0, u32::MAX);
        assert_eq!(create(&mut c, 0, 0).await.unwrap().0, u32::MAX);
        let cc = text(&mut c).await;
        write_generic(&mut c, 0x400, 0x12345678u32).unwrap();
        get_time(&mut c, cc, 0x400).await.unwrap();
        assert_eq!(read_generic::<u32, _>(&c, 0x400).unwrap(), 0x12345678);
    }
    #[futures_test::test]
    async fn date_value_is_per_instance_and_writes_full_tm() {
        let mut c = TestContext::new();
        let cc = text(&mut c).await;
        let mut value = load(&c, cc).unwrap();
        value[1] = DATE;
        value[11] = 951782400; // 2000-02-29 00:00 UTC
        store(&mut c, cc, value).unwrap();
        get_time(&mut c, cc, 0x400).await.unwrap();
        assert_eq!(read_generic::<[i32; 9], _>(&c, 0x400).unwrap(), [0, 0, 0, 29, 1, 100, 2, 59, 0]);
        get_time(&mut c, cc, 0).await.unwrap();
    }
}
