#[inline]
pub fn local_irq_save_and_disable() -> usize {
    crate_interface::call_interface!(crate::KernelGuardIf::local_irq_save_and_disable)
}

#[inline]
pub fn local_irq_restore(flags: usize) {
    crate_interface::call_interface!(crate::KernelGuardIf::local_irq_restore(flags))
}
