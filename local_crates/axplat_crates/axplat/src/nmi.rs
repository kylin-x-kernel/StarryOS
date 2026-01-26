
/// Platform NMI capability.
#[derive(Clone, Copy, Debug)]
pub enum NmiType {
    /// True hardware NMI (cannot be masked by IRQ disable)
    TrueNmi,
    /// Pseudo NMI (implemented via high-priority IRQ / FIQ / SGI)
    PseudoNmi,
    /// Not supported
    None,
}

pub type NmiHandler = fn();

/// Trait for NMI sources.
///
/// Implementors provide a mechanism to trigger NMI-like interrupts
/// at regular intervals for watchdog purposes.
#[def_plat_interface]
pub trait NmiIf{
    /// Initialize the NMI source.
    ///
    /// This should configure the hardware but not start triggering.
    fn init(threshold: u64) -> bool;

    /// Returns the NMI capability of this platform.
    fn nmi_type() -> NmiType;

    /// Enable NMI generation.
    ///
    /// After this call, NMIs will be triggered at the configured period.
    fn enable();

    /// Disable NMI generation.
    fn disable();

    /// Check if NMI generation is currently enabled.
    fn is_enabled() -> bool;

    /// Get the name of this NMI source (for debugging).
    fn name() -> &'static str;

    /// Nmi handle func
    fn register_nmi_handler(handler:NmiHandler) -> bool;
}