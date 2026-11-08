//! APFS-aware size accounting.
//!
//! On APFS, cloned files (`cp -c`, Finder duplicate, Xcode simulator clones…)
//! share data blocks. `st_blocks` reports the full size for every clone, so
//! summing it over-promises how much space deleting them will free.
//!
//! We ask the kernel, per file, whether it *may* share blocks
//! (`EF_MAY_SHARE_BLOCKS`) and how many bytes are private to it
//! (`ATTR_CMNEXT_PRIVATESIZE`). For shared files we count only the private
//! bytes. This is a conservative estimate: deleting every clone of a set can
//! free more than we report, never less.

use std::path::Path;

/// Information returned by `getattrlist` for a single file.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CloneInfo {
    /// True if the file may share blocks with another file (clone).
    pub may_share_blocks: bool,
    /// Bytes allocated only to this file (not shared). `None` if unavailable.
    pub private_size: Option<u64>,
    /// APFS clone id, if reported.
    pub clone_id: Option<u64>,
}

#[cfg(target_os = "macos")]
mod imp {
    use super::CloneInfo;
    use std::ffi::CString;
    use std::os::raw::{c_char, c_int, c_void};
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    #[repr(C)]
    struct AttrList {
        bitmapcount: u16,
        reserved: u16,
        commonattr: u32,
        volattr: u32,
        dirattr: u32,
        fileattr: u32,
        forkattr: u32,
    }

    extern "C" {
        // On LP64 macOS the options argument is `unsigned int`.
        fn getattrlist(
            path: *const c_char,
            attr_list: *mut c_void,
            attr_buf: *mut c_void,
            attr_buf_size: usize,
            options: u32,
        ) -> c_int;
    }

    const ATTR_BIT_MAP_COUNT: u16 = 5;
    const ATTR_CMN_RETURNED_ATTRS: u32 = 0x8000_0000;
    const ATTR_CMNEXT_PRIVATESIZE: u32 = 0x0000_0008;
    const ATTR_CMNEXT_CLONEID: u32 = 0x0000_0100;
    const ATTR_CMNEXT_EXT_FLAGS: u32 = 0x0000_0200;
    const FSOPT_NOFOLLOW: u32 = 0x0000_0001;
    const FSOPT_ATTR_CMN_EXTENDED: u32 = 0x0000_0020;
    const EF_MAY_SHARE_BLOCKS: u64 = 0x0000_0001;

    pub fn clone_info(path: &Path) -> Option<CloneInfo> {
        let c_path = CString::new(path.as_os_str().as_bytes()).ok()?;
        let mut list = AttrList {
            bitmapcount: ATTR_BIT_MAP_COUNT,
            reserved: 0,
            commonattr: ATTR_CMN_RETURNED_ATTRS,
            volattr: 0,
            dirattr: 0,
            fileattr: 0,
            forkattr: ATTR_CMNEXT_PRIVATESIZE | ATTR_CMNEXT_CLONEID | ATTR_CMNEXT_EXT_FLAGS,
        };
        let mut buf = [0u8; 128];

        // SAFETY: all pointers are valid for the duration of the call and the
        // buffer size passed matches the buffer.
        let rc = unsafe {
            getattrlist(
                c_path.as_ptr(),
                &mut list as *mut AttrList as *mut c_void,
                buf.as_mut_ptr() as *mut c_void,
                buf.len(),
                FSOPT_NOFOLLOW | FSOPT_ATTR_CMN_EXTENDED,
            )
        };
        if rc != 0 {
            return None;
        }

        let read_u32 = |off: usize| -> Option<u32> {
            buf.get(off..off + 4)
                .map(|b| u32::from_ne_bytes([b[0], b[1], b[2], b[3]]))
        };
        let read_u64 = |off: usize| -> Option<u64> {
            buf.get(off..off + 8).map(|b| {
                u64::from_ne_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
            })
        };

        let total_len = read_u32(0)? as usize;
        // attribute_set_t returned: common, vol, dir, file, fork (5 x u32)
        let returned_fork = read_u32(4 + 16)?;
        let mut off = 4 + 20;

        let mut info = CloneInfo::default();
        if returned_fork & ATTR_CMNEXT_PRIVATESIZE != 0 {
            if off + 8 > total_len {
                return Some(info);
            }
            info.private_size = read_u64(off);
            off += 8;
        }
        if returned_fork & ATTR_CMNEXT_CLONEID != 0 {
            if off + 8 > total_len {
                return Some(info);
            }
            info.clone_id = read_u64(off);
            off += 8;
        }
        if returned_fork & ATTR_CMNEXT_EXT_FLAGS != 0 && off + 8 <= total_len {
            let flags = read_u64(off).unwrap_or(0);
            info.may_share_blocks = flags & EF_MAY_SHARE_BLOCKS != 0;
        }
        Some(info)
    }
}

/// Query clone information for a file (macOS only; `None` elsewhere or on error).
pub fn clone_info(path: &Path) -> Option<CloneInfo> {
    #[cfg(target_os = "macos")]
    {
        imp::clone_info(path)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        None
    }
}

/// Bytes that deleting this single file would actually free.
///
/// `allocated` is the file's `st_blocks * 512`. For files that may share
/// blocks (APFS clones) we return the private size instead.
pub fn reclaimable_bytes(path: &Path, allocated: u64) -> u64 {
    match clone_info(path) {
        Some(info) if info.may_share_blocks => info
            .private_size
            .map(|p| p.min(allocated))
            .unwrap_or(allocated),
        _ => allocated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regular_file_reclaimable_equals_allocated_or_less() {
        let dir = tempfile::TempDir::new().unwrap();
        let p = dir.path().join("a.bin");
        std::fs::write(&p, vec![7u8; 8192]).unwrap();
        let r = reclaimable_bytes(&p, 8192);
        assert!(r <= 8192);
    }

    #[test]
    fn missing_file_falls_back_to_allocated() {
        let r = reclaimable_bytes(Path::new("/nonexistent/tidymac/x"), 4096);
        assert_eq!(r, 4096);
    }
}
