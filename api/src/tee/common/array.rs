use core::ptr;

pub fn get_const_ptr_and_len<T>(opt_slice: Option<T>) -> (*const u8, usize)
where
    T: AsRef<[u8]>, // 限制 T 必须是可以提供 &[u8] 引用的类型
{
    match opt_slice {
        Some(s) => (s.as_ref().as_ptr(), s.as_ref().len()),
        None => (ptr::null(), 0),
    }
}

pub fn get_mut_ptr_and_len<T>(opt_slice: Option<T>) -> (*mut u8, usize)
where
    T: AsMut<[u8]>, // 限制 T 必须是可以提供 &mut [u8] 引用的类型
{
    match opt_slice {
        Some(mut s) => (s.as_mut().as_mut_ptr(), s.as_mut().len()),
        None => (ptr::null_mut(), 0), // 注意这里使用 ptr::null_mut()
    }
}