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
            cfg_if::cfg_if! {
            if #[cfg(feature = "tee_test_mock_user_access")] {
                let user_data: [u8; 5] = [1, 2, 3, 4, 5];
                let mut kernel_data: [u8; 5] = [0; 5];

                copy_from_user(&mut kernel_data, &user_data, 5).unwrap();
                assert_eq!(kernel_data, user_data);
                }
            }
        }
    }

    test_fn! {
        using TestResult;

        fn test_copy_to_user() {
            cfg_if::cfg_if! {
            if #[cfg(feature = "tee_test_mock_user_access")] {
                let kernel_data: [u8; 5] = [10, 20, 30, 40, 50];
                let mut user_data: [u8; 5] = [0; 5];

                copy_to_user(&mut user_data, &kernel_data, 5).unwrap();
                assert_eq!(user_data, kernel_data);
                }
            }
        }
    }

    tests_name! {
        TEST_USER_ACCESS;
        //------------------------
        test_copy_from_user,
        test_copy_to_user,
    }
}
