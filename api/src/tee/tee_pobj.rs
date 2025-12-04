use core::default;

#[repr(C)]
pub  struct tee_pobj {
	obj_id_len: u32,
}

impl default::Default for tee_pobj {
	fn default() -> Self {
		tee_pobj {
			obj_id_len: 0,
		}
	}
}
#[cfg(feature = "tee_test")]
pub mod tests_tee_pobj {
    //-------- test framework import --------
	use crate::tee::TestDescriptor;
    use crate::test_fn;
    use crate::{assert, assert_eq, assert_ne, tests, tests_name};
	use crate::tee::TestResult;

    //-------- local tests import --------
    use super::*;

    test_fn! {
        using TestResult;

		fn test_tee_pobj_default() {
			let pobj = tee_pobj::default();
			assert_eq!(pobj.obj_id_len, 0);
		}
    }

    tests_name! {
        TEST_TEE_POBJ;
        //------------------------
		test_tee_pobj_default,
	}
}
