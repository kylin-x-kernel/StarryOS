
/// 位图类型别名
pub type BitStr = u8;

/// 计算位所在的字节索引
#[inline]
fn bit_byte(bit: usize) -> usize {
    bit >> 3
}

/// 计算位在字节内的掩码
#[inline]
fn bit_mask(bit: usize) -> u8 {
    1 << (bit & 0x7)
}

/// 计算 nbits 位所需的字节数
#[inline]
pub fn bitstr_size(nbits: usize) -> usize {
    (nbits + 7) >> 3
}

#[inline]
pub fn bit_test(name: &[u8], bit: usize) -> bool {
    (name[bit_byte(bit)] & bit_mask(bit)) != 0
}

#[inline]
pub fn bit_set(name: &mut [u8], bit: usize) {
    name[bit_byte(bit)] |= bit_mask(bit);
}

#[inline]
pub fn bit_clear(name: &mut [u8], bit: usize) {
    name[bit_byte(bit)] &= !bit_mask(bit);
}

pub fn bit_nclear(name: &mut [u8], start: usize, stop: usize) {
    let start_byte = bit_byte(start);
    let stop_byte = bit_byte(stop);
    if start_byte == stop_byte {
        name[start_byte] &= (0xff >> (8 - (start & 0x7))) | (0xff << ((stop & 0x7) + 1));
    } else {
        name[start_byte] &= 0xff >> (8 - (start & 0x7));

        for i in (start_byte + 1)..stop_byte {
            name[i] = 0;
        }

        name[stop_byte] &= 0xff << ((stop & 0x7) + 1);
    }
}

pub fn bit_ffc(name: &[u8], nbits: usize, value: &mut isize) {
    let stop_byte = bit_byte(nbits - 1);
    let mut val: isize = -1;
    if nbits > 0 {
        for byte_index in 0..=stop_byte {
            if name[byte_index] != 0xff {
                let mut lb = name[byte_index];
                val = (byte_index << 3) as isize;
                while (lb & 0x1) != 0 {
                    val += 1;
                    lb >>= 1;
                }
                break;
            }
        }
    }
    if val as usize >= nbits {
        val = -1;
    }
    *value = val;
}