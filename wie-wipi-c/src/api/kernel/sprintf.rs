use alloc::{format, vec::Vec};

use wie_util::{ByteRead, Result, read_generic};

const MAX_WIDTH: usize = 4096;

fn read_string(context: &(impl ByteRead + ?Sized), ptr: u32, precision: Option<usize>) -> Result<Vec<u8>> {
    if ptr == 0 {
        return Ok(b"(null)"[..precision.unwrap_or(6).min(6)].to_vec());
    }
    let mut bytes = Vec::new();
    while precision.is_none_or(|limit| bytes.len() < limit) {
        let byte: u8 = read_generic(context, ptr.wrapping_add(bytes.len() as u32))?;
        if byte == 0 {
            break;
        }
        bytes.push(byte);
    }
    Ok(bytes)
}

pub fn sprintf(context: &(impl ByteRead + ?Sized), format_bytes: &[u8], args: &[u32]) -> Result<Vec<u8>> {
    let mut args = args.iter();
    format_with_args(
        format_bytes,
        &mut || {
            Ok(args.next().copied().unwrap_or_else(|| {
                tracing::warn!("printf: more format specifiers than arguments");
                0
            }))
        },
        &mut |ptr, precision| read_string(context, ptr, precision),
    )
}

/// ARM32 va_list points to consecutive guest argument words. Read only consumed words.
pub fn vsprintf(context: &(impl ByteRead + ?Sized), format_bytes: &[u8], mut arguments: u32) -> Result<Vec<u8>> {
    format_with_args(
        format_bytes,
        &mut || {
            let value = read_generic::<u32, _>(context, arguments)?;
            arguments = arguments.wrapping_add(4);
            Ok(value)
        },
        &mut |ptr, precision| read_string(context, ptr, precision),
    )
}

// C string widths and precisions are bytes, including incomplete EUC-KR sequences.
fn format_with_args(
    format: &[u8],
    next: &mut dyn FnMut() -> Result<u32>,
    read_string: &mut dyn FnMut(u32, Option<usize>) -> Result<Vec<u8>>,
) -> Result<Vec<u8>> {
    let mut result = Vec::with_capacity(format.len());
    let mut i = 0;
    while i < format.len() {
        if format[i] != b'%' {
            result.push(format[i]);
            i += 1;
            continue;
        }
        let start = i;
        i += 1;
        let mut left = false;
        let mut zero = false;
        while i < format.len() && matches!(format[i], b'-' | b'0') {
            left |= format[i] == b'-';
            zero |= format[i] == b'0';
            i += 1;
        }
        let mut width = 0usize;
        if format.get(i) == Some(&b'*') {
            let value = next()? as i32;
            left |= value < 0;
            width = (value.unsigned_abs() as usize).min(MAX_WIDTH);
            i += 1;
        } else {
            while let Some(c @ b'0'..=b'9') = format.get(i) {
                width = width.saturating_mul(10).saturating_add((c - b'0') as usize).min(MAX_WIDTH);
                i += 1;
            }
        }
        let mut precision = None;
        if format.get(i) == Some(&b'.') {
            i += 1;
            if format.get(i) == Some(&b'*') {
                let value = next()? as i32;
                if value >= 0 {
                    precision = Some(value as usize);
                }
                i += 1;
            } else {
                let mut value = 0usize;
                while let Some(c @ b'0'..=b'9') = format.get(i) {
                    value = value.saturating_mul(10).saturating_add((c - b'0') as usize);
                    i += 1;
                }
                precision = Some(value);
            }
        }
        let mut longs = 0;
        while format.get(i) == Some(&b'l') {
            longs += 1;
            i += 1;
        }
        let Some(&conversion) = format.get(i) else {
            result.extend_from_slice(&format[start..]);
            break;
        };
        i += 1;
        let mut value;
        match conversion {
            b'%' => {
                result.push(b'%');
                continue;
            }
            b's' => {
                value = read_string(next()?, precision)?;
                zero = false;
            }
            b'c' => {
                value = alloc::vec![next()? as u8];
                zero = false;
            }
            b'd' | b'u' | b'x' => {
                let raw = if longs >= 2 {
                    let low = next()? as u64;
                    low | ((next()? as u64) << 32)
                } else {
                    next()? as u64
                };
                value = match conversion {
                    b'd' => format!("{}", if longs >= 2 { raw as i64 } else { raw as u32 as i32 as i64 }).into_bytes(),
                    b'u' => format!("{raw}").into_bytes(),
                    _ => format!("{raw:x}").into_bytes(),
                };
                if let Some(digits) = precision {
                    zero = false;
                    if digits == 0 && raw == 0 {
                        value.clear();
                    }
                    let sign = usize::from(value.first() == Some(&b'-'));
                    let padding = digits.min(MAX_WIDTH).saturating_sub(value.len() - sign);
                    value.splice(sign..sign, core::iter::repeat_n(b'0', padding));
                }
            }
            _ => {
                tracing::warn!("unsupported printf conversion: {conversion:#x}");
                result.extend_from_slice(&format[start..i]);
                continue;
            }
        }
        let padding = width.saturating_sub(value.len());
        if !left {
            if zero && value.first() == Some(&b'-') {
                result.push(b'-');
                value.remove(0);
            }
            result.extend(core::iter::repeat_n(if zero { b'0' } else { b' ' }, padding));
        }
        result.extend_from_slice(&value);
        if left {
            result.extend(core::iter::repeat_n(b' ', padding));
        }
    }
    Ok(result)
}

#[cfg(test)]
mod test {
    use alloc::{string::String, vec, vec::Vec};

    use wie_util::{ByteRead, Result, WieError};

    struct GuestMemory {
        bytes: Vec<u8>,
    }

    impl ByteRead for GuestMemory {
        fn read_bytes(&self, address: u32, result: &mut [u8]) -> Result<usize> {
            let start = address as usize;
            let end = start + result.len();
            if end > self.bytes.len() {
                return Err(WieError::InvalidMemoryAccess(address));
            }

            result.copy_from_slice(&self.bytes[start..end]);
            Ok(result.len())
        }
    }

    fn format(format_string: &str, args: &[u32]) -> Result<String> {
        let mut args = args.iter();
        let bytes = super::format_with_args(
            format_string.as_bytes(),
            &mut || Ok(args.next().copied().unwrap_or(0)),
            &mut |ptr, precision| {
                let bytes = if ptr == 0 { b"(null)".as_slice() } else { b"stub".as_slice() };
                Ok(bytes[..precision.unwrap_or(bytes.len()).min(bytes.len())].to_vec())
            },
        )?;
        Ok(String::from_utf8(bytes).unwrap())
    }

    #[test]
    fn string_precision_preserves_guest_bytes_and_argument_order() -> Result<()> {
        let mut memory = GuestMemory { bytes: vec![0; 32] };
        // Deliberately no NUL after the final EUC-KR character.
        memory.bytes[30..].copy_from_slice(&[0xb0, 0xa1]);
        assert_eq!(super::sprintf(&memory, b"[%.*s] %u", &[2, 30, 7])?, [b'[', 0xb0, 0xa1, b']', b' ', b'7']);
        assert_eq!(super::sprintf(&memory, b"%.*s", &[1, 30])?, [0xb0]);
        assert_eq!(super::sprintf(&memory, b"%.0s!", &[32])?, b"!");
        assert!(super::sprintf(&memory, b"%.*s", &[3, 30]).is_err());
        for (i, word) in [4u32, 2, 30, 9].iter().enumerate() {
            memory.bytes[4 + i * 4..8 + i * 4].copy_from_slice(&word.to_le_bytes());
        }
        assert_eq!(super::vsprintf(&memory, b"%*.*s/%d", 4)?, [b' ', b' ', 0xb0, 0xa1, b'/', b'9']);
        Ok(())
    }

    #[test]
    fn dynamic_width_and_precision_follow_c_argument_rules() -> Result<()> {
        assert_eq!(format("%*.*s %d", &[6, 2, 1, 7])?, "    st 7");
        assert_eq!(format("%*s!", &[(-6i32) as u32, 1])?, "stub  !");
        assert_eq!(format("%.*s %d", &[(-1i32) as u32, 1, 7])?, "stub 7");
        assert_eq!(format("%.s", &[1])?, "");
        assert_eq!(format("%.*d", &[4, (-7i32) as u32])?, "-0007");
        assert_eq!(format("%05d", &[(-7i32) as u32])?, "-0007");
        assert_eq!(format("%*s", &[i32::MIN as u32, 1])?.len(), 4096);
        Ok(())
    }

    #[test]
    fn variadic_arguments_are_read_lazily_and_faults_propagate() -> Result<()> {
        let mut memory = GuestMemory { bytes: vec![0; 8] };
        memory.bytes[4..8].copy_from_slice(&42u32.to_le_bytes());
        assert_eq!(super::vsprintf(&memory, b"plain %%", 0)?, b"plain %");
        assert_eq!(super::vsprintf(&memory, b"%u", 4)?, b"42");
        assert!(super::vsprintf(&memory, b"%u %u", 4).is_err());
        Ok(())
    }

    #[test]
    fn test_unsigned() -> Result<()> {
        assert_eq!(format("%u", &[0xffff_ffff])?, "4294967295");

        Ok(())
    }

    #[test]
    fn test_unknown_specifier_passthrough() -> Result<()> {
        assert_eq!(format("a%qb", &[])?, "a%qb");

        Ok(())
    }

    #[test]
    fn test_more_specifiers_than_args() -> Result<()> {
        assert_eq!(format("%d %d %d %d %d", &[1, 2, 3, 4])?, "1 2 3 4 0");

        Ok(())
    }

    #[test]
    fn test_width_and_zero_flag() -> Result<()> {
        assert_eq!(format("%02d", &[1])?, "01");
        assert_eq!(format("%10d", &[42])?, "        42");
        assert_eq!(format("%d", &[0xffff_ffff])?, "-1");

        Ok(())
    }

    #[test]
    fn test_long_specifiers() -> Result<()> {
        // ILP32: long is one word, so %ld must not shift later arguments
        assert_eq!(format("%ld", &[0xffff_ffff])?, "-1");
        assert_eq!(format("%lu", &[0xffff_ffff])?, "4294967295");
        assert_eq!(format("%ld %d", &[1, 42])?, "1 42");

        // long long arguments are two words, low first
        assert_eq!(format("%lld", &[0xffff_ffff, 0xffff_ffff])?, "-1");
        assert_eq!(format("%llu", &[0xffff_ffff, 0xffff_ffff])?, "18446744073709551615");
        assert_eq!(format("%llx", &[0x9abc_def0, 0x1234_5678])?, "123456789abcdef0");
        assert_eq!(format("%lld %d", &[1, 0, 42])?, "1 42");

        Ok(())
    }

    #[test]
    fn test_hex_width() -> Result<()> {
        assert_eq!(format("%08x", &[0xbeef])?, "0000beef");
        assert_eq!(format("%8x", &[0xbeef])?, "    beef");

        Ok(())
    }

    #[test]
    fn test_huge_width_is_clamped() -> Result<()> {
        // core::fmt panics on width >= 65536; must not propagate guest width unclamped
        assert_eq!(format("%65536d", &[1])?.len(), 4096);
        assert_eq!(format("%99999999999999999999d", &[1])?.len(), 4096);

        Ok(())
    }

    #[test]
    fn test_string_and_null() -> Result<()> {
        assert_eq!(format("%s!", &[1])?, "stub!");
        assert_eq!(format("%s", &[0])?, "(null)");

        Ok(())
    }

    #[test]
    fn sprintf_formats_euc_kr_guest_strings_and_values() -> Result<()> {
        let mut memory = GuestMemory { bytes: vec![0; 0x100] };
        let guest_string = encoding_rs::EUC_KR.encode("세계").0;
        memory.bytes[0x40..0x40 + guest_string.len()].copy_from_slice(&guest_string);
        memory.bytes[0x40 + guest_string.len()] = 0;

        let format = encoding_rs::EUC_KR.encode("안녕 %s %c %d %u %x").0;
        let result = super::sprintf(&memory, &format, &[0x40, b'A' as u32, (-7i32) as u32, 42, 0xbeef])?;

        let expected = "안녕 세계 A -7 42 beef";
        assert_eq!(encoding_rs::EUC_KR.decode(&result).0, expected);
        assert_eq!(result, encoding_rs::EUC_KR.encode(expected).0.into_owned());

        Ok(())
    }
}
