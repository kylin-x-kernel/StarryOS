/// Trait for PmuIf

/// PMU counter overflow callback.
///
/// Called in interrupt context.
pub type OverflowHandler = fn();

#[def_plat_interface]
pub trait PmuIf{
    /// Pmu interrupt handle func
    fn handle_overflows() -> bool;

    /// Register an overflow handler for a PMU counter.
    fn register_overflow_handler(index: u32, handler: OverflowHandler) -> bool;
}