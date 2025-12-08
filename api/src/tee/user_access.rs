use axerrno::{AxError, AxResult};
use cfg_if::cfg_if;
use core::mem::{MaybeUninit, transmute};
use starry_vm::{VmError, VmPtr, vm_read_slice, vm_write_slice};
use tee_raw_sys::libc_compat::size_t;
use tee_raw_sys::*;

use super::TeeResult;

pub(crate) fn copy_from_user(kaddr: &mut [u8], uaddr: &[u8], len: size_t) -> TeeResult {
    cfg_if::cfg_if! {
        if #[cfg(feature = "tee_test_mock_user_access")] {
            kaddr[..len].copy_from_slice(&uaddr[..len]);
            Ok(())
        } else {
            vm_read_slice(uaddr.as_ptr(), unsafe {
                transmute::<&mut [u8], &mut [MaybeUninit<u8>]>(&mut kaddr[..len])
            })
            .map_err(|error| match error {
                VmError::BadAddress => TEE_ERROR_BAD_PARAMETERS,
                _ => TEE_ERROR_GENERIC,
            })?;
            Ok(())
        }
    }
}

pub fn copy_from_user_u64(s: &mut u64, user_s: &u64) -> TeeResult {
    let s_bytes: &mut [u8] = unsafe {
        core::slice::from_raw_parts_mut(s as *mut u64 as *mut u8, core::mem::size_of::<u64>())
    };
    let size_bytes: &[u8] = unsafe {
        core::slice::from_raw_parts(
            user_s as *const u64 as *const u8,
            core::mem::size_of::<u64>(),
        )
    };
    copy_from_user(s_bytes, size_bytes, core::mem::size_of::<u64>())?;

    Ok(())
}

pub fn copy_to_user_u64(user_s: &mut u64, s: &u64) -> TeeResult {
    let s_bytes: &[u8] = unsafe {
        core::slice::from_raw_parts(s as *const u64 as *const u8, core::mem::size_of::<u64>())
    };
    let size_bytes: &mut [u8] = unsafe {
        core::slice::from_raw_parts_mut(
            user_s as *mut u64 as *mut u8,
            core::mem::size_of::<u64>(),
        )
    };
    copy_to_user(size_bytes, s_bytes, core::mem::size_of::<u64>())?;

    Ok(())
}


pub(crate) fn copy_to_user(uaddr: &mut [u8], kaddr: &[u8], len: size_t) -> TeeResult {
    cfg_if::cfg_if! {
        if #[cfg(feature = "tee_test_mock_user_access")] {
            uaddr[..len].copy_from_slice(&kaddr[..len]);
            Ok(())
        } else {
            vm_read_slice(uaddr.as_ptr(), unsafe {
                transmute::<&mut [u8], &mut [MaybeUninit<u8>]>(&mut kaddr[..len])
            })
            .map_err(|error| match error {
                VmError::BadAddress => TEE_ERROR_BAD_PARAMETERS,
                _ => TEE_ERROR_GENERIC,
            })?;
            Ok(())
        }
    }
}

#[cfg(feature = "tee_test")]
pub mod tests_user_access {
    //-------- test framework import --------
    use crate::tee::TestDescriptor;
    use crate::tee::TestResult;
    use crate::test_fn;
    use crate::{assert, assert_eq, assert_ne, tests, tests_name};

    //-------- local tests import --------
    use super::*;

    test_fn! {
        using TestResult;

        fn test_copy_from_user() {
            let user_data: [u8; 5] = [1, 2, 3, 4, 5];
            let mut kernel_data: [u8; 5] = [0; 5];

            copy_from_user(&mut kernel_data, &user_data, 5).unwrap();
            assert_eq!(kernel_data, user_data);
        }
    }

    test_fn! {
        using TestResult;

        fn test_copy_to_user() {
            let kernel_data: [u8; 5] = [10, 20, 30, 40, 50];
            let mut user_data: [u8; 5] = [0; 5];

            copy_to_user(&mut user_data, &kernel_data, 5).unwrap();
            assert_eq!(user_data, kernel_data);
        }
    }

    test_fn! {
        using TestResult;

        fn test_copy_from_user_u64() {
            let user_value: u64 = 0x1122334455667788;
            let mut kernel_value: u64 = 0;

            copy_from_user_u64(&mut kernel_value, &user_value).unwrap();
            assert_eq!(kernel_value, user_value);

            // test copy_to_user_u64
            let mut user_value_out: u64 = 1;
            copy_to_user_u64(&mut user_value_out, &kernel_value).unwrap();
            assert_eq!(user_value_out, kernel_value);
        }
    }

    tests_name! {
        TEST_USER_ACCESS;
        //------------------------
        test_copy_from_user,
        test_copy_to_user,
        test_copy_from_user_u64,
    }
}
