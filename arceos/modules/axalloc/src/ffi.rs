use core::{
    alloc::Layout,
    ffi::{c_void, c_int},
    ptr::{self, NonNull},
};
use crate::global_allocator;

// malloc - 分配内存并存储大小元数据
#[unsafe(no_mangle)]
pub unsafe extern "C" fn malloc(size: c_int) -> *mut c_void {
    if size <= 0 {
        return ptr::null_mut();
    }

    let user_size = size as usize;
    // 元数据大小（例如存储一个 usize）
    let metadata_size = size_of::<usize>();
    // 总分配大小：用户请求大小 + 元数据大小
    let total_size = user_size + metadata_size;

    // 创建布局，对齐方式与元数据对齐（此处简化为元数据对齐，用户对齐需求需额外处理）
    let layout = match Layout::from_size_align(total_size, size_of::<usize>()) {
        Ok(layout) => layout,
        Err(_) => return ptr::null_mut(),
    };

    let ptr = global_allocator().alloc(layout);
    match ptr {
        Ok(ptr) => {
            // 在指针开头存储用户请求的大小
            *(ptr.as_ptr() as *mut usize) = user_size;
            // 返回元数据之后的地址（用户可用空间）
            ptr.as_ptr().add(metadata_size) as *mut c_void
        },
        Err(_) => ptr::null_mut(),
    }
}

// free - 通过元数据获取布局信息后释放
#[unsafe(no_mangle)]
pub unsafe extern "C" fn free(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }

    let metadata_size = size_of::<usize>();
    // 计算原始分配指针（向前偏移元数据大小）
    let base_ptr = (ptr as *mut u8).sub(metadata_size);
    // 读取存储的用户请求大小
    let user_size = *(base_ptr as *const usize);

    // 构建释放用的布局（总大小需包含元数据，对齐与分配时一致）
    let total_size = user_size + metadata_size;
    let layout = Layout::from_size_align_unchecked(total_size, size_of::<usize>());

    global_allocator().dealloc(NonNull::new_unchecked(base_ptr), layout);
}

// calloc - 分配并清零指定数量和大小的内存
#[unsafe(no_mangle)]
pub unsafe extern "C" fn calloc(nmemb: c_int, size: c_int) -> *mut c_void {
    let total_size = nmemb.saturating_mul(size);
    if total_size == 0 {
        return ptr::null_mut();
    }
    
    let ptr = malloc(total_size);
    if !ptr.is_null() {
        ptr::write_bytes(ptr as *mut u8, 0, total_size as usize);
    }
    ptr
}

// __memcpy_chk - 带边界检查的内存拷贝
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __memcpy_chk(
    dest: *mut c_void,
    src: *const c_void,
    len: c_int,
    dest_len: c_int,
) -> *mut c_void {
    if dest.is_null() || src.is_null() {
        return dest;
    }
    
    if len > dest_len {
        return ptr::null_mut();
    }
    
    ptr::copy_nonoverlapping(src as *const u8, dest as *mut u8, len as usize);
    dest
}