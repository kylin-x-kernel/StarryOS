use core::default;
use slab::Slab;
use axerrno::{AxError, AxResult};
use alloc::{
    sync::{Arc},
};

use tee_raw_sys::libc_compat::size_t;
use flatten_objects::FlattenObjects;
use spin::RwLock;

use super::{
    tee_fs::tee_file_handle,
	tee_pobj::tee_pobj,
};

pub type tee_obj_id_type = usize;
/// The maximum number of open files
pub const AX_TEE_OBJ_LIMIT: usize = 1024;

scope_local::scope_local! {
    /// The open objects for TA.
    pub static TEE_OBJ_TABLE: Arc<RwLock<Slab<Arc<tee_obj>>>> = Arc::default();
}

#[repr(C)]
pub struct tee_obj {
	// TEE_ObjectInfo info;
	busy: bool,		/* true if used by an operation */
	have_attrs: u32,	/* bitfield identifying set properties */
	//void *attr;
	ds_pos: size_t,
	pobj: Arc<tee_pobj>,
	fh: Arc<tee_file_handle>,
}

impl default::Default for tee_obj {
	fn default() -> Self {
		tee_obj {
			busy: false,
			have_attrs: 0,
			ds_pos: 0,
			pobj: Arc::new(tee_pobj::default()),
			fh: Arc::new(tee_file_handle {}),
		}
	}
}

pub fn tee_obj_add(obj: tee_obj)-> AxResult<tee_obj_id_type> 
{
	let mut table = TEE_OBJ_TABLE.write();
	Ok(table.insert(Arc::new(obj)) as tee_obj_id_type)
}

pub fn tee_obj_get(obj_id : tee_obj_id_type) -> AxResult<Arc<tee_obj>>
{
		let mut table = TEE_OBJ_TABLE.read();
		if let Some(obj) = table.get(obj_id as usize) {
			Ok(obj.clone())
		} else {
			Err(AxError::InvalidInput)
		}
}

