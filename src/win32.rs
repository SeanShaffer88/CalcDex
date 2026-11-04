//! Tiny Windows FFI surface for the binary shell: the
//! single-instance mutex (Contract V4.0 §2/§6). No `windows`
//! crate is used; the declarations are kept minimal and
//! commented. Everything is a no-op on other platforms so the
//! shell still builds during development and in CI.

/// Name of the single-instance mutex. The GUID is CalcDex's own,
/// generated for v0.9.0; it must never change between releases or
/// two builds could run side by side against one data folder.
pub const SINGLE_INSTANCE_MUTEX: &str = "Global\\CalcDex-{11A5D674-3C31-43A1-A932-6D514EF598BE}";

/// RAII guard holding the single-instance mutex. Dropping it
/// closes the handle, which releases the name for the next
/// instance.
#[cfg(windows)]
pub struct InstanceGuard {
    handle: *mut std::ffi::c_void,
}

/// RAII guard holding the single-instance mutex (no-op off Windows).
#[cfg(not(windows))]
pub struct InstanceGuard;

/// Tries to claim the single-instance mutex for a window launch.
/// Returns `None` when another CalcDex process already holds it.
pub fn acquire_single_instance() -> Option<InstanceGuard> {
    imp::acquire_single_instance(SINGLE_INSTANCE_MUTEX)
}

/// True when another CalcDex process holds the single-instance
/// mutex right now. Used by `--clean`/`--uninstall`, which must
/// refuse to delete files the running window has open; ordinary
/// CLI calculation runs never take or probe the mutex.
pub fn another_instance_running() -> bool {
    imp::another_instance_running(SINGLE_INSTANCE_MUTEX)
}

#[cfg(windows)]
impl Drop for InstanceGuard {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            // SAFETY: the handle came from CreateMutexW in
            // `acquire_single_instance` and is closed exactly once,
            // here.
            imp::close_handle(self.handle);
            self.handle = std::ptr::null_mut();
        }
    }
}

#[cfg(not(windows))]
impl Drop for InstanceGuard {
    fn drop(&mut self) {}
}

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;

    use super::InstanceGuard;

    const ERROR_ALREADY_EXISTS: u32 = 183;

    // SAFETY for the whole block: these are the documented
    // kernel32 entry points with the exact signatures from the
    // Windows SDK; every call site below passes valid arguments
    // as documented per function.
    unsafe extern "system" {
        fn CreateMutexW(
            attributes: *mut c_void,
            initial_owner: i32,
            name: *const u16,
        ) -> *mut c_void;
        fn CloseHandle(handle: *mut c_void) -> i32;
        fn GetLastError() -> u32;
    }

    pub fn close_handle(handle: *mut c_void) {
        // SAFETY: caller guarantees a live handle from CreateMutexW.
        unsafe {
            CloseHandle(handle);
        }
    }

    /// Creates (or opens) the named mutex and reports whether it
    /// already existed. The returned handle is open either way;
    /// the caller closes it. Returns `(null, false)` when the
    /// mutex could not even be created.
    fn probe(name: &str) -> (*mut c_void, bool) {
        let name: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: `name` is a NUL-terminated UTF-16 string that
        // outlives the call; null attributes and no initial
        // ownership are the documented defaults. GetLastError is
        // read immediately after, as the API requires, before any
        // other Win32 call can overwrite it.
        unsafe {
            let handle = CreateMutexW(std::ptr::null_mut(), 0, name.as_ptr());
            if handle.is_null() {
                return (std::ptr::null_mut(), false);
            }
            (handle, GetLastError() == ERROR_ALREADY_EXISTS)
        }
    }

    pub fn acquire_single_instance(name: &str) -> Option<InstanceGuard> {
        let (handle, existed) = probe(name);
        if handle.is_null() {
            // The mutex could not even be created (exotic
            // failure): do not block startup over it.
            return Some(InstanceGuard {
                handle: std::ptr::null_mut(),
            });
        }
        if existed {
            close_handle(handle);
            return None;
        }
        Some(InstanceGuard { handle })
    }

    pub fn another_instance_running(name: &str) -> bool {
        let (handle, existed) = probe(name);
        if handle.is_null() {
            return false;
        }
        // The probe handle is closed at once: this process is not
        // claiming the name, only asking whether someone else did.
        close_handle(handle);
        existed
    }
}

#[cfg(not(windows))]
mod imp {
    use super::InstanceGuard;

    pub fn acquire_single_instance(_name: &str) -> Option<InstanceGuard> {
        Some(InstanceGuard)
    }

    pub fn another_instance_running(_name: &str) -> bool {
        false
    }
}
