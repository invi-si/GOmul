use alloc::{string::String, vec::Vec};
use core::fmt::Display;

use wie_core_arm::ArmCore;
use wie_util::{read_generic, read_null_terminated_string_bytes};

use super::Result;

#[derive(Clone)]
pub struct JavaFullName {
    pub tag: u8,
    pub name: String,
    pub descriptor: String,
}

impl JavaFullName {
    // Lookup scans need equality, not separately allocated name/descriptor strings.
    // Read guest bytes each time so modified metadata stays authoritative.
    pub fn matches(core: &ArmCore, ptr: u32, name: &str, descriptor: &str) -> Result<bool> {
        let _: u8 = read_generic(core, ptr)?;
        let value = read_null_terminated_string_bytes(core, ptr + 1)?;
        let value = String::from_utf8(value).unwrap();
        let mut values = value.split('+');
        let stored_descriptor = values.next().unwrap();
        let stored_name = values.next().unwrap();
        Ok(stored_name == name && stored_descriptor == descriptor)
    }

    pub fn from_ptr(core: &ArmCore, ptr: u32) -> Result<Self> {
        let tag = read_generic(core, ptr)?;

        let value = read_null_terminated_string_bytes(core, ptr + 1)?;
        let value = String::from_utf8(value).unwrap();
        let mut values = value.split('+');

        let descriptor = values.next().unwrap().into();
        let name = values.next().unwrap().into();

        Ok(JavaFullName { tag, name, descriptor })
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.push(self.tag);
        bytes.extend_from_slice(self.descriptor.as_bytes());
        bytes.push(b'+');
        bytes.extend_from_slice(self.name.as_bytes());
        bytes.push(0);

        bytes
    }
}

impl Display for JavaFullName {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.name.fmt(f)?;
        self.descriptor.fmt(f)?;
        write!(f, "@{}", self.tag)?;

        Ok(())
    }
}

impl PartialEq for JavaFullName {
    fn eq(&self, other: &Self) -> bool {
        self.descriptor == other.descriptor && self.name == other.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wie_util::ByteWrite;

    #[test]
    fn borrowed_comparison_matches_owned_names_and_observes_guest_writes() -> Result<()> {
        let mut core = ArmCore::new(false, None)?;
        core.load(&[0], 0x10000, 65536)?;
        for (name, descriptor) in [("a", "I"), ("paint", "(Ljava/lang/Object;)V"), ("한글", "[B")] {
            let bytes = JavaFullName {
                tag: 37,
                name: name.into(),
                descriptor: descriptor.into(),
            }
            .as_bytes();
            let ptr = 0x20000 - bytes.len() as u32;
            core.write_bytes(ptr, &bytes)?;
            let owned = JavaFullName::from_ptr(&core, ptr)?;
            for (query_name, query_descriptor) in [(name, descriptor), ("", descriptor), (name, "J"), ("paintExtra", descriptor)] {
                assert_eq!(
                    JavaFullName::matches(&core, ptr, query_name, query_descriptor)?,
                    owned.name == query_name && owned.descriptor == query_descriptor
                );
            }
            core.write_bytes(ptr + 1, b"Z")?;
            assert!(!JavaFullName::matches(&core, ptr, name, descriptor)?);
            core.write_bytes(ptr, &bytes)?;
            assert!(JavaFullName::matches(&core, ptr, name, descriptor)?);
        }
        core.write_bytes(0x1fffe, &[0, b'I'])?;
        assert!(JavaFullName::from_ptr(&core, 0x1fffe).is_err());
        assert!(JavaFullName::matches(&core, 0x1fffe, "a", "I").is_err());
        Ok(())
    }
}
