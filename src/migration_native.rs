//! Private Windows migration operations; names are single internal components.
//! ABI: Windows SDK 10.0.26100.0 um/winternl.h and um/winbase.h.
use std::ffi::c_void;
use std::fs::File;
use std::io;
use std::os::windows::io::{AsRawHandle, FromRawHandle};
use std::ptr::null_mut;

#[repr(C)]
struct UnicodeString {
    length: u16,
    maximum_length: u16,
    buffer: *mut u16,
}

#[repr(C)]
struct ObjectAttributes {
    length: u32,
    root: *mut c_void,
    name: *mut UnicodeString,
    attributes: u32,
    security_descriptor: *mut c_void,
    security_quality_of_service: *mut c_void,
}

#[repr(C)]
struct IoStatusBlock {
    status_or_pointer: usize,
    information: usize,
}

#[repr(C)]
struct RenameInfo {
    flags: u32,
    root: *mut c_void,
    length: u32,
    name: [u16; 1],
}

#[link(name = "ntdll")]
extern "system" {
    fn NtCreateFile(
        handle: *mut *mut c_void,
        access: u32,
        attributes: *mut ObjectAttributes,
        status: *mut IoStatusBlock,
        allocation_size: *mut i64,
        file_attributes: u32,
        share: u32,
        disposition: u32,
        options: u32,
        ea_buffer: *mut c_void,
        ea_length: u32,
    ) -> i32;
    fn RtlNtStatusToDosError(status: i32) -> u32;
}

#[link(name = "kernel32")]
extern "system" {
    fn SetFileInformationByHandle(
        handle: *mut c_void,
        class: i32,
        information: *const c_void,
        size: u32,
    ) -> i32;
}

pub(super) fn create(parent: &File, name: &str, directory: bool) -> io::Result<File> {
    // Only generated staging names and fixed output names cross this boundary.
    if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\', ':', '\0']) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid migration component",
        ));
    }
    let mut wide: Vec<u16> = name.encode_utf16().collect();
    let length = u16::try_from(wide.len() * 2)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "migration name too long"))?;
    let mut unicode = UnicodeString {
        length,
        maximum_length: length,
        buffer: wide.as_mut_ptr(),
    };
    let mut attributes = ObjectAttributes {
        length: size_of::<ObjectAttributes>() as u32,
        root: parent.as_raw_handle(),
        name: &mut unicode,
        attributes: 0x1040, // OBJ_CASE_INSENSITIVE | OBJ_DONT_REPARSE
        security_descriptor: null_mut(),
        security_quality_of_service: null_mut(),
    };
    let mut status = IoStatusBlock {
        status_or_pointer: 0,
        information: 0,
    };
    let mut handle = null_mut();
    // DELETE | SYNCHRONIZE | READ_ATTRIBUTES; directory LIST/ADD_FILE or file WRITE_DATA.
    let access = 0x00110080 | if directory { 3 } else { 2 };
    // All buffers live through this synchronous call. FILE_CREATE never acquires an existing object.
    let result = unsafe {
        NtCreateFile(
            &mut handle,
            access,
            &mut attributes,
            &mut status,
            null_mut(),
            0x80,
            if directory { 3 } else { 1 },
            2,
            0x20 | if directory { 1 } else { 0x40 },
            null_mut(),
            0,
        )
    };
    if result < 0 {
        // Conversion has no ownership side effects; a failed create returns no owned handle.
        return Err(io::Error::from_raw_os_error(
            unsafe { RtlNtStatusToDosError(result) } as i32,
        ));
    }
    // The successful native call transfers this handle exactly once.
    let file = unsafe { File::from_raw_handle(handle) };
    if status.information != 2 {
        return Err(io::Error::other(
            "native migration create did not report FILE_CREATED",
        ));
    }
    Ok(file)
}

pub(super) fn delete(file: &File) -> io::Result<()> {
    // FILE_DISPOSITION_INFO::DeleteFile (BOOLEAN).
    // The one-byte SDK structure and retained DELETE-capable handle are valid for the call.
    let delete: u8 = 1;
    if unsafe {
        SetFileInformationByHandle(file.as_raw_handle(), 4, (&delete as *const u8).cast(), 1)
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

pub(super) fn publish(stage: &File, target: &std::path::Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    if !target.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "migration target must be absolute",
        ));
    }
    let wide: Vec<u16> = target.as_os_str().encode_wide().collect();
    let bytes = wide
        .len()
        .checked_mul(2)
        .and_then(|n| u32::try_from(n).ok())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "migration target too long"))?;
    let offset = std::mem::offset_of!(RenameInfo, name);
    let size = std::mem::size_of::<RenameInfo>().max(offset + bytes as usize + 2);
    let mut buffer = vec![0u64; size.div_ceil(8)];
    let length = u32::try_from(buffer.len() * 8)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "migration rename too large"))?;
    let ptr = buffer.as_mut_ptr().cast::<u8>();
    // Zeroed, 8-aligned SDK buffer: ReplaceIfExists=false, RootDirectory=NULL.
    // Parent/ancestor pins remain live; the source is the original creation handle.
    unsafe {
        ptr.add(std::mem::offset_of!(RenameInfo, length))
            .cast::<u32>()
            .write(bytes);
        std::ptr::copy_nonoverlapping(wide.as_ptr().cast::<u8>(), ptr.add(offset), bytes as usize);
        if SetFileInformationByHandle(stage.as_raw_handle(), 3, ptr.cast(), length) == 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sdk_x64_layouts() {
        assert_eq!(std::mem::size_of::<UnicodeString>(), 16);
        assert_eq!(std::mem::offset_of!(UnicodeString, buffer), 8);
        assert_eq!(std::mem::size_of::<ObjectAttributes>(), 48);
        assert_eq!(std::mem::offset_of!(ObjectAttributes, root), 8);
        assert_eq!(std::mem::offset_of!(ObjectAttributes, name), 16);
        assert_eq!(std::mem::offset_of!(ObjectAttributes, attributes), 24);
        assert_eq!(std::mem::size_of::<IoStatusBlock>(), 16);
        assert_eq!(std::mem::offset_of!(IoStatusBlock, information), 8);
        assert_eq!(std::mem::size_of::<RenameInfo>(), 24);
        assert_eq!(std::mem::align_of::<RenameInfo>(), 8);
        assert_eq!(std::mem::offset_of!(RenameInfo, root), 8);
        assert_eq!(std::mem::offset_of!(RenameInfo, length), 16);
        assert_eq!(std::mem::offset_of!(RenameInfo, name), 20);
    }

    #[test]
    fn owned_fixture_absolute_publication_collision_and_cleanup() {
        use std::io::Write;
        use std::os::windows::fs::OpenOptionsExt;
        for collision in [false, true] {
            let root = std::env::temp_dir().join(format!(
                "calcdex-stage-final-b-native-{}-{collision}",
                std::process::id()
            ));
            std::fs::create_dir(&root).unwrap();
            let parent = std::fs::OpenOptions::new()
                .access_mode(0)
                .share_mode(3)
                .custom_flags(0x02200000)
                .open(&root)
                .unwrap();
            let stage = create(&parent, "stage", true).unwrap();
            assert_eq!(
                create(&parent, "stage", true).unwrap_err().kind(),
                io::ErrorKind::AlreadyExists
            );
            assert!(std::fs::rename(root.join("stage"), root.join("displaced")).is_err());
            let mut child = create(&stage, "child", false).unwrap();
            child.write_all(b"owned bytes").unwrap();
            child.sync_all().unwrap();
            assert!(std::fs::remove_file(root.join("stage/child")).is_err());
            assert!(
                std::fs::rename(root.join("stage/child"), root.join("stage/replaced")).is_err()
            );
            assert!(std::fs::write(root.join("stage/child"), b"replacement").is_err());
            std::fs::write(root.join("stage/foreign"), b"foreign sentinel").unwrap();
            delete(&child).unwrap();
            drop(child);
            assert!(!root.join("stage/child").exists());
            assert!(delete(&stage).is_err());
            assert_eq!(
                std::fs::read(root.join("stage/foreign")).unwrap(),
                b"foreign sentinel"
            );
            let mut child = create(&stage, "published", false).unwrap();
            child.write_all(b"published bytes").unwrap();
            child.sync_all().unwrap();
            drop(child);
            if collision {
                std::fs::create_dir(root.join("data")).unwrap();
                std::fs::write(root.join("data/sentinel"), b"destination sentinel").unwrap();
            }
            assert!(std::fs::rename(root.join("stage"), root.join("displaced")).is_err());
            let result = publish(&stage, &root.join("data"));
            if collision {
                assert!(result.is_err(), "{result:?}");
                assert_eq!(
                    std::fs::read(root.join("data/sentinel")).unwrap(),
                    b"destination sentinel"
                );
                assert_eq!(
                    std::fs::read(root.join("stage/published")).unwrap(),
                    b"published bytes"
                );
                assert!(delete(&stage).is_err());
            } else {
                assert!(
                    result.is_ok(),
                    "absolute retained-guard publication: {result:?}"
                );
                assert_eq!(
                    std::fs::read(root.join("data/published")).unwrap(),
                    b"published bytes"
                );
                assert_eq!(
                    std::fs::read(root.join("data/foreign")).unwrap(),
                    b"foreign sentinel"
                );
            }
            let empty = create(&parent, "empty", true).unwrap();
            delete(&empty).unwrap();
            drop(empty);
            assert!(!root.join("empty").exists());
            // Fixture retained for inspection; no path-based ownership cleanup.
        }
    }
}
