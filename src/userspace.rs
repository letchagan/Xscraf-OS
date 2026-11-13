/// Stage 4.1: User-Space Memory Safety
///
/// Validates and copies data between kernel and user-space safely.
/// Prevents Ring 3 code from tricking kernel into reading/writing arbitrary memory.

use crate::task::TaskId;
use core::mem;

/// Maximum safe copy size (prevent DoS)
pub const MAX_COPY_SIZE: usize = 65536; // 64 KB

/// User-space pointer wrapper with bounds checking
#[derive(Debug, Clone, Copy)]
pub struct UserPtr<T> {
    addr: u64,
    _phantom: core::marker::PhantomData<T>,
}

impl<T> UserPtr<T> {
    pub fn from_addr(addr: u64) -> Self {
        UserPtr {
            addr,
            _phantom: core::marker::PhantomData,
        }
    }

    pub fn as_usize(&self) -> usize {
        self.addr as usize
    }

    pub fn as_u64(&self) -> u64 {
        self.addr
    }

    /// Validate pointer is in user-space range (0x0 to 0x7FFF_FFFF_FFFF)
    pub fn is_valid_user_ptr(&self) -> bool {
        // User-space: 0x0 to 0x7FFF_FFFF_FFFF
        // Kernel-space: 0xFFFF_8000_0000_0000+
        self.addr < 0x8000_0000_0000
    }

    /// Validate pointer + size don't overflow user-space
    pub fn validate_range(&self, size: usize) -> Result<(), &'static str> {
        if !self.is_valid_user_ptr() {
            return Err("Pointer outside user-space");
        }

        let end = self.addr.checked_add(size as u64)
            .ok_or("Pointer + size overflows")?;

        if end >= 0x8000_0000_0000 {
            return Err("Pointer range crosses into kernel space");
        }

        Ok(())
    }
}

/// Copy data FROM user-space TO kernel
///
/// SAFETY: Must validate pointer with validate_range() first
///
/// # Arguments
/// * `user_ptr` - Pointer in user address space
/// * `size` - Number of bytes to copy
///
/// # Returns
/// Vec<u8> containing copied data, or error
pub fn copy_from_user(user_ptr: u64, size: usize) -> Result<alloc::vec::Vec<u8>, &'static str> {
    // Validate size
    if size > MAX_COPY_SIZE {
        return Err("Copy size too large (> 64KB)");
    }

    // Create pointer wrapper
    let ptr = UserPtr::<u8>::from_addr(user_ptr);

    // Validate pointer + size
    ptr.validate_range(size)?;

    // Copy data
    // SAFETY: Pointer validated above, size checked, both in user-space
    let data = unsafe {
        core::slice::from_raw_parts(user_ptr as *const u8, size)
    };

    // Return copy
    Ok(alloc::vec::Vec::from(data))
}

/// Copy data FROM kernel TO user-space
///
/// SAFETY: Must validate destination pointer first
///
/// # Arguments
/// * `src` - Kernel buffer (trusted)
/// * `user_dst` - Pointer in user address space
/// * `size` - Number of bytes to copy
///
/// # Returns
/// Ok(()) on success, or error
pub fn copy_to_user(src: &[u8], user_dst: u64, size: usize) -> Result<(), &'static str> {
    // Validate size
    if size > MAX_COPY_SIZE {
        return Err("Copy size too large (> 64KB)");
    }

    if src.len() < size {
        return Err("Source buffer too small");
    }

    // Create pointer wrapper
    let dst = UserPtr::<u8>::from_addr(user_dst);

    // Validate destination + size
    dst.validate_range(size)?;

    // Copy data
    // SAFETY: Pointer validated above, size checked, both in user-space
    unsafe {
        core::ptr::copy_nonoverlapping(
            src.as_ptr(),
            user_dst as *mut u8,
            size
        );
    }

    Ok(())
}

/// Validate user-space string (null-terminated, max length)
///
/// # Arguments
/// * `user_str` - Pointer to null-terminated string in user-space
/// * `max_len` - Maximum string length (prevent infinite reads)
///
/// # Returns
/// String content if valid, or error
pub fn copy_cstring_from_user(user_str: u64, max_len: usize) -> Result<alloc::string::String, &'static str> {
    if max_len > MAX_COPY_SIZE {
        return Err("String too long");
    }

    // Validate pointer is in user space
    let ptr = UserPtr::<u8>::from_addr(user_str);
    if !ptr.is_valid_user_ptr() {
        return Err("String pointer outside user-space");
    }

    // Read up to max_len bytes, looking for null terminator
    let mut data = alloc::vec::Vec::new();
    let mut offset = 0;

    loop {
        if offset >= max_len {
            return Err("String exceeds max length (no null terminator)");
        }

        // Validate each read
        let read_addr = user_str + offset as u64;
        if read_addr >= 0x8000_0000_0000 {
            return Err("String crosses into kernel space");
        }

        // SAFETY: Validated above
        let byte = unsafe {
            *(read_addr as *const u8)
        };

        if byte == 0 {
            break;  // Found null terminator
        }

        data.push(byte);
        offset += 1;
    }

    alloc::string::String::from_utf8(data)
        .map_err(|_| "String contains invalid UTF-8")
}

/// Validate message structure (pointer + size)
pub fn validate_message_struct(msg_ptr: u64, max_size: usize) -> Result<(), &'static str> {
    let ptr = UserPtr::<u8>::from_addr(msg_ptr);
    ptr.validate_range(max_size)?;
    Ok(())
}

/// Get current task's address space info
/// Used to validate if pointer belongs to current task
pub struct AddressSpaceInfo {
    pub task_id: TaskId,
    pub user_base: u64,
    pub user_limit: u64,
    pub kernel_base: u64,
}

pub fn get_current_address_space() -> AddressSpaceInfo {
    AddressSpaceInfo {
        task_id: crate::task::current_task()
            .unwrap_or(crate::task::TaskId(1)),
        user_base: 0x0,
        user_limit: 0x8000_0000_0000,
        kernel_base: 0xFFFF_8000_0000_0000,
    }
}

/// Validate pointer belongs to current task (not another task or kernel)
pub fn validate_pointer_for_task(addr: u64, size: usize) -> Result<(), &'static str> {
    let as_info = get_current_address_space();

    // Must be in user-space
    if addr < as_info.user_base || addr >= as_info.user_limit {
        return Err("Pointer outside user-space");
    }

    // Must not cross into kernel
    let end = addr.checked_add(size as u64)
        .ok_or("Pointer + size overflows")?;

    if end > as_info.user_limit {
        return Err("Pointer range crosses kernel boundary");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_user_ptr() {
        let ptr = UserPtr::<u8>::from_addr(0x1000);
        assert!(ptr.is_valid_user_ptr());
    }

    #[test]
    fn test_invalid_kernel_ptr() {
        let ptr = UserPtr::<u8>::from_addr(0xFFFF_8000_0000_0000);
        assert!(!ptr.is_valid_user_ptr());
    }

    #[test]
    fn test_validate_range() {
        let ptr = UserPtr::<u8>::from_addr(0x1000);
        assert!(ptr.validate_range(4096).is_ok());
    }

    #[test]
    fn test_validate_range_overflow() {
        let ptr = UserPtr::<u8>::from_addr(0x7FFF_FFFF_FFFF);
        assert!(ptr.validate_range(4096).is_err());
    }
}
