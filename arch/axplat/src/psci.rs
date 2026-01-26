//! Psci interface.

/// Power state coordination interface
#[def_plat_interface]
pub trait PsciIf {
    /// Tell host share dma buf between guest and host
    fn share_dma_buffer(phys_addr: usize, size: usize);

    /// Tell host unshare dma buf between guest and host
    fn unshare_dma_buffer(phys_addr: usize, size: usize);
}
