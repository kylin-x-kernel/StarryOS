// =============================================================================
// PMU-based NMI Source
// =============================================================================

/// Default implementation of [`axplat::nmi::NmiIf`] using PMU
#[macro_export]
macro_rules! nmi_if_impl {
    ($name:ident) => {
        struct $name;

        use axplat::nmi::{NmiType, NmiHandler};

        const CYCLE_COUNTER_INDEX: u32 = 31;

        #[impl_plat_interface]
        impl axplat::nmi::NmiIf for $name {
            fn init(threshold: u64) -> bool {
                // Register interrupt handler on primary core only
                $crate::gic::set_priority(crate::config::devices::PMU_IRQ, 0);
                $crate::pmu::init_cycle_counter(threshold)
            }

            fn enable() {
                $crate::pmu::enable(CYCLE_COUNTER_INDEX);
            }

            fn disable() {
                $crate::pmu::disable(CYCLE_COUNTER_INDEX);
            }

            fn is_enabled() -> bool {
                $crate::pmu::is_enabled(CYCLE_COUNTER_INDEX)
            }

            fn name() -> &'static str {
                "PMU"
            }

            fn nmi_type() -> NmiType {
                NmiType::PseudoNmi
            }

            fn register_nmi_handler(handler:NmiHandler) -> bool{
                $crate::pmu::register_overflow_handler(CYCLE_COUNTER_INDEX, handler)
            }
        }
    };
}
