use core::mem::size_of;

use mbedtls::pk::Pk;
use mbedtls_sys_auto::{
    ERR_MPI_BAD_INPUT_DATA, ERR_PK_TYPE_MISMATCH, ERR_RSA_BAD_INPUT_DATA, ERR_RSA_INVALID_PADDING,
    ERR_RSA_OUTPUT_TOO_LARGE, ERR_RSA_PRIVATE_FAILED, mpi_bitlen, mpi_write_binary, rsa_context,
};
use tee_raw_sys::{TEE_ERROR_BAD_PARAMETERS, TEE_ERROR_BAD_STATE, TEE_ERROR_SHORT_BUFFER};

use crate::tee::{
    TeeResult, crypto::crypto::rsa_keypair, libmbedtls::bignum::crypto_bignum_copy_from_mpi,
    libutee::utee_defines::tee_u32_from_big_endian, rng_software::TeeSoftwareRng,
};

pub fn get_tee_result(lmd_res: i32) -> TeeResult {
    const ERR_RSA_PRIVATE_FAILED_BAD_INPUT: i32 = ERR_RSA_PRIVATE_FAILED + ERR_MPI_BAD_INPUT_DATA;

    match lmd_res {
        0 => Ok(()),
        ERR_RSA_PRIVATE_FAILED_BAD_INPUT
        | ERR_RSA_BAD_INPUT_DATA
        | ERR_RSA_INVALID_PADDING
        | ERR_PK_TYPE_MISMATCH => Err(TEE_ERROR_BAD_PARAMETERS),
        ERR_RSA_OUTPUT_TOO_LARGE => Err(TEE_ERROR_SHORT_BUFFER),
        _ => Err(TEE_ERROR_BAD_STATE),
    }
}

pub fn crypto_acipher_gen_rsa_key(key: &mut rsa_keypair, key_size: usize) -> TeeResult {
    let mut e: u32 = 0;

    // get the public exponent
    unsafe {
        mpi_write_binary(
            key.e.as_mpi().into(),
            &mut e as *mut u32 as *mut u8,
            size_of::<u32>(),
        );
    }
    e = tee_u32_from_big_endian(e);
    tee_debug!("crypto_acipher_gen_rsa_key get e: {:?}", e);

    let mut rng = TeeSoftwareRng::new();
    let pk = Pk::generate_rsa(&mut rng, key_size as u32, e);

    match pk {
        Ok(pk) => {
            let ctx = pk.inner().pk_ctx as *mut rsa_context;
            unsafe {
                if mpi_bitlen(&(*ctx).N) != key_size {
                    return Err(TEE_ERROR_BAD_PARAMETERS);
                }
            }

            // copy the key
            unsafe {
                crypto_bignum_copy_from_mpi(&mut key.e, &(*ctx).E);
                crypto_bignum_copy_from_mpi(&mut key.d, &(*ctx).D);
                crypto_bignum_copy_from_mpi(&mut key.n, &(*ctx).N);
                crypto_bignum_copy_from_mpi(&mut key.p, &(*ctx).P);
                crypto_bignum_copy_from_mpi(&mut key.q, &(*ctx).Q);

                crypto_bignum_copy_from_mpi(&mut key.qp, &(*ctx).QP);
                crypto_bignum_copy_from_mpi(&mut key.dp, &(*ctx).DP);
                crypto_bignum_copy_from_mpi(&mut key.dq, &(*ctx).DQ);
            }

            return Ok(());
        }
        Err(e) => {
            let e = e.to_int();
            return get_tee_result(e);
        }
    }

    Ok(())
}
