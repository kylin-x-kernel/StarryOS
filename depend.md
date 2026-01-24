axplat-aarch64-crosvm-virt v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/local_crates/axplat-aarch64-crosvm-virt) - https://github.com/kylin-x-kernel/axplat-aarch64-crosvm-virt.git
├── aarch64-cpu feature "default"
│   └── aarch64-cpu v10.0.0 - https://github.com/rust-embedded/aarch64-cpu
│       └── tock-registers v0.9.0 - https://github.com/tock/tock/tree/master/libraries/tock-register-interface
├── arm-gic feature "default"
│   └── arm-gic v0.1.0 (https://github.com/kylin-x-kernel/arm-gic.git?branch=main#35bfb52d) - https://github.com/kylin-x-kernel/arm-gic.git
│       └── bitflags feature "default"
│           └── bitflags v2.10.0 - https://github.com/bitflags/bitflags
├── axconfig-macros feature "default"
│   └── axconfig-macros v0.2.0 (proc-macro) - https://github.com/arceos-org/axconfig_crates
│       ├── axconfig-gen feature "default"
│       │   └── axconfig-gen v0.2.0 - https://github.com/arceos-org/axconfig_crates
│       │       ├── clap feature "default"
│       │       │   ├── clap v4.5.54 - https://github.com/clap-rs/clap
│       │       │   │   ├── clap_builder v4.5.54 - https://github.com/clap-rs/clap
│       │       │   │   │   ├── anstream feature "default"
│       │       │   │   │   │   ├── anstream v0.6.21 - https://github.com/rust-cli/anstyle.git
│       │       │   │   │   │   │   ├── anstyle feature "default"
│       │       │   │   │   │   │   │   ├── anstyle v1.0.13 - https://github.com/rust-cli/anstyle.git
│       │       │   │   │   │   │   │   └── anstyle feature "std"
│       │       │   │   │   │   │   │       └── anstyle v1.0.13 - https://github.com/rust-cli/anstyle.git
│       │       │   │   │   │   │   ├── anstyle-parse feature "default"
│       │       │   │   │   │   │   │   ├── anstyle-parse v0.2.7 - https://github.com/rust-cli/anstyle.git
│       │       │   │   │   │   │   │   │   └── utf8parse feature "default"
│       │       │   │   │   │   │   │   │       └── utf8parse v0.2.2 - https://github.com/alacritty/vte
│       │       │   │   │   │   │   │   └── anstyle-parse feature "utf8"
│       │       │   │   │   │   │   │       └── anstyle-parse v0.2.7 - https://github.com/rust-cli/anstyle.git (*)
│       │       │   │   │   │   │   ├── utf8parse feature "default" (*)
│       │       │   │   │   │   │   ├── anstyle-query feature "default"
│       │       │   │   │   │   │   │   └── anstyle-query v1.1.5 - https://github.com/rust-cli/anstyle.git
│       │       │   │   │   │   │   ├── colorchoice feature "default"
│       │       │   │   │   │   │   │   └── colorchoice v1.0.4 - https://github.com/rust-cli/anstyle.git
│       │       │   │   │   │   │   └── is_terminal_polyfill feature "default"
│       │       │   │   │   │   │       └── is_terminal_polyfill v1.70.2 - https://github.com/polyfill-rs/is_terminal_polyfill
│       │       │   │   │   │   ├── anstream feature "auto"
│       │       │   │   │   │   │   └── anstream v0.6.21 - https://github.com/rust-cli/anstyle.git (*)
│       │       │   │   │   │   └── anstream feature "wincon"
│       │       │   │   │   │       └── anstream v0.6.21 - https://github.com/rust-cli/anstyle.git (*)
│       │       │   │   │   ├── anstyle feature "default" (*)
│       │       │   │   │   ├── clap_lex feature "default"
│       │       │   │   │   │   └── clap_lex v0.7.7 - https://github.com/clap-rs/clap
│       │       │   │   │   └── strsim feature "default"
│       │       │   │   │       └── strsim v0.11.1 - https://github.com/rapidfuzz/strsim-rs
│       │       │   │   └── clap_derive feature "default"
│       │       │   │       └── clap_derive v4.5.49 (proc-macro) - https://github.com/clap-rs/clap
│       │       │   │           ├── heck feature "default"
│       │       │   │           │   └── heck v0.5.0 - https://github.com/withoutboats/heck
│       │       │   │           ├── proc-macro2 feature "default"
│       │       │   │           │   ├── proc-macro2 v1.0.105 - https://github.com/dtolnay/proc-macro2
│       │       │   │           │   │   └── unicode-ident feature "default"
│       │       │   │           │   │       └── unicode-ident v1.0.22 - https://github.com/dtolnay/unicode-ident
│       │       │   │           │   └── proc-macro2 feature "proc-macro"
│       │       │   │           │       └── proc-macro2 v1.0.105 - https://github.com/dtolnay/proc-macro2 (*)
│       │       │   │           ├── quote feature "default"
│       │       │   │           │   ├── quote v1.0.43 - https://github.com/dtolnay/quote
│       │       │   │           │   │   └── proc-macro2 v1.0.105 - https://github.com/dtolnay/proc-macro2 (*)
│       │       │   │           │   └── quote feature "proc-macro"
│       │       │   │           │       ├── quote v1.0.43 - https://github.com/dtolnay/quote (*)
│       │       │   │           │       └── proc-macro2 feature "proc-macro" (*)
│       │       │   │           ├── syn feature "default"
│       │       │   │           │   ├── syn v2.0.114 - https://github.com/dtolnay/syn
│       │       │   │           │   │   ├── proc-macro2 v1.0.105 - https://github.com/dtolnay/proc-macro2 (*)
│       │       │   │           │   │   ├── quote v1.0.43 - https://github.com/dtolnay/quote (*)
│       │       │   │           │   │   └── unicode-ident feature "default" (*)
│       │       │   │           │   ├── syn feature "clone-impls"
│       │       │   │           │   │   └── syn v2.0.114 - https://github.com/dtolnay/syn (*)
│       │       │   │           │   ├── syn feature "derive"
│       │       │   │           │   │   └── syn v2.0.114 - https://github.com/dtolnay/syn (*)
│       │       │   │           │   ├── syn feature "parsing"
│       │       │   │           │   │   └── syn v2.0.114 - https://github.com/dtolnay/syn (*)
│       │       │   │           │   ├── syn feature "printing"
│       │       │   │           │   │   └── syn v2.0.114 - https://github.com/dtolnay/syn (*)
│       │       │   │           │   └── syn feature "proc-macro"
│       │       │   │           │       ├── syn v2.0.114 - https://github.com/dtolnay/syn (*)
│       │       │   │           │       ├── proc-macro2 feature "proc-macro" (*)
│       │       │   │           │       └── quote feature "proc-macro" (*)
│       │       │   │           └── syn feature "full"
│       │       │   │               └── syn v2.0.114 - https://github.com/dtolnay/syn (*)
│       │       │   ├── clap feature "color"
│       │       │   │   ├── clap v4.5.54 - https://github.com/clap-rs/clap (*)
│       │       │   │   └── clap_builder feature "color"
│       │       │   │       └── clap_builder v4.5.54 - https://github.com/clap-rs/clap (*)
│       │       │   ├── clap feature "error-context"
│       │       │   │   ├── clap v4.5.54 - https://github.com/clap-rs/clap (*)
│       │       │   │   └── clap_builder feature "error-context"
│       │       │   │       └── clap_builder v4.5.54 - https://github.com/clap-rs/clap (*)
│       │       │   ├── clap feature "help"
│       │       │   │   ├── clap v4.5.54 - https://github.com/clap-rs/clap (*)
│       │       │   │   └── clap_builder feature "help"
│       │       │   │       └── clap_builder v4.5.54 - https://github.com/clap-rs/clap (*)
│       │       │   ├── clap feature "std"
│       │       │   │   ├── clap v4.5.54 - https://github.com/clap-rs/clap (*)
│       │       │   │   └── clap_builder feature "std"
│       │       │   │       ├── clap_builder v4.5.54 - https://github.com/clap-rs/clap (*)
│       │       │   │       └── anstyle feature "std" (*)
│       │       │   ├── clap feature "suggestions"
│       │       │   │   ├── clap v4.5.54 - https://github.com/clap-rs/clap (*)
│       │       │   │   └── clap_builder feature "suggestions"
│       │       │   │       ├── clap_builder v4.5.54 - https://github.com/clap-rs/clap (*)
│       │       │   │       └── clap_builder feature "error-context" (*)
│       │       │   └── clap feature "usage"
│       │       │       ├── clap v4.5.54 - https://github.com/clap-rs/clap (*)
│       │       │       └── clap_builder feature "usage"
│       │       │           └── clap_builder v4.5.54 - https://github.com/clap-rs/clap (*)
│       │       ├── clap feature "derive"
│       │       │   └── clap v4.5.54 - https://github.com/clap-rs/clap (*)
│       │       └── toml_edit feature "default"
│       │           ├── toml_edit v0.22.27 - https://github.com/toml-rs/toml
│       │           │   ├── indexmap feature "default"
│       │           │   │   ├── indexmap v2.13.0 - https://github.com/indexmap-rs/indexmap
│       │           │   │   │   ├── equivalent v1.0.2 - https://github.com/indexmap-rs/equivalent
│       │           │   │   │   └── hashbrown v0.16.1 - https://github.com/rust-lang/hashbrown
│       │           │   │   └── indexmap feature "std"
│       │           │   │       └── indexmap v2.13.0 - https://github.com/indexmap-rs/indexmap (*)
│       │           │   ├── indexmap feature "std" (*)
│       │           │   ├── toml_datetime feature "default"
│       │           │   │   └── toml_datetime v0.6.11 - https://github.com/toml-rs/toml
│       │           │   ├── toml_write feature "default"
│       │           │   │   ├── toml_write v0.1.2 - https://github.com/toml-rs/toml
│       │           │   │   └── toml_write feature "std"
│       │           │   │       ├── toml_write v0.1.2 - https://github.com/toml-rs/toml
│       │           │   │       └── toml_write feature "alloc"
│       │           │   │           └── toml_write v0.1.2 - https://github.com/toml-rs/toml
│       │           │   └── winnow feature "default"
│       │           │       ├── winnow v0.7.14 - https://github.com/winnow-rs/winnow
│       │           │       └── winnow feature "std"
│       │           │           ├── winnow v0.7.14 - https://github.com/winnow-rs/winnow
│       │           │           └── winnow feature "alloc"
│       │           │               └── winnow v0.7.14 - https://github.com/winnow-rs/winnow
│       │           ├── toml_edit feature "display"
│       │           │   └── toml_edit v0.22.27 - https://github.com/toml-rs/toml (*)
│       │           └── toml_edit feature "parse"
│       │               └── toml_edit v0.22.27 - https://github.com/toml-rs/toml (*)
│       ├── proc-macro2 feature "default" (*)
│       ├── quote feature "default" (*)
│       ├── syn feature "default" (*)
│       └── syn feature "full" (*)
├── axcpu feature "default"
│   └── axcpu v0.3.0 (https://github.com/kylin-x-kernel/axcpu?branch=dev#df212325) - https://github.com/arceos-org/axcpu
│       ├── aarch64-cpu feature "default" (*)
│       ├── tock-registers feature "default"
│       │   ├── tock-registers v0.9.0 - https://github.com/tock/tock/tree/master/libraries/tock-register-interface
│       │   └── tock-registers feature "register_types"
│       │       └── tock-registers v0.9.0 - https://github.com/tock/tock/tree/master/libraries/tock-register-interface
│       ├── axbacktrace feature "default"
│       │   └── axbacktrace v0.1.1 - https://github.com/Starry-OS/axbacktrace
│       │       ├── addr2line feature "cpp_demangle"
│       │       │   └── addr2line v0.25.1 - https://github.com/gimli-rs/addr2line
│       │       │       ├── cpp_demangle feature "alloc"
│       │       │       │   └── cpp_demangle v0.4.5 - https://github.com/gimli-rs/cpp_demangle
│       │       │       │       └── cfg-if feature "default"
│       │       │       │           └── cfg-if v1.0.4 - https://github.com/rust-lang/cfg-if
│       │       │       ├── gimli feature "read"
│       │       │       │   ├── gimli v0.32.3 - https://github.com/gimli-rs/gimli
│       │       │       │   │   └── stable_deref_trait v1.2.1 - https://github.com/storyyeller/stable_deref_trait
│       │       │       │   └── gimli feature "read-core"
│       │       │       │       └── gimli v0.32.3 - https://github.com/gimli-rs/gimli (*)
│       │       │       └── rustc-demangle feature "default"
│       │       │           └── rustc-demangle v0.1.26 - https://github.com/rust-lang/rustc-demangle
│       │       ├── addr2line feature "rustc-demangle"
│       │       │   └── addr2line v0.25.1 - https://github.com/gimli-rs/addr2line (*)
│       │       ├── cfg-if feature "default" (*)
│       │       ├── gimli feature "endian-reader"
│       │       │   ├── gimli v0.32.3 - https://github.com/gimli-rs/gimli (*)
│       │       │   └── gimli feature "read" (*)
│       │       ├── gimli feature "read-core" (*)
│       │       ├── log feature "default"
│       │       │   └── log v0.4.29 - https://github.com/rust-lang/log
│       │       ├── paste feature "default"
│       │       │   └── paste v1.0.15 (proc-macro) - https://github.com/dtolnay/paste
│       │       └── spin feature "once"
│       │           └── spin v0.10.0 - https://github.com/mvdnes/spin-rs.git
│       │               └── lock_api feature "default"
│       │                   ├── lock_api v0.4.14 - https://github.com/Amanieu/parking_lot
│       │                   │   └── scopeguard v1.2.0 - https://github.com/bluss/scopeguard
│       │                   └── lock_api feature "atomic_usize"
│       │                       └── lock_api v0.4.14 - https://github.com/Amanieu/parking_lot (*)
│       ├── cfg-if feature "default" (*)
│       ├── log feature "default" (*)
│       ├── axplat feature "default"
│       │   └── axplat v0.3.0 (https://github.com/kylin-x-kernel/axplat_crates.git?branch=dev#28d9b734) - https://github.com/arceos-org/axplat_crates
│       │       ├── bitflags feature "default" (*)
│       │       ├── axplat-macros feature "default"
│       │       │   └── axplat-macros v0.1.0 (proc-macro) (https://github.com/kylin-x-kernel/axplat_crates.git?branch=dev#28d9b734) - https://github.com/arceos-org/axplat_crates
│       │       │       ├── proc-macro2 feature "default" (*)
│       │       │       ├── quote feature "default" (*)
│       │       │       ├── syn feature "default" (*)
│       │       │       └── syn feature "full" (*)
│       │       ├── const-str feature "default"
│       │       │   └── const-str v0.6.4 - https://github.com/Nugine/const-str
│       │       ├── crate_interface feature "default"
│       │       │   └── crate_interface v0.1.4 (proc-macro) - https://github.com/arceos-org/crate_interface
│       │       │       ├── proc-macro2 feature "default" (*)
│       │       │       ├── quote feature "default" (*)
│       │       │       ├── syn feature "default" (*)
│       │       │       └── syn feature "full" (*)
│       │       ├── handler_table feature "default"
│       │       │   └── handler_table v0.1.2 - https://github.com/arceos-org/handler_table
│       │       ├── kspin feature "default"
│       │       │   └── kspin v0.1.1 - https://github.com/arceos-org/kspin
│       │       │       ├── cfg-if feature "default" (*)
│       │       │       └── kernel_guard feature "default"
│       │       │           └── kernel_guard v0.1.3 (https://github.com/kylin-x-kernel/kernel_guard?branch=main#58b0f7b2) - https://github.com/arceos-org/kernel_guard
│       │       │               ├── cfg-if feature "default" (*)
│       │       │               └── crate_interface feature "default" (*)
│       │       ├── memory_addr feature "default"
│       │       │   └── memory_addr v0.4.1 - https://github.com/arceos-org/axmm_crates
│       │       └── percpu feature "default"
│       │           └── percpu v0.2.0 (https://github.com/arceos-org/percpu?rev=89c8a54#89c8a54c) - https://github.com/arceos-org/percpu
│       │               ├── cfg-if feature "default" (*)
│       │               ├── kernel_guard feature "default" (*)
│       │               ├── percpu_macros feature "default"
│       │               │   └── percpu_macros v0.2.0 (proc-macro) (https://github.com/arceos-org/percpu?rev=89c8a54#89c8a54c) - https://github.com/arceos-org/percpu
│       │               │       ├── proc-macro2 feature "default" (*)
│       │               │       ├── quote feature "default" (*)
│       │               │       ├── syn feature "default" (*)
│       │               │       └── syn feature "full" (*)
│       │               └── spin feature "default"
│       │                   ├── spin v0.9.8 - https://github.com/mvdnes/spin-rs.git
│       │                   │   └── lock_api feature "default" (*)
│       │                   ├── spin feature "barrier"
│       │                   │   ├── spin v0.9.8 - https://github.com/mvdnes/spin-rs.git (*)
│       │                   │   └── spin feature "mutex"
│       │                   │       └── spin v0.9.8 - https://github.com/mvdnes/spin-rs.git (*)
│       │                   ├── spin feature "lazy"
│       │                   │   ├── spin v0.9.8 - https://github.com/mvdnes/spin-rs.git (*)
│       │                   │   └── spin feature "once"
│       │                   │       └── spin v0.9.8 - https://github.com/mvdnes/spin-rs.git (*)
│       │                   ├── spin feature "lock_api"
│       │                   │   ├── spin v0.9.8 - https://github.com/mvdnes/spin-rs.git (*)
│       │                   │   └── spin feature "lock_api_crate"
│       │                   │       └── spin v0.9.8 - https://github.com/mvdnes/spin-rs.git (*)
│       │                   ├── spin feature "mutex" (*)
│       │                   ├── spin feature "once" (*)
│       │                   ├── spin feature "rwlock"
│       │                   │   └── spin v0.9.8 - https://github.com/mvdnes/spin-rs.git (*)
│       │                   └── spin feature "spin_mutex"
│       │                       ├── spin v0.9.8 - https://github.com/mvdnes/spin-rs.git (*)
│       │                       └── spin feature "mutex" (*)
│       ├── axplat feature "irq"
│       │   └── axplat v0.3.0 (https://github.com/kylin-x-kernel/axplat_crates.git?branch=dev#28d9b734) - https://github.com/arceos-org/axplat_crates (*)
│       ├── memory_addr feature "default" (*)
│       ├── linkme feature "default"
│       │   └── linkme v0.3.35 - https://github.com/dtolnay/linkme
│       │       └── linkme-impl feature "default"
│       │           └── linkme-impl v0.3.35 (proc-macro) - https://github.com/dtolnay/linkme
│       │               ├── proc-macro2 feature "default" (*)
│       │               ├── quote feature "default" (*)
│       │               └── syn feature "default" (*)
│       ├── page_table_entry feature "default"
│       │   └── page_table_entry v0.5.7 - https://github.com/arceos-org/page_table_multiarch
│       │       ├── aarch64-cpu feature "default" (*)
│       │       ├── bitflags feature "default" (*)
│       │       └── memory_addr feature "default" (*)
│       └── static_assertions feature "default"
│           └── static_assertions v1.1.0 - https://github.com/nvzqz/static-assertions-rs
├── log feature "default" (*)
├── axplat feature "default" (*)
├── kspin feature "default" (*)
├── spin feature "default" (*)
├── page_table_entry feature "default" (*)
├── axplat-aarch64-peripherals feature "gicv3"
│   ├── axplat-aarch64-peripherals v0.3.0 (https://github.com/kylin-x-kernel/axplat_crates.git?branch=dev#28d9b734) - https://github.com/arceos-org/axplat_crates
│   │   ├── aarch64-cpu feature "default" (*)
│   │   ├── axcpu feature "default" (*)
│   │   ├── log feature "default" (*)
│   │   ├── spin feature "default"
│   │   │   ├── spin v0.10.0 - https://github.com/mvdnes/spin-rs.git (*)
│   │   │   ├── spin feature "barrier"
│   │   │   │   ├── spin v0.10.0 - https://github.com/mvdnes/spin-rs.git (*)
│   │   │   │   └── spin feature "mutex"
│   │   │   │       └── spin v0.10.0 - https://github.com/mvdnes/spin-rs.git (*)
│   │   │   ├── spin feature "lazy"
│   │   │   │   ├── spin v0.10.0 - https://github.com/mvdnes/spin-rs.git (*)
│   │   │   │   └── spin feature "once" (*)
│   │   │   ├── spin feature "lock_api"
│   │   │   │   └── spin v0.10.0 - https://github.com/mvdnes/spin-rs.git (*)
│   │   │   ├── spin feature "mutex" (*)
│   │   │   ├── spin feature "once" (*)
│   │   │   ├── spin feature "rwlock"
│   │   │   │   └── spin v0.10.0 - https://github.com/mvdnes/spin-rs.git (*)
│   │   │   └── spin feature "spin_mutex"
│   │   │       ├── spin v0.10.0 - https://github.com/mvdnes/spin-rs.git (*)
│   │   │       └── spin feature "mutex" (*)
│   │   ├── axplat feature "default" (*)
│   │   ├── kspin feature "default" (*)
│   │   ├── percpu feature "default" (*)
│   │   ├── page_table_entry feature "default" (*)
│   │   ├── arm-gic-driver feature "default"
│   │   │   └── arm-gic-driver v0.15.10 - https://github.com/rcore-os/arm-gic-driver
│   │   │       ├── aarch64-cpu feature "default" (*)
│   │   │       ├── tock-registers feature "default" (*)
│   │   │       ├── bitflags feature "default" (*)
│   │   │       ├── log feature "default" (*)
│   │   │       ├── paste feature "default" (*)
│   │   │       └── enum_dispatch feature "default"
│   │   │           └── enum_dispatch v0.3.13 (proc-macro) - https://gitlab.com/antonok/enum_dispatch
│   │   │               ├── proc-macro2 feature "default" (*)
│   │   │               ├── quote feature "default" (*)
│   │   │               ├── syn feature "default" (*)
│   │   │               ├── syn feature "full" (*)
│   │   │               └── once_cell feature "default"
│   │   │                   ├── once_cell v1.21.3 - https://github.com/matklad/once_cell
│   │   │                   └── once_cell feature "std"
│   │   │                       ├── once_cell v1.21.3 - https://github.com/matklad/once_cell
│   │   │                       └── once_cell feature "alloc"
│   │   │                           ├── once_cell v1.21.3 - https://github.com/matklad/once_cell
│   │   │                           └── once_cell feature "race"
│   │   │                               └── once_cell v1.21.3 - https://github.com/matklad/once_cell
│   │   ├── arm_pl011 feature "default"
│   │   │   └── arm_pl011 v0.1.0 - https://github.com/arceos-org/arm_pl011
│   │   │       └── tock-registers feature "default"
│   │   │           ├── tock-registers v0.8.1 - https://github.com/tock/tock/tree/master/libraries/tock-register-interface
│   │   │           └── tock-registers feature "register_types"
│   │   │               └── tock-registers v0.8.1 - https://github.com/tock/tock/tree/master/libraries/tock-register-interface
│   │   ├── arm_pl031 feature "default"
│   │   │   ├── arm_pl031 v0.2.1 - https://github.com/arceos-org/arm_pl031
│   │   │   │   └── chrono v0.4.43 - https://github.com/chronotope/chrono
│   │   │   │       └── num-traits v0.2.19 - https://github.com/rust-num/num-traits
│   │   │   │           [build-dependencies]
│   │   │   │           └── autocfg feature "default"
│   │   │   │               └── autocfg v1.5.0 - https://github.com/cuviper/autocfg
│   │   │   └── arm_pl031 feature "chrono"
│   │   │       └── arm_pl031 v0.2.1 - https://github.com/arceos-org/arm_pl031 (*)
│   │   ├── int_ratio feature "default"
│   │   │   └── int_ratio v0.1.2 - https://github.com/arceos-org/int_ratio
│   │   ├── lazyinit feature "default"
│   │   │   └── lazyinit v0.2.2 - https://github.com/arceos-org/lazyinit
│   │   └── uart_16550 feature "default"
│   │       └── uart_16550 v0.4.0 - https://github.com/rust-osdev/uart_16550
│   │           ├── bitflags feature "default" (*)
│   │           └── rustversion feature "default"
│   │               └── rustversion v1.0.22 (proc-macro) - https://github.com/dtolnay/rustversion
│   └── axplat-aarch64-peripherals feature "irq"
│       ├── axplat-aarch64-peripherals v0.3.0 (https://github.com/kylin-x-kernel/axplat_crates.git?branch=dev#28d9b734) - https://github.com/arceos-org/axplat_crates (*)
│       └── axplat feature "irq" (*)
├── lazyinit feature "default" (*)
├── dw_apb_uart feature "default"
│   └── dw_apb_uart v0.1.0 - https://github.com/arceos-org/dw_apb_uart
│       └── tock-registers feature "default" (*)
└── fdtree_rs feature "default"
    └── fdtree_rs v0.1.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/local_crates/fdtree-rs) - 

axplat-bootloader v0.1.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/local_crates/axplat-bootloader) - 
├── log feature "default" (*)
├── uefi feature "alloc"
│   └── uefi v0.36.1 - https://github.com/rust-osdev/uefi-rs
│       ├── log v0.4.29 - https://github.com/rust-lang/log
│       ├── bitflags feature "default" (*)
│       ├── cfg-if feature "default" (*)
│       ├── ptr_meta feature "derive"
│       │   ├── ptr_meta v0.3.1 - https://github.com/rkyv/ptr_meta
│       │   │   └── ptr_meta_derive v0.3.1 (proc-macro) - https://github.com/rkyv/ptr_meta
│       │   │       ├── proc-macro2 feature "proc-macro" (*)
│       │   │       ├── quote feature "proc-macro" (*)
│       │   │       ├── syn feature "clone-impls" (*)
│       │   │       ├── syn feature "derive" (*)
│       │   │       ├── syn feature "full" (*)
│       │   │       ├── syn feature "parsing" (*)
│       │   │       ├── syn feature "printing" (*)
│       │   │       └── syn feature "proc-macro" (*)
│       │   └── ptr_meta feature "ptr_meta_derive"
│       │       └── ptr_meta v0.3.1 - https://github.com/rkyv/ptr_meta (*)
│       ├── ucs2 feature "default"
│       │   └── ucs2 v0.3.3 - https://github.com/rust-osdev/ucs2-rs
│       │       └── bit_field feature "default"
│       │           └── bit_field v0.10.3 - https://github.com/phil-opp/rust-bit-field
│       ├── uefi-macros feature "default"
│       │   └── uefi-macros v0.19.0 (proc-macro) - https://github.com/rust-osdev/uefi-rs
│       │       ├── proc-macro2 feature "default" (*)
│       │       ├── quote feature "default" (*)
│       │       ├── syn feature "default" (*)
│       │       └── syn feature "full" (*)
│       ├── uefi-raw feature "default"
│       │   └── uefi-raw v0.13.0 - https://github.com/rust-osdev/uefi-rs
│       │       ├── bitflags feature "default" (*)
│       │       └── uguid feature "default"
│       │           └── uguid v2.2.1 - https://github.com/google/gpt-disk-rs
│       └── uguid feature "default" (*)
├── uefi feature "default"
│   └── uefi v0.36.1 - https://github.com/rust-osdev/uefi-rs (*)
├── uefi feature "global_allocator"
│   └── uefi v0.36.1 - https://github.com/rust-osdev/uefi-rs (*)
├── uefi feature "logger"
│   └── uefi v0.36.1 - https://github.com/rust-osdev/uefi-rs (*)
└── uefi feature "panic_handler"
    └── uefi v0.36.1 - https://github.com/rust-osdev/uefi-rs (*)

axplat-x86-csv v0.3.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/local_crates/axplat-x86-csv) - https://github.com/arceos-org/starry-next
├── bitflags feature "default" (*)
├── axconfig-macros feature "default" (*)
├── axcpu feature "default" (*)
├── log feature "default" (*)
├── axplat feature "default" (*)
├── kspin feature "default" (*)
├── percpu feature "default" (*)
├── int_ratio feature "default" (*)
├── lazyinit feature "default" (*)
├── uart_16550 feature "default" (*)
├── heapless feature "default"
│   └── heapless v0.9.2 - https://github.com/rust-embedded/heapless
│       ├── stable_deref_trait v1.2.1 - https://github.com/storyyeller/stable_deref_trait
│       └── hash32 feature "default"
│           └── hash32 v0.3.1 - https://github.com/japaric/hash32
│               └── byteorder v1.5.0 - https://github.com/BurntSushi/byteorder
├── multiboot feature "default"
│   └── multiboot v0.8.0 - https://github.com/gz/rust-multiboot
│       └── paste feature "default" (*)
├── raw-cpuid feature "default"
│   └── raw-cpuid v11.6.0 - https://github.com/gz/rust-cpuid
│       └── bitflags feature "default" (*)
├── x2apic feature "default"
│   └── x2apic v0.5.0 - https://github.com/kzhao0986/x2apic-rs
│       ├── paste feature "default" (*)
│       ├── bit feature "default"
│       │   └── bit v0.1.1 - https://github.com/jmi2k/bit-rs
│       ├── bitflags feature "default"
│       │   └── bitflags v1.3.2 - https://github.com/bitflags/bitflags
│       ├── raw-cpuid feature "default"
│       │   └── raw-cpuid v10.7.0 - https://github.com/gz/rust-cpuid
│       │       └── bitflags feature "default" (*)
│       └── x86_64 feature "default"
│           ├── x86_64 v0.15.4 - https://github.com/rust-osdev/x86_64
│           │   ├── bitflags feature "default" (*)
│           │   ├── rustversion feature "default" (*)
│           │   ├── bit_field feature "default" (*)
│           │   ├── const_fn feature "default"
│           │   │   └── const_fn v0.4.11 (proc-macro) - https://github.com/taiki-e/const_fn
│           │   └── volatile feature "default"
│           │       └── volatile v0.4.6 - https://github.com/rust-osdev/volatile
│           ├── x86_64 feature "instructions"
│           │   └── x86_64 v0.15.4 - https://github.com/rust-osdev/x86_64 (*)
│           └── x86_64 feature "nightly"
│               ├── x86_64 v0.15.4 - https://github.com/rust-osdev/x86_64 (*)
│               ├── x86_64 feature "abi_x86_interrupt"
│               │   └── x86_64 v0.15.4 - https://github.com/rust-osdev/x86_64 (*)
│               ├── x86_64 feature "asm_const"
│               │   └── x86_64 v0.15.4 - https://github.com/rust-osdev/x86_64 (*)
│               ├── x86_64 feature "const_fn"
│               │   └── x86_64 v0.15.4 - https://github.com/rust-osdev/x86_64 (*)
│               └── x86_64 feature "step_trait"
│                   └── x86_64 v0.15.4 - https://github.com/rust-osdev/x86_64 (*)
├── x86_64 feature "default" (*)
├── x86 feature "default"
│   └── x86 v0.52.0 - https://github.com/gz/rust-x86
│       ├── bit_field feature "default" (*)
│       ├── bitflags feature "default" (*)
│       └── raw-cpuid feature "default" (*)
└── x86_rtc feature "default"
    └── x86_rtc v0.1.1 - https://github.com/arceos-org/x86_rtc
        └── cfg-if feature "default" (*)

starry v0.1.0 (/Users/debin/Desktop/Codes/amd64/StarryOS) - https://github.com/arceos-org/starry-next
├── axplat-aarch64-crosvm-virt feature "default" (command-line)
│   └── axplat-aarch64-crosvm-virt v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/local_crates/axplat-aarch64-crosvm-virt) - https://github.com/kylin-x-kernel/axplat-aarch64-crosvm-virt.git (*)
├── axplat-aarch64-crosvm-virt feature "fp-simd" (command-line)
│   ├── axplat-aarch64-crosvm-virt v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/local_crates/axplat-aarch64-crosvm-virt) - https://github.com/kylin-x-kernel/axplat-aarch64-crosvm-virt.git (*)
│   └── axcpu feature "fp-simd"
│       └── axcpu v0.3.0 (https://github.com/kylin-x-kernel/axcpu?branch=dev#df212325) - https://github.com/arceos-org/axcpu (*)
├── axplat-aarch64-crosvm-virt feature "irq" (command-line)
│   ├── axplat-aarch64-crosvm-virt v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/local_crates/axplat-aarch64-crosvm-virt) - https://github.com/kylin-x-kernel/axplat-aarch64-crosvm-virt.git (*)
│   └── axplat feature "irq" (*)
├── axplat-aarch64-crosvm-virt feature "rtc" (command-line)
│   └── axplat-aarch64-crosvm-virt v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/local_crates/axplat-aarch64-crosvm-virt) - https://github.com/kylin-x-kernel/axplat-aarch64-crosvm-virt.git (*)
├── axplat-aarch64-crosvm-virt feature "smp" (command-line)
│   ├── axplat-aarch64-crosvm-virt v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/local_crates/axplat-aarch64-crosvm-virt) - https://github.com/kylin-x-kernel/axplat-aarch64-crosvm-virt.git (*)
│   └── axplat feature "smp"
│       ├── axplat v0.3.0 (https://github.com/kylin-x-kernel/axplat_crates.git?branch=dev#28d9b734) - https://github.com/arceos-org/axplat_crates (*)
│       └── kspin feature "smp"
│           └── kspin v0.1.1 - https://github.com/arceos-org/kspin (*)
├── cfg-if feature "default" (*)
├── linkme feature "default" (*)
├── axplat-x86-csv feature "default" (command-line)
│   └── axplat-x86-csv v0.3.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/local_crates/axplat-x86-csv) - https://github.com/arceos-org/starry-next (*)
├── axplat-x86-csv feature "fp-simd" (command-line)
│   ├── axplat-x86-csv v0.3.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/local_crates/axplat-x86-csv) - https://github.com/arceos-org/starry-next (*)
│   └── axcpu feature "fp-simd" (*)
├── axplat-x86-csv feature "irq" (command-line)
│   ├── axplat-x86-csv v0.3.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/local_crates/axplat-x86-csv) - https://github.com/arceos-org/starry-next (*)
│   └── axplat feature "irq" (*)
├── axplat-x86-csv feature "rtc" (command-line)
│   ├── axplat-x86-csv v0.3.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/local_crates/axplat-x86-csv) - https://github.com/arceos-org/starry-next (*)
│   └── axplat-x86-csv feature "x86_rtc" (command-line)
│       └── axplat-x86-csv v0.3.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/local_crates/axplat-x86-csv) - https://github.com/arceos-org/starry-next (*)
├── axplat-x86-csv feature "smp" (command-line)
│   ├── axplat-x86-csv v0.3.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/local_crates/axplat-x86-csv) - https://github.com/arceos-org/starry-next (*)
│   ├── axplat feature "smp" (*)
│   └── kspin feature "smp" (*)
├── axdriver feature "default"
│   ├── axdriver v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axdriver) - https://github.com/arceos-org/arceos/tree/main/modules/axdriver
│   │   ├── cfg-if feature "default" (*)
│   │   ├── log feature "default" (*)
│   │   ├── spin feature "default" (*)
│   │   ├── crate_interface feature "default" (*)
│   │   ├── axalloc feature "default"
│   │   │   ├── axalloc v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axalloc) - https://github.com/arceos-org/arceos/tree/main/modules/axalloc
│   │   │   │   ├── axbacktrace feature "default" (*)
│   │   │   │   ├── cfg-if feature "default" (*)
│   │   │   │   ├── log feature "default" (*)
│   │   │   │   ├── kspin feature "default" (*)
│   │   │   │   ├── memory_addr feature "default" (*)
│   │   │   │   ├── percpu feature "default" (*)
│   │   │   │   ├── allocator feature "axerrno"
│   │   │   │   │   └── allocator v0.1.2 (https://github.com/arceos-org/allocator.git?tag=v0.1.2#922e72a7) - https://github.com/arceos-org/allocator
│   │   │   │   │       ├── cfg-if feature "default" (*)
│   │   │   │   │       ├── axerrno feature "default"
│   │   │   │   │       │   └── axerrno v0.1.2 - https://github.com/arceos-org/axerrno
│   │   │   │   │       │       ├── log feature "default" (*)
│   │   │   │   │       │       └── axerrno feature "default"
│   │   │   │   │       │           └── axerrno v0.2.2 - https://github.com/arceos-org/axerrno
│   │   │   │   │       │               ├── log feature "default" (*)
│   │   │   │   │       │               └── strum feature "derive"
│   │   │   │   │       │                   ├── strum v0.27.2 - https://github.com/Peternator7/strum
│   │   │   │   │       │                   │   └── strum_macros feature "default"
│   │   │   │   │       │                   │       └── strum_macros v0.27.2 (proc-macro) - https://github.com/Peternator7/strum
│   │   │   │   │       │                   │           ├── heck feature "default" (*)
│   │   │   │   │       │                   │           ├── proc-macro2 feature "default" (*)
│   │   │   │   │       │                   │           ├── quote feature "default" (*)
│   │   │   │   │       │                   │           ├── syn feature "default" (*)
│   │   │   │   │       │                   │           └── syn feature "parsing" (*)
│   │   │   │   │       │                   └── strum feature "strum_macros"
│   │   │   │   │       │                       └── strum v0.27.2 - https://github.com/Peternator7/strum (*)
│   │   │   │   │       ├── bitmap-allocator feature "default"
│   │   │   │   │       │   └── bitmap-allocator v0.2.1 - https://github.com/rcore-os/bitmap-allocator
│   │   │   │   │       │       └── bit_field feature "default" (*)
│   │   │   │   │       ├── rlsf feature "default"
│   │   │   │   │       │   └── rlsf v0.2.1 - https://github.com/yvt/rlsf
│   │   │   │   │       │       ├── const-default v1.0.0 - https://github.com/AerialX/const-default.rs
│   │   │   │   │       │       ├── cfg-if feature "default" (*)
│   │   │   │   │       │       ├── libc feature "default"
│   │   │   │   │       │       │   ├── libc v0.2.180 - https://github.com/rust-lang/libc
│   │   │   │   │       │       │   └── libc feature "std"
│   │   │   │   │       │       │       └── libc v0.2.180 - https://github.com/rust-lang/libc
│   │   │   │   │       │       └── svgbobdoc feature "default"
│   │   │   │   │       │           └── svgbobdoc v0.3.0 (proc-macro) - https://github.com/yvt/svgbobdoc
│   │   │   │   │       │               ├── proc-macro2 feature "default" (*)
│   │   │   │   │       │               ├── quote feature "default" (*)
│   │   │   │   │       │               ├── base64 feature "default"
│   │   │   │   │       │               │   ├── base64 v0.13.1 - https://github.com/marshallpierce/rust-base64
│   │   │   │   │       │               │   └── base64 feature "std"
│   │   │   │   │       │               │       └── base64 v0.13.1 - https://github.com/marshallpierce/rust-base64
│   │   │   │   │       │               ├── syn feature "default"
│   │   │   │   │       │               │   ├── syn v1.0.109 - https://github.com/dtolnay/syn
│   │   │   │   │       │               │   │   ├── proc-macro2 v1.0.105 - https://github.com/dtolnay/proc-macro2 (*)
│   │   │   │   │       │               │   │   ├── quote v1.0.43 - https://github.com/dtolnay/quote (*)
│   │   │   │   │       │               │   │   └── unicode-ident feature "default" (*)
│   │   │   │   │       │               │   ├── syn feature "clone-impls"
│   │   │   │   │       │               │   │   └── syn v1.0.109 - https://github.com/dtolnay/syn (*)
│   │   │   │   │       │               │   ├── syn feature "derive"
│   │   │   │   │       │               │   │   └── syn v1.0.109 - https://github.com/dtolnay/syn (*)
│   │   │   │   │       │               │   ├── syn feature "parsing"
│   │   │   │   │       │               │   │   └── syn v1.0.109 - https://github.com/dtolnay/syn (*)
│   │   │   │   │       │               │   ├── syn feature "printing"
│   │   │   │   │       │               │   │   ├── syn v1.0.109 - https://github.com/dtolnay/syn (*)
│   │   │   │   │       │               │   │   └── syn feature "quote"
│   │   │   │   │       │               │   │       └── syn v1.0.109 - https://github.com/dtolnay/syn (*)
│   │   │   │   │       │               │   └── syn feature "proc-macro"
│   │   │   │   │       │               │       ├── syn v1.0.109 - https://github.com/dtolnay/syn (*)
│   │   │   │   │       │               │       ├── proc-macro2 feature "proc-macro" (*)
│   │   │   │   │       │               │       ├── quote feature "proc-macro" (*)
│   │   │   │   │       │               │       └── syn feature "quote" (*)
│   │   │   │   │       │               └── unicode-width feature "default"
│   │   │   │   │       │                   ├── unicode-width v0.1.14 - https://github.com/unicode-rs/unicode-width
│   │   │   │   │       │                   └── unicode-width feature "cjk"
│   │   │   │   │       │                       └── unicode-width v0.1.14 - https://github.com/unicode-rs/unicode-width
│   │   │   │   │       └── slab_allocator feature "default"
│   │   │   │   │           └── slab_allocator v0.3.1 (https://github.com/arceos-org/slab_allocator.git?tag=v0.3.1#3c13499d) - 
│   │   │   │   │               └── buddy_system_allocator v0.10.0 - https://github.com/rcore-os/buddy_system_allocator
│   │   │   │   ├── allocator feature "bitmap"
│   │   │   │   │   └── allocator v0.1.2 (https://github.com/arceos-org/allocator.git?tag=v0.1.2#922e72a7) - https://github.com/arceos-org/allocator (*)
│   │   │   │   ├── allocator feature "default"
│   │   │   │   │   ├── allocator v0.1.2 (https://github.com/arceos-org/allocator.git?tag=v0.1.2#922e72a7) - https://github.com/arceos-org/allocator (*)
│   │   │   │   │   └── allocator feature "page-alloc-256m"
│   │   │   │   │       └── allocator v0.1.2 (https://github.com/arceos-org/allocator.git?tag=v0.1.2#922e72a7) - https://github.com/arceos-org/allocator (*)
│   │   │   │   ├── axerrno feature "default" (*)
│   │   │   │   └── strum feature "derive" (*)
│   │   │   ├── axalloc feature "tlsf"
│   │   │   │   ├── axalloc v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axalloc) - https://github.com/arceos-org/arceos/tree/main/modules/axalloc (*)
│   │   │   │   └── allocator feature "tlsf"
│   │   │   │       └── allocator v0.1.2 (https://github.com/arceos-org/allocator.git?tag=v0.1.2#922e72a7) - https://github.com/arceos-org/allocator (*)
│   │   │   └── allocator feature "page-alloc-256m" (*)
│   │   ├── axconfig feature "default"
│   │   │   └── axconfig v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axconfig) - https://github.com/arceos-org/arceos/tree/main/modules/axconfig
│   │   │       └── axconfig-macros feature "default" (*)
│   │   ├── axdriver_base feature "default"
│   │   │   └── axdriver_base v0.1.2 (https://github.com/kylin-x-kernel/axdriver_crates.git?branch=dev#cab57c73) - https://github.com/arceos-org/axdriver_crates
│   │   ├── axdriver_block feature "default"
│   │   │   └── axdriver_block v0.1.2 (https://github.com/kylin-x-kernel/axdriver_crates.git?branch=dev#cab57c73) - https://github.com/arceos-org/axdriver_crates
│   │   │       ├── log feature "default" (*)
│   │   │       ├── axdriver_base feature "default" (*)
│   │   │       └── simple-sdmmc feature "default"
│   │   │           └── simple-sdmmc v0.1.0 (https://github.com/Starry-OS/simple-sdmmc.git?rev=9e6420c#9e6420c3) - 
│   │   │               ├── log feature "default" (*)
│   │   │               ├── bitfield-struct feature "default"
│   │   │               │   └── bitfield-struct v0.11.0 (proc-macro) - https://github.com/wrenger/bitfield-struct-rs.git
│   │   │               │       ├── proc-macro2 feature "default" (*)
│   │   │               │       ├── quote feature "default" (*)
│   │   │               │       ├── syn feature "default" (*)
│   │   │               │       ├── syn feature "extra-traits"
│   │   │               │       │   └── syn v2.0.114 - https://github.com/dtolnay/syn (*)
│   │   │               │       └── syn feature "full" (*)
│   │   │               ├── volatile feature "default"
│   │   │               │   └── volatile v0.6.1 - https://github.com/rust-osdev/volatile
│   │   │               │       └── volatile-macro feature "default"
│   │   │               │           └── volatile-macro v0.6.0 (proc-macro) - https://github.com/rust-osdev/volatile
│   │   │               │               ├── proc-macro2 feature "default" (*)
│   │   │               │               ├── quote feature "default" (*)
│   │   │               │               ├── syn feature "default" (*)
│   │   │               │               └── syn feature "full" (*)
│   │   │               └── volatile feature "derive"
│   │   │                   └── volatile v0.6.1 - https://github.com/rust-osdev/volatile (*)
│   │   ├── axdriver_display feature "default"
│   │   │   └── axdriver_display v0.1.2 (https://github.com/kylin-x-kernel/axdriver_crates.git?branch=dev#cab57c73) - https://github.com/arceos-org/axdriver_crates
│   │   │       └── axdriver_base feature "default" (*)
│   │   ├── axdriver_input feature "default"
│   │   │   └── axdriver_input v0.1.2 (https://github.com/kylin-x-kernel/axdriver_crates.git?branch=dev#cab57c73) - https://github.com/arceos-org/axdriver_crates
│   │   │       ├── strum feature "derive" (*)
│   │   │       └── axdriver_base feature "default" (*)
│   │   ├── axdriver_net feature "default"
│   │   │   └── axdriver_net v0.1.2 (https://github.com/kylin-x-kernel/axdriver_crates.git?branch=dev#cab57c73) - https://github.com/arceos-org/axdriver_crates
│   │   │       ├── log feature "default" (*)
│   │   │       ├── spin feature "default" (*)
│   │   │       └── axdriver_base feature "default" (*)
│   │   ├── axdriver_pci feature "default"
│   │   │   └── axdriver_pci v0.1.2 (https://github.com/kylin-x-kernel/axdriver_crates.git?branch=dev#cab57c73) - https://github.com/arceos-org/axdriver_crates
│   │   │       └── virtio-drivers feature "default"
│   │   │           ├── virtio-drivers v0.7.5 - https://github.com/rcore-os/virtio-drivers
│   │   │           │   ├── bitflags feature "default" (*)
│   │   │           │   ├── log feature "default" (*)
│   │   │           │   ├── enumn feature "default"
│   │   │           │   │   └── enumn v0.1.14 (proc-macro) - https://github.com/dtolnay/enumn
│   │   │           │   │       ├── proc-macro2 feature "default" (*)
│   │   │           │   │       ├── quote feature "default" (*)
│   │   │           │   │       └── syn feature "default" (*)
│   │   │           │   ├── zerocopy feature "default"
│   │   │           │   │   ├── zerocopy v0.7.35 - https://github.com/google/zerocopy
│   │   │           │   │   │   ├── byteorder v1.5.0 - https://github.com/BurntSushi/byteorder
│   │   │           │   │   │   └── zerocopy-derive feature "default"
│   │   │           │   │   │       └── zerocopy-derive v0.7.35 (proc-macro) - https://github.com/google/zerocopy
│   │   │           │   │   │           ├── proc-macro2 feature "default" (*)
│   │   │           │   │   │           ├── quote feature "default" (*)
│   │   │           │   │   │           └── syn feature "default" (*)
│   │   │           │   │   └── zerocopy feature "byteorder"
│   │   │           │   │       └── zerocopy v0.7.35 - https://github.com/google/zerocopy (*)
│   │   │           │   └── zerocopy feature "derive"
│   │   │           │       ├── zerocopy v0.7.35 - https://github.com/google/zerocopy (*)
│   │   │           │       └── zerocopy feature "zerocopy-derive"
│   │   │           │           └── zerocopy v0.7.35 - https://github.com/google/zerocopy (*)
│   │   │           └── virtio-drivers feature "alloc"
│   │   │               ├── virtio-drivers v0.7.5 - https://github.com/rcore-os/virtio-drivers (*)
│   │   │               └── zerocopy feature "alloc"
│   │   │                   └── zerocopy v0.7.35 - https://github.com/google/zerocopy (*)
│   │   ├── axdriver_virtio feature "default"
│   │   │   └── axdriver_virtio v0.1.2 (https://github.com/kylin-x-kernel/axdriver_crates.git?branch=dev#cab57c73) - https://github.com/arceos-org/axdriver_crates
│   │   │       ├── virtio-drivers v0.7.5 - https://github.com/rcore-os/virtio-drivers (*)
│   │   │       ├── log feature "default" (*)
│   │   │       ├── axdriver_base feature "default" (*)
│   │   │       ├── axdriver_block feature "default" (*)
│   │   │       ├── axdriver_display feature "default" (*)
│   │   │       ├── axdriver_input feature "default" (*)
│   │   │       ├── axdriver_net feature "default" (*)
│   │   │       └── axdriver_vsock feature "default"
│   │   │           └── axdriver_vsock v0.1.2 (https://github.com/kylin-x-kernel/axdriver_crates.git?branch=dev#cab57c73) - https://github.com/arceos-org/axdriver_crates
│   │   │               ├── log feature "default" (*)
│   │   │               └── axdriver_base feature "default" (*)
│   │   ├── axdriver_vsock feature "default" (*)
│   │   ├── axhal feature "default"
│   │   │   └── axhal v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axhal) - https://github.com/arceos-org/arceos/tree/main/modules/axhal
│   │   │       ├── axcpu feature "default" (*)
│   │   │       ├── cfg-if feature "default" (*)
│   │   │       ├── log feature "default" (*)
│   │   │       ├── axplat feature "default" (*)
│   │   │       ├── kernel_guard feature "default" (*)
│   │   │       ├── memory_addr feature "default" (*)
│   │   │       ├── percpu feature "default" (*)
│   │   │       ├── linkme feature "default" (*)
│   │   │       ├── lazyinit feature "default" (*)
│   │   │       ├── heapless feature "default" (*)
│   │   │       ├── axalloc feature "default" (*)
│   │   │       ├── axconfig feature "default" (*)
│   │   │       ├── axplat-aarch64-qemu-virt feature "default"
│   │   │       │   └── axplat-aarch64-qemu-virt v0.3.0 (https://github.com/kylin-x-kernel/axplat_crates.git?branch=dev#28d9b734) - https://github.com/arceos-org/axplat_crates
│   │   │       │       ├── axconfig-macros feature "default" (*)
│   │   │       │       ├── axcpu feature "default" (*)
│   │   │       │       ├── log feature "default" (*)
│   │   │       │       ├── axplat feature "default" (*)
│   │   │       │       ├── page_table_entry feature "default" (*)
│   │   │       │       └── axplat-aarch64-peripherals feature "default"
│   │   │       │           ├── axplat-aarch64-peripherals v0.3.0 (https://github.com/kylin-x-kernel/axplat_crates.git?branch=dev#28d9b734) - https://github.com/arceos-org/axplat_crates (*)
│   │   │       │           └── axplat-aarch64-peripherals feature "gicv2"
│   │   │       │               ├── axplat-aarch64-peripherals v0.3.0 (https://github.com/kylin-x-kernel/axplat_crates.git?branch=dev#28d9b734) - https://github.com/arceos-org/axplat_crates (*)
│   │   │       │               └── axplat-aarch64-peripherals feature "irq" (*)
│   │   │       ├── fdt-parser feature "default"
│   │   │       │   └── fdt-parser v0.4.18 - https://github.com/qclic/fdt-parser
│   │   │       ├── page_table_multiarch feature "axerrno"
│   │   │       │   └── page_table_multiarch v0.5.7 (https://github.com/kylin-x-kernel/page_table_multiarch.git?branch=dev#01df8185) - https://github.com/arceos-org/page_table_multiarch
│   │   │       │       ├── arrayvec v0.7.6 - https://github.com/bluss/arrayvec
│   │   │       │       ├── log feature "default" (*)
│   │   │       │       ├── memory_addr feature "default" (*)
│   │   │       │       ├── page_table_entry feature "default" (*)
│   │   │       │       └── axerrno feature "default" (*)
│   │   │       └── page_table_multiarch feature "default"
│   │   │           └── page_table_multiarch v0.5.7 (https://github.com/kylin-x-kernel/page_table_multiarch.git?branch=dev#01df8185) - https://github.com/arceos-org/page_table_multiarch (*)
│   │   │       [build-dependencies]
│   │   │       └── axconfig feature "default" (*)
│   │   ├── axsync feature "default"
│   │   │   └── axsync v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axsync) - https://github.com/arceos-org/arceos/tree/main/modules/axsync
│   │   │       ├── event-listener v5.4.1 - https://github.com/smol-rs/event-listener
│   │   │       │   ├── concurrent-queue v2.5.0 - https://github.com/smol-rs/concurrent-queue
│   │   │       │   │   └── crossbeam-utils v0.8.21 - https://github.com/crossbeam-rs/crossbeam
│   │   │       │   └── pin-project-lite feature "default"
│   │   │       │       └── pin-project-lite v0.2.16 - https://github.com/taiki-e/pin-project-lite
│   │   │       ├── lock_api v0.4.14 - https://github.com/Amanieu/parking_lot (*)
│   │   │       ├── kspin feature "default" (*)
│   │   │       └── axtask feature "default"
│   │   │           └── axtask v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axtask) - https://github.com/arceos-org/arceos/tree/main/modules/axtask
│   │   │               ├── event-listener v5.4.1 - https://github.com/smol-rs/event-listener (*)
│   │   │               ├── cfg-if feature "default" (*)
│   │   │               ├── log feature "default" (*)
│   │   │               ├── crate_interface feature "default" (*)
│   │   │               ├── kspin feature "default" (*)
│   │   │               ├── kernel_guard feature "default" (*)
│   │   │               ├── memory_addr feature "default" (*)
│   │   │               ├── percpu feature "default" (*)
│   │   │               ├── lazyinit feature "default" (*)
│   │   │               ├── axerrno feature "default" (*)
│   │   │               ├── axconfig feature "default" (*)
│   │   │               ├── axhal feature "default" (*)
│   │   │               ├── axpoll feature "default"
│   │   │               │   ├── axpoll v0.1.0 - https://github.com/Starry-OS/axpoll
│   │   │               │   │   ├── bitflags v2.10.0 - https://github.com/bitflags/bitflags
│   │   │               │   │   ├── spin feature "spin_mutex" (*)
│   │   │               │   │   ├── linux-raw-sys feature "general"
│   │   │               │   │   │   └── linux-raw-sys v0.11.0 - https://github.com/sunfishcode/linux-raw-sys
│   │   │               │   │   └── linux-raw-sys feature "no_std"
│   │   │               │   │       └── linux-raw-sys v0.11.0 - https://github.com/sunfishcode/linux-raw-sys
│   │   │               │   └── axpoll feature "alloc"
│   │   │               │       └── axpoll v0.1.0 - https://github.com/Starry-OS/axpoll (*)
│   │   │               ├── axsched feature "default"
│   │   │               │   └── axsched v0.3.1 - https://github.com/arceos-org/axsched
│   │   │               │       └── linked_list_r4l feature "default"
│   │   │               │           └── linked_list_r4l v0.3.0 - https://github.com/arceos-org/linked_list_r4l
│   │   │               ├── cpumask feature "default"
│   │   │               │   └── cpumask v0.1.0 - https://github.com/arceos-org/cpumask
│   │   │               │       └── bitmaps v3.2.1 - https://github.com/bodil/bitmaps
│   │   │               ├── extern-trait feature "default"
│   │   │               │   └── extern-trait v0.2.0 (proc-macro) - https://github.com/AsakuraMizu/extern-trait
│   │   │               │       ├── proc-macro2 feature "default" (*)
│   │   │               │       ├── quote feature "default" (*)
│   │   │               │       ├── syn feature "default" (*)
│   │   │               │       ├── syn feature "extra-traits" (*)
│   │   │               │       └── syn feature "full" (*)
│   │   │               ├── futures-util feature "alloc"
│   │   │               │   ├── futures-util v0.3.31 - https://github.com/rust-lang/futures-rs
│   │   │               │   │   ├── futures-core v0.3.31 - https://github.com/rust-lang/futures-rs
│   │   │               │   │   ├── futures-macro v0.3.31 (proc-macro) - https://github.com/rust-lang/futures-rs
│   │   │               │   │   │   ├── proc-macro2 feature "default" (*)
│   │   │               │   │   │   ├── quote feature "default" (*)
│   │   │               │   │   │   ├── syn feature "default" (*)
│   │   │               │   │   │   └── syn feature "full" (*)
│   │   │               │   │   ├── futures-task v0.3.31 - https://github.com/rust-lang/futures-rs
│   │   │               │   │   ├── pin-project-lite feature "default" (*)
│   │   │               │   │   └── pin-utils feature "default"
│   │   │               │   │       └── pin-utils v0.1.0 - https://github.com/rust-lang-nursery/pin-utils
│   │   │               │   ├── futures-core feature "alloc"
│   │   │               │   │   └── futures-core v0.3.31 - https://github.com/rust-lang/futures-rs
│   │   │               │   └── futures-task feature "alloc"
│   │   │               │       └── futures-task v0.3.31 - https://github.com/rust-lang/futures-rs
│   │   │               └── futures-util feature "async-await-macro"
│   │   │                   ├── futures-util v0.3.31 - https://github.com/rust-lang/futures-rs (*)
│   │   │                   ├── futures-util feature "async-await"
│   │   │                   │   └── futures-util v0.3.31 - https://github.com/rust-lang/futures-rs (*)
│   │   │                   └── futures-util feature "futures-macro"
│   │   │                       └── futures-util v0.3.31 - https://github.com/rust-lang/futures-rs (*)
│   │   ├── hashbrown feature "default"
│   │   │   ├── hashbrown v0.16.1 - https://github.com/rust-lang/hashbrown
│   │   │   │   ├── equivalent v1.0.2 - https://github.com/indexmap-rs/equivalent
│   │   │   │   ├── foldhash v0.2.0 - https://github.com/orlp/foldhash
│   │   │   │   └── allocator-api2 feature "alloc"
│   │   │   │       └── allocator-api2 v0.2.21 - https://github.com/zakarumych/allocator-api2
│   │   │   ├── hashbrown feature "allocator-api2"
│   │   │   │   └── hashbrown v0.16.1 - https://github.com/rust-lang/hashbrown (*)
│   │   │   ├── hashbrown feature "default-hasher"
│   │   │   │   └── hashbrown v0.16.1 - https://github.com/rust-lang/hashbrown (*)
│   │   │   ├── hashbrown feature "equivalent"
│   │   │   │   └── hashbrown v0.16.1 - https://github.com/rust-lang/hashbrown (*)
│   │   │   ├── hashbrown feature "inline-more"
│   │   │   │   └── hashbrown v0.16.1 - https://github.com/rust-lang/hashbrown (*)
│   │   │   └── hashbrown feature "raw-entry"
│   │   │       └── hashbrown v0.16.1 - https://github.com/rust-lang/hashbrown (*)
│   │   ├── smallvec feature "const_generics"
│   │   │   └── smallvec v1.15.1 - https://github.com/servo/rust-smallvec
│   │   ├── smallvec feature "default"
│   │   │   └── smallvec v1.15.1 - https://github.com/servo/rust-smallvec
│   │   └── smallvec feature "union"
│   │       └── smallvec v1.15.1 - https://github.com/servo/rust-smallvec
│   └── axdriver feature "bus-pci"
│       └── axdriver v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axdriver) - https://github.com/arceos-org/arceos/tree/main/modules/axdriver (*)
├── axerrno feature "default" (*)
├── axhal feature "default" (*)
├── axsync feature "default" (*)
├── axtask feature "default" (*)
├── linux-raw-sys feature "general" (*)
├── linux-raw-sys feature "net"
│   └── linux-raw-sys v0.11.0 - https://github.com/sunfishcode/linux-raw-sys
├── linux-raw-sys feature "no_std" (*)
├── linux-raw-sys feature "prctl"
│   └── linux-raw-sys v0.11.0 - https://github.com/sunfishcode/linux-raw-sys
├── linux-raw-sys feature "system"
│   └── linux-raw-sys v0.11.0 - https://github.com/sunfishcode/linux-raw-sys
├── axfeat feature "alloc-slab"
│   ├── axfeat v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/api/axfeat) - https://github.com/arceos-org/arceos/tree/main/api/axfeat
│   │   ├── axbacktrace feature "default" (*)
│   │   ├── kspin feature "default" (*)
│   │   ├── axdriver feature "default" (*)
│   │   ├── axalloc feature "default" (*)
│   │   ├── axhal feature "default" (*)
│   │   ├── axsync feature "default" (*)
│   │   ├── axtask feature "default" (*)
│   │   ├── axdisplay feature "default"
│   │   │   └── axdisplay v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axdisplay) - https://github.com/arceos-org/arceos/tree/main/modules/axdisplay
│   │   │       ├── log feature "default" (*)
│   │   │       ├── lazyinit feature "default" (*)
│   │   │       ├── axdriver feature "default" (*)
│   │   │       ├── axdriver feature "display"
│   │   │       │   └── axdriver v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axdriver) - https://github.com/arceos-org/arceos/tree/main/modules/axdriver (*)
│   │   │       └── axsync feature "default" (*)
│   │   ├── axfs feature "default"
│   │   │   └── axfs v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axfs) - 
│   │   │       ├── chrono v0.4.43 - https://github.com/chronotope/chrono (*)
│   │   │       ├── lwext4_rust v0.2.0 (https://github.com/Starry-OS/lwext4_rust.git?rev=033fa2c#033fa2cc) - https://github.com/Mivik/lwext4_rust
│   │   │       │   └── log feature "default" (*)
│   │   │       │   [build-dependencies]
│   │   │       │   └── bindgen feature "default"
│   │   │       │       ├── bindgen v0.72.1 - https://github.com/rust-lang/rust-bindgen
│   │   │       │       │   ├── itertools v0.13.0 - https://github.com/rust-itertools/itertools
│   │   │       │       │   │   └── either v1.15.0 - https://github.com/rayon-rs/either
│   │   │       │       │   ├── quote v1.0.43 - https://github.com/dtolnay/quote (*)
│   │   │       │       │   ├── bitflags feature "default" (*)
│   │   │       │       │   ├── proc-macro2 feature "default" (*)
│   │   │       │       │   ├── syn feature "default" (*)
│   │   │       │       │   ├── syn feature "extra-traits" (*)
│   │   │       │       │   ├── syn feature "full" (*)
│   │   │       │       │   ├── syn feature "visit-mut"
│   │   │       │       │   │   └── syn v2.0.114 - https://github.com/dtolnay/syn (*)
│   │   │       │       │   ├── log feature "default" (*)
│   │   │       │       │   ├── cexpr feature "default"
│   │   │       │       │   │   └── cexpr v0.6.0 - https://github.com/jethrogb/rust-cexpr
│   │   │       │       │   │       └── nom feature "std"
│   │   │       │       │   │           ├── nom v7.1.3 - https://github.com/Geal/nom
│   │   │       │       │   │           │   ├── memchr v2.7.6 - https://github.com/BurntSushi/memchr
│   │   │       │       │   │           │   └── minimal-lexical v0.2.1 - https://github.com/Alexhuszagh/minimal-lexical
│   │   │       │       │   │           ├── nom feature "alloc"
│   │   │       │       │   │           │   └── nom v7.1.3 - https://github.com/Geal/nom (*)
│   │   │       │       │   │           ├── memchr feature "std"
│   │   │       │       │   │           │   ├── memchr v2.7.6 - https://github.com/BurntSushi/memchr
│   │   │       │       │   │           │   └── memchr feature "alloc"
│   │   │       │       │   │           │       └── memchr v2.7.6 - https://github.com/BurntSushi/memchr
│   │   │       │       │   │           └── minimal-lexical feature "std"
│   │   │       │       │   │               └── minimal-lexical v0.2.1 - https://github.com/Alexhuszagh/minimal-lexical
│   │   │       │       │   ├── clang-sys feature "clang_11_0"
│   │   │       │       │   │   ├── clang-sys v1.8.1 - https://github.com/KyleMayes/clang-sys
│   │   │       │       │   │   │   ├── libc v0.2.180 - https://github.com/rust-lang/libc
│   │   │       │       │   │   │   ├── glob feature "default"
│   │   │       │       │   │   │   │   └── glob v0.3.3 - https://github.com/rust-lang/glob
│   │   │       │       │   │   │   └── libloading feature "default"
│   │   │       │       │   │   │       └── libloading v0.8.9 - https://github.com/nagisa/rust_libloading/
│   │   │       │       │   │   │           └── cfg-if feature "default" (*)
│   │   │       │       │   │   │   [build-dependencies]
│   │   │       │       │   │   │   └── glob feature "default" (*)
│   │   │       │       │   │   └── clang-sys feature "clang_10_0"
│   │   │       │       │   │       ├── clang-sys v1.8.1 - https://github.com/KyleMayes/clang-sys (*)
│   │   │       │       │   │       └── clang-sys feature "clang_9_0"
│   │   │       │       │   │           ├── clang-sys v1.8.1 - https://github.com/KyleMayes/clang-sys (*)
│   │   │       │       │   │           └── clang-sys feature "clang_8_0"
│   │   │       │       │   │               ├── clang-sys v1.8.1 - https://github.com/KyleMayes/clang-sys (*)
│   │   │       │       │   │               └── clang-sys feature "clang_7_0"
│   │   │       │       │   │                   ├── clang-sys v1.8.1 - https://github.com/KyleMayes/clang-sys (*)
│   │   │       │       │   │                   └── clang-sys feature "clang_6_0"
│   │   │       │       │   │                       ├── clang-sys v1.8.1 - https://github.com/KyleMayes/clang-sys (*)
│   │   │       │       │   │                       └── clang-sys feature "clang_5_0"
│   │   │       │       │   │                           ├── clang-sys v1.8.1 - https://github.com/KyleMayes/clang-sys (*)
│   │   │       │       │   │                           └── clang-sys feature "clang_4_0"
│   │   │       │       │   │                               ├── clang-sys v1.8.1 - https://github.com/KyleMayes/clang-sys (*)
│   │   │       │       │   │                               └── clang-sys feature "clang_3_9"
│   │   │       │       │   │                                   ├── clang-sys v1.8.1 - https://github.com/KyleMayes/clang-sys (*)
│   │   │       │       │   │                                   └── clang-sys feature "clang_3_8"
│   │   │       │       │   │                                       ├── clang-sys v1.8.1 - https://github.com/KyleMayes/clang-sys (*)
│   │   │       │       │   │                                       └── clang-sys feature "clang_3_7"
│   │   │       │       │   │                                           ├── clang-sys v1.8.1 - https://github.com/KyleMayes/clang-sys (*)
│   │   │       │       │   │                                           └── clang-sys feature "clang_3_6"
│   │   │       │       │   │                                               ├── clang-sys v1.8.1 - https://github.com/KyleMayes/clang-sys (*)
│   │   │       │       │   │                                               └── clang-sys feature "clang_3_5"
│   │   │       │       │   │                                                   └── clang-sys v1.8.1 - https://github.com/KyleMayes/clang-sys (*)
│   │   │       │       │   ├── clang-sys feature "default"
│   │   │       │       │   │   └── clang-sys v1.8.1 - https://github.com/KyleMayes/clang-sys (*)
│   │   │       │       │   ├── prettyplease feature "default"
│   │   │       │       │   │   └── prettyplease v0.2.37 - https://github.com/dtolnay/prettyplease
│   │   │       │       │   │       ├── proc-macro2 v1.0.105 - https://github.com/dtolnay/proc-macro2 (*)
│   │   │       │       │   │       └── syn feature "full" (*)
│   │   │       │       │   ├── prettyplease feature "verbatim"
│   │   │       │       │   │   ├── prettyplease v0.2.37 - https://github.com/dtolnay/prettyplease (*)
│   │   │       │       │   │   └── syn feature "parsing" (*)
│   │   │       │       │   ├── regex feature "std"
│   │   │       │       │   │   ├── regex v1.12.2 - https://github.com/rust-lang/regex
│   │   │       │       │   │   │   ├── regex-syntax v0.8.8 - https://github.com/rust-lang/regex
│   │   │       │       │   │   │   ├── regex-automata feature "alloc"
│   │   │       │       │   │   │   │   └── regex-automata v0.4.13 - https://github.com/rust-lang/regex
│   │   │       │       │   │   │   │       └── regex-syntax v0.8.8 - https://github.com/rust-lang/regex
│   │   │       │       │   │   │   ├── regex-automata feature "meta"
│   │   │       │       │   │   │   │   ├── regex-automata v0.4.13 - https://github.com/rust-lang/regex (*)
│   │   │       │       │   │   │   │   ├── regex-automata feature "nfa-pikevm"
│   │   │       │       │   │   │   │   │   ├── regex-automata v0.4.13 - https://github.com/rust-lang/regex (*)
│   │   │       │       │   │   │   │   │   └── regex-automata feature "nfa-thompson"
│   │   │       │       │   │   │   │   │       ├── regex-automata v0.4.13 - https://github.com/rust-lang/regex (*)
│   │   │       │       │   │   │   │   │       └── regex-automata feature "alloc" (*)
│   │   │       │       │   │   │   │   └── regex-automata feature "syntax"
│   │   │       │       │   │   │   │       ├── regex-automata v0.4.13 - https://github.com/rust-lang/regex (*)
│   │   │       │       │   │   │   │       └── regex-automata feature "alloc" (*)
│   │   │       │       │   │   │   ├── regex-automata feature "nfa-pikevm" (*)
│   │   │       │       │   │   │   └── regex-automata feature "syntax" (*)
│   │   │       │       │   │   ├── regex-automata feature "std"
│   │   │       │       │   │   │   ├── regex-automata v0.4.13 - https://github.com/rust-lang/regex (*)
│   │   │       │       │   │   │   ├── regex-automata feature "alloc" (*)
│   │   │       │       │   │   │   └── regex-syntax feature "std"
│   │   │       │       │   │   │       └── regex-syntax v0.8.8 - https://github.com/rust-lang/regex
│   │   │       │       │   │   └── regex-syntax feature "std" (*)
│   │   │       │       │   ├── regex feature "unicode-perl"
│   │   │       │       │   │   ├── regex v1.12.2 - https://github.com/rust-lang/regex (*)
│   │   │       │       │   │   ├── regex-automata feature "unicode-perl"
│   │   │       │       │   │   │   ├── regex-automata v0.4.13 - https://github.com/rust-lang/regex (*)
│   │   │       │       │   │   │   └── regex-syntax feature "unicode-perl"
│   │   │       │       │   │   │       └── regex-syntax v0.8.8 - https://github.com/rust-lang/regex
│   │   │       │       │   │   ├── regex-automata feature "unicode-word-boundary"
│   │   │       │       │   │   │   └── regex-automata v0.4.13 - https://github.com/rust-lang/regex (*)
│   │   │       │       │   │   └── regex-syntax feature "unicode-perl" (*)
│   │   │       │       │   ├── rustc-hash feature "default"
│   │   │       │       │   │   ├── rustc-hash v2.1.1 - https://github.com/rust-lang/rustc-hash
│   │   │       │       │   │   └── rustc-hash feature "std"
│   │   │       │       │   │       └── rustc-hash v2.1.1 - https://github.com/rust-lang/rustc-hash
│   │   │       │       │   └── shlex feature "default"
│   │   │       │       │       ├── shlex v1.3.0 - https://github.com/comex/rust-shlex
│   │   │       │       │       └── shlex feature "std"
│   │   │       │       │           └── shlex v1.3.0 - https://github.com/comex/rust-shlex
│   │   │       │       ├── bindgen feature "logging"
│   │   │       │       │   └── bindgen v0.72.1 - https://github.com/rust-lang/rust-bindgen (*)
│   │   │       │       ├── bindgen feature "prettyplease"
│   │   │       │       │   └── bindgen v0.72.1 - https://github.com/rust-lang/rust-bindgen (*)
│   │   │       │       └── bindgen feature "runtime"
│   │   │       │           ├── bindgen v0.72.1 - https://github.com/rust-lang/rust-bindgen (*)
│   │   │       │           └── clang-sys feature "runtime"
│   │   │       │               ├── clang-sys v1.8.1 - https://github.com/KyleMayes/clang-sys (*)
│   │   │       │               └── clang-sys feature "libloading"
│   │   │       │                   └── clang-sys v1.8.1 - https://github.com/KyleMayes/clang-sys (*)
│   │   │       ├── slab v0.4.11 - https://github.com/tokio-rs/slab
│   │   │       ├── bitflags feature "default" (*)
│   │   │       ├── cfg-if feature "default" (*)
│   │   │       ├── log feature "default" (*)
│   │   │       ├── spin feature "default" (*)
│   │   │       ├── kspin feature "default" (*)
│   │   │       ├── axdriver feature "block"
│   │   │       │   └── axdriver v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axdriver) - https://github.com/arceos-org/arceos/tree/main/modules/axdriver (*)
│   │   │       ├── axdriver feature "default" (*)
│   │   │       ├── axalloc feature "default" (*)
│   │   │       ├── axerrno feature "default" (*)
│   │   │       ├── axhal feature "default" (*)
│   │   │       ├── axsync feature "default" (*)
│   │   │       ├── axpoll feature "default" (*)
│   │   │       ├── axfs-ng-vfs feature "default"
│   │   │       │   └── axfs-ng-vfs v0.1.0 - https://github.com/Starry-OS/axfs-ng-vfs
│   │   │       │       ├── bitflags feature "default" (*)
│   │   │       │       ├── cfg-if feature "default" (*)
│   │   │       │       ├── log feature "default" (*)
│   │   │       │       ├── spin feature "mutex" (*)
│   │   │       │       ├── axerrno feature "default" (*)
│   │   │       │       ├── axpoll feature "default" (*)
│   │   │       │       ├── smallvec feature "default" (*)
│   │   │       │       ├── hashbrown feature "default"
│   │   │       │       │   ├── hashbrown v0.15.5 - https://github.com/rust-lang/hashbrown
│   │   │       │       │   │   ├── equivalent v1.0.2 - https://github.com/indexmap-rs/equivalent
│   │   │       │       │   │   ├── foldhash v0.1.5 - https://github.com/orlp/foldhash
│   │   │       │       │   │   └── allocator-api2 feature "alloc" (*)
│   │   │       │       │   ├── hashbrown feature "allocator-api2"
│   │   │       │       │   │   └── hashbrown v0.15.5 - https://github.com/rust-lang/hashbrown (*)
│   │   │       │       │   ├── hashbrown feature "default-hasher"
│   │   │       │       │   │   └── hashbrown v0.15.5 - https://github.com/rust-lang/hashbrown (*)
│   │   │       │       │   ├── hashbrown feature "equivalent"
│   │   │       │       │   │   └── hashbrown v0.15.5 - https://github.com/rust-lang/hashbrown (*)
│   │   │       │       │   ├── hashbrown feature "inline-more"
│   │   │       │       │   │   └── hashbrown v0.15.5 - https://github.com/rust-lang/hashbrown (*)
│   │   │       │       │   └── hashbrown feature "raw-entry"
│   │   │       │       │       └── hashbrown v0.15.5 - https://github.com/rust-lang/hashbrown (*)
│   │   │       │       └── inherit-methods-macro feature "default"
│   │   │       │           └── inherit-methods-macro v0.1.0 (proc-macro) - https://github.com/asterinas/inherit-methods-macro
│   │   │       │               ├── proc-macro2 feature "default" (*)
│   │   │       │               ├── quote feature "default" (*)
│   │   │       │               ├── syn feature "default" (*)
│   │   │       │               ├── syn feature "extra-traits"
│   │   │       │               │   └── syn v1.0.109 - https://github.com/dtolnay/syn (*)
│   │   │       │               ├── syn feature "full"
│   │   │       │               │   └── syn v1.0.109 - https://github.com/dtolnay/syn (*)
│   │   │       │               └── darling feature "default"
│   │   │       │                   ├── darling v0.13.4 - https://github.com/TedDriggs/darling
│   │   │       │                   │   ├── darling_core feature "default"
│   │   │       │                   │   │   └── darling_core v0.13.4 - https://github.com/TedDriggs/darling
│   │   │       │                   │   │       ├── proc-macro2 feature "default" (*)
│   │   │       │                   │   │       ├── quote feature "default" (*)
│   │   │       │                   │   │       ├── syn feature "default" (*)
│   │   │       │                   │   │       ├── syn feature "extra-traits" (*)
│   │   │       │                   │   │       ├── syn feature "full" (*)
│   │   │       │                   │   │       ├── fnv feature "default"
│   │   │       │                   │   │       │   ├── fnv v1.0.7 - https://github.com/servo/rust-fnv
│   │   │       │                   │   │       │   └── fnv feature "std"
│   │   │       │                   │   │       │       └── fnv v1.0.7 - https://github.com/servo/rust-fnv
│   │   │       │                   │   │       ├── ident_case feature "default"
│   │   │       │                   │   │       │   └── ident_case v1.0.1 - https://github.com/TedDriggs/ident_case
│   │   │       │                   │   │       └── strsim feature "default"
│   │   │       │                   │   │           └── strsim v0.10.0 - https://github.com/dguo/strsim-rs
│   │   │       │                   │   └── darling_macro feature "default"
│   │   │       │                   │       └── darling_macro v0.13.4 (proc-macro) - https://github.com/TedDriggs/darling
│   │   │       │                   │           ├── quote feature "default" (*)
│   │   │       │                   │           ├── syn feature "default" (*)
│   │   │       │                   │           └── darling_core feature "default" (*)
│   │   │       │                   └── darling feature "suggestions"
│   │   │       │                       ├── darling v0.13.4 - https://github.com/TedDriggs/darling (*)
│   │   │       │                       └── darling_core feature "suggestions"
│   │   │       │                           ├── darling_core v0.13.4 - https://github.com/TedDriggs/darling (*)
│   │   │       │                           └── darling_core feature "strsim"
│   │   │       │                               └── darling_core v0.13.4 - https://github.com/TedDriggs/darling (*)
│   │   │       ├── axio feature "alloc"
│   │   │       │   └── axio v0.3.0-pre.1 - https://github.com/arceos-org/axio
│   │   │       │       ├── memchr v2.7.6 - https://github.com/BurntSushi/memchr
│   │   │       │       ├── heapless feature "default" (*)
│   │   │       │       └── axerrno feature "default" (*)
│   │   │       │       [build-dependencies]
│   │   │       │       └── autocfg feature "default" (*)
│   │   │       ├── axio feature "default"
│   │   │       │   └── axio v0.3.0-pre.1 - https://github.com/arceos-org/axio (*)
│   │   │       ├── intrusive-collections feature "default"
│   │   │       │   ├── intrusive-collections v0.9.7 - https://github.com/Amanieu/intrusive-rs
│   │   │       │   │   └── memoffset feature "default"
│   │   │       │   │       └── memoffset v0.9.1 - https://github.com/Gilnaa/memoffset
│   │   │       │   │           [build-dependencies]
│   │   │       │   │           └── autocfg feature "default" (*)
│   │   │       │   └── intrusive-collections feature "alloc"
│   │   │       │       └── intrusive-collections v0.9.7 - https://github.com/Amanieu/intrusive-rs (*)
│   │   │       ├── lru feature "default"
│   │   │       │   ├── lru v0.16.3 - https://github.com/jeromefroe/lru-rs.git
│   │   │       │   │   └── hashbrown feature "default" (*)
│   │   │       │   └── lru feature "hashbrown"
│   │   │       │       └── lru v0.16.3 - https://github.com/jeromefroe/lru-rs.git (*)
│   │   │       └── scope-local feature "default"
│   │   │           └── scope-local v0.1.1 - https://github.com/Starry-OS/scope-local
│   │   │               ├── spin feature "lazy" (*)
│   │   │               └── percpu feature "default" (*)
│   │   ├── axinput feature "default"
│   │   │   └── axinput v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axinput) - https://github.com/arceos-org/arceos
│   │   │       ├── log feature "default" (*)
│   │   │       ├── lazyinit feature "default" (*)
│   │   │       ├── axdriver feature "default" (*)
│   │   │       ├── axdriver feature "input"
│   │   │       │   └── axdriver v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axdriver) - https://github.com/arceos-org/arceos/tree/main/modules/axdriver (*)
│   │   │       └── axsync feature "default" (*)
│   │   ├── axlog feature "default"
│   │   │   └── axlog v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axlog) - https://github.com/arceos-org/arceos/tree/main/modules/axlog
│   │   │       ├── cfg-if feature "default" (*)
│   │   │       ├── log feature "default" (*)
│   │   │       ├── crate_interface feature "default" (*)
│   │   │       └── kspin feature "default" (*)
│   │   ├── axnet feature "default"
│   │   │   └── axnet v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axnet) - https://github.com/arceos-org/arceos/tree/main/modules/axnet
│   │   │       ├── async-channel v2.5.0 - https://github.com/smol-rs/async-channel
│   │   │       │   ├── concurrent-queue v2.5.0 - https://github.com/smol-rs/concurrent-queue (*)
│   │   │       │   ├── event-listener-strategy v0.5.4 - https://github.com/smol-rs/event-listener-strategy
│   │   │       │   │   ├── event-listener v5.4.1 - https://github.com/smol-rs/event-listener (*)
│   │   │       │   │   └── pin-project-lite feature "default" (*)
│   │   │       │   ├── futures-core v0.3.31 - https://github.com/rust-lang/futures-rs
│   │   │       │   └── pin-project-lite feature "default" (*)
│   │   │       ├── event-listener v5.4.1 - https://github.com/smol-rs/event-listener (*)
│   │   │       ├── bitflags feature "default" (*)
│   │   │       ├── cfg-if feature "default" (*)
│   │   │       ├── log feature "default" (*)
│   │   │       ├── spin feature "default" (*)
│   │   │       ├── enum_dispatch feature "default" (*)
│   │   │       ├── lazyinit feature "default" (*)
│   │   │       ├── axdriver feature "default" (*)
│   │   │       ├── axdriver feature "net"
│   │   │       │   └── axdriver v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axdriver) - https://github.com/arceos-org/arceos/tree/main/modules/axdriver (*)
│   │   │       ├── axerrno feature "default" (*)
│   │   │       ├── axconfig feature "default" (*)
│   │   │       ├── axhal feature "default" (*)
│   │   │       ├── axsync feature "default" (*)
│   │   │       ├── axtask feature "default" (*)
│   │   │       ├── axpoll feature "default" (*)
│   │   │       ├── hashbrown feature "default" (*)
│   │   │       ├── axfs feature "default" (*)
│   │   │       ├── axfs-ng-vfs feature "default" (*)
│   │   │       ├── axio feature "default" (*)
│   │   │       ├── async-trait feature "default"
│   │   │       │   └── async-trait v0.1.89 (proc-macro) - https://github.com/dtolnay/async-trait
│   │   │       │       ├── proc-macro2 feature "default" (*)
│   │   │       │       ├── quote feature "default" (*)
│   │   │       │       ├── syn feature "clone-impls" (*)
│   │   │       │       ├── syn feature "full" (*)
│   │   │       │       ├── syn feature "parsing" (*)
│   │   │       │       ├── syn feature "printing" (*)
│   │   │       │       ├── syn feature "proc-macro" (*)
│   │   │       │       └── syn feature "visit-mut" (*)
│   │   │       ├── lazy_static feature "default"
│   │   │       │   └── lazy_static v1.5.0 - https://github.com/rust-lang-nursery/lazy-static.rs
│   │   │       │       └── spin feature "once" (*)
│   │   │       ├── lazy_static feature "spin_no_std"
│   │   │       │   ├── lazy_static v1.5.0 - https://github.com/rust-lang-nursery/lazy-static.rs (*)
│   │   │       │   └── lazy_static feature "spin"
│   │   │       │       └── lazy_static v1.5.0 - https://github.com/rust-lang-nursery/lazy-static.rs (*)
│   │   │       ├── ringbuf feature "alloc"
│   │   │       │   └── ringbuf v0.4.8 - https://github.com/agerasev/ringbuf.git
│   │   │       │       └── crossbeam-utils v0.8.21 - https://github.com/crossbeam-rs/crossbeam
│   │   │       ├── smoltcp feature "alloc"
│   │   │       │   ├── smoltcp v0.12.0 (https://github.com/Starry-OS/smoltcp.git?rev=7401a54#7401a54b) - https://github.com/smoltcp-rs/smoltcp.git
│   │   │       │   │   ├── bitflags v1.3.2 - https://github.com/bitflags/bitflags
│   │   │       │   │   ├── byteorder v1.5.0 - https://github.com/BurntSushi/byteorder
│   │   │       │   │   ├── log v0.4.29 - https://github.com/rust-lang/log
│   │   │       │   │   ├── cfg-if feature "default" (*)
│   │   │       │   │   ├── heapless feature "default"
│   │   │       │   │   │   └── heapless v0.8.0 - https://github.com/rust-embedded/heapless
│   │   │       │   │   │       ├── stable_deref_trait v1.2.1 - https://github.com/storyyeller/stable_deref_trait
│   │   │       │   │   │       └── hash32 feature "default" (*)
│   │   │       │   │   └── managed feature "map"
│   │   │       │   │       └── managed v0.8.0 - https://github.com/m-labs/rust-managed.git
│   │   │       │   └── managed feature "alloc"
│   │   │       │       └── managed v0.8.0 - https://github.com/m-labs/rust-managed.git
│   │   │       ├── smoltcp feature "async"
│   │   │       │   └── smoltcp v0.12.0 (https://github.com/Starry-OS/smoltcp.git?rev=7401a54#7401a54b) - https://github.com/smoltcp-rs/smoltcp.git (*)
│   │   │       ├── smoltcp feature "log"
│   │   │       │   └── smoltcp v0.12.0 (https://github.com/Starry-OS/smoltcp.git?rev=7401a54#7401a54b) - https://github.com/smoltcp-rs/smoltcp.git (*)
│   │   │       ├── smoltcp feature "medium-ethernet"
│   │   │       │   ├── smoltcp v0.12.0 (https://github.com/Starry-OS/smoltcp.git?rev=7401a54#7401a54b) - https://github.com/smoltcp-rs/smoltcp.git (*)
│   │   │       │   └── smoltcp feature "socket"
│   │   │       │       └── smoltcp v0.12.0 (https://github.com/Starry-OS/smoltcp.git?rev=7401a54#7401a54b) - https://github.com/smoltcp-rs/smoltcp.git (*)
│   │   │       ├── smoltcp feature "medium-ip"
│   │   │       │   ├── smoltcp v0.12.0 (https://github.com/Starry-OS/smoltcp.git?rev=7401a54#7401a54b) - https://github.com/smoltcp-rs/smoltcp.git (*)
│   │   │       │   └── smoltcp feature "socket" (*)
│   │   │       ├── smoltcp feature "proto-ipv4"
│   │   │       │   └── smoltcp v0.12.0 (https://github.com/Starry-OS/smoltcp.git?rev=7401a54#7401a54b) - https://github.com/smoltcp-rs/smoltcp.git (*)
│   │   │       ├── smoltcp feature "proto-ipv6"
│   │   │       │   └── smoltcp v0.12.0 (https://github.com/Starry-OS/smoltcp.git?rev=7401a54#7401a54b) - https://github.com/smoltcp-rs/smoltcp.git (*)
│   │   │       ├── smoltcp feature "socket-dns"
│   │   │       │   ├── smoltcp v0.12.0 (https://github.com/Starry-OS/smoltcp.git?rev=7401a54#7401a54b) - https://github.com/smoltcp-rs/smoltcp.git (*)
│   │   │       │   ├── smoltcp feature "proto-dns"
│   │   │       │   │   └── smoltcp v0.12.0 (https://github.com/Starry-OS/smoltcp.git?rev=7401a54#7401a54b) - https://github.com/smoltcp-rs/smoltcp.git (*)
│   │   │       │   └── smoltcp feature "socket" (*)
│   │   │       ├── smoltcp feature "socket-icmp"
│   │   │       │   ├── smoltcp v0.12.0 (https://github.com/Starry-OS/smoltcp.git?rev=7401a54#7401a54b) - https://github.com/smoltcp-rs/smoltcp.git (*)
│   │   │       │   └── smoltcp feature "socket" (*)
│   │   │       ├── smoltcp feature "socket-raw"
│   │   │       │   ├── smoltcp v0.12.0 (https://github.com/Starry-OS/smoltcp.git?rev=7401a54#7401a54b) - https://github.com/smoltcp-rs/smoltcp.git (*)
│   │   │       │   └── smoltcp feature "socket" (*)
│   │   │       ├── smoltcp feature "socket-tcp"
│   │   │       │   ├── smoltcp v0.12.0 (https://github.com/Starry-OS/smoltcp.git?rev=7401a54#7401a54b) - https://github.com/smoltcp-rs/smoltcp.git (*)
│   │   │       │   └── smoltcp feature "socket" (*)
│   │   │       └── smoltcp feature "socket-udp"
│   │   │           ├── smoltcp v0.12.0 (https://github.com/Starry-OS/smoltcp.git?rev=7401a54#7401a54b) - https://github.com/smoltcp-rs/smoltcp.git (*)
│   │   │           └── smoltcp feature "socket" (*)
│   │   └── axruntime feature "default"
│   │       └── axruntime v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axruntime) - https://github.com/arceos-org/arceos/tree/main/modules/axruntime
│   │           ├── chrono v0.4.43 - https://github.com/chronotope/chrono (*)
│   │           ├── axbacktrace feature "default" (*)
│   │           ├── axplat feature "default" (*)
│   │           ├── crate_interface feature "default" (*)
│   │           ├── percpu feature "default" (*)
│   │           ├── axdriver feature "default" (*)
│   │           ├── axalloc feature "default" (*)
│   │           ├── axconfig feature "default" (*)
│   │           ├── axhal feature "default" (*)
│   │           ├── axtask feature "default" (*)
│   │           ├── axdisplay feature "default" (*)
│   │           ├── axfs feature "default" (*)
│   │           ├── axinput feature "default" (*)
│   │           ├── axlog feature "default" (*)
│   │           ├── axnet feature "default" (*)
│   │           ├── axmm feature "default"
│   │           │   └── axmm v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axmm) - https://github.com/arceos-org/arceos/tree/main/modules/axmm
│   │           │       ├── log feature "default" (*)
│   │           │       ├── kspin feature "default" (*)
│   │           │       ├── memory_addr feature "default" (*)
│   │           │       ├── enum_dispatch feature "default" (*)
│   │           │       ├── lazyinit feature "default" (*)
│   │           │       ├── axalloc feature "default" (*)
│   │           │       ├── axerrno feature "default" (*)
│   │           │       ├── axconfig feature "default" (*)
│   │           │       ├── axhal feature "default" (*)
│   │           │       ├── axhal feature "paging"
│   │           │       │   └── axhal v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axhal) - https://github.com/arceos-org/arceos/tree/main/modules/axhal (*)
│   │           │       ├── axsync feature "default" (*)
│   │           │       ├── axtask feature "default" (*)
│   │           │       ├── axfs feature "default" (*)
│   │           │       ├── axfs-ng-vfs feature "default" (*)
│   │           │       ├── memory_set feature "axerrno"
│   │           │       │   └── memory_set v0.4.1 - https://github.com/arceos-org/axmm_crates
│   │           │       │       ├── memory_addr feature "default" (*)
│   │           │       │       └── axerrno feature "default" (*)
│   │           │       └── memory_set feature "default"
│   │           │           └── memory_set v0.4.1 - https://github.com/arceos-org/axmm_crates (*)
│   │           ├── ctor_bare feature "default"
│   │           │   └── ctor_bare v0.2.1 - https://github.com/arceos-org/ctor_bare
│   │           │       └── ctor_bare_macros feature "default"
│   │           │           └── ctor_bare_macros v0.2.1 (proc-macro) - https://github.com/arceos-org/ctor_bare
│   │           │               ├── proc-macro2 feature "default" (*)
│   │           │               ├── quote feature "default" (*)
│   │           │               ├── syn feature "default" (*)
│   │           │               └── syn feature "full" (*)
│   │           └── indoc feature "default"
│   │               └── indoc v2.0.7 (proc-macro) - https://github.com/dtolnay/indoc
│   │                   [build-dependencies]
│   │                   └── rustversion feature "default" (*)
│   ├── axalloc feature "slab"
│   │   ├── axalloc v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axalloc) - https://github.com/arceos-org/arceos/tree/main/modules/axalloc (*)
│   │   └── allocator feature "slab"
│   │       └── allocator v0.1.2 (https://github.com/arceos-org/allocator.git?tag=v0.1.2#922e72a7) - https://github.com/arceos-org/allocator (*)
│   └── axfeat feature "axalloc"
│       └── axfeat v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/api/axfeat) - https://github.com/arceos-org/arceos/tree/main/api/axfeat (*)
├── axfeat feature "default"
│   └── axfeat v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/api/axfeat) - https://github.com/arceos-org/arceos/tree/main/api/axfeat (*)
├── axfeat feature "fp-simd"
│   ├── axfeat v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/api/axfeat) - https://github.com/arceos-org/arceos/tree/main/api/axfeat (*)
│   └── axhal feature "fp-simd"
│       ├── axhal v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axhal) - https://github.com/arceos-org/arceos/tree/main/modules/axhal (*)
│       ├── axcpu feature "fp-simd" (*)
│       └── axplat-aarch64-qemu-virt feature "fp-simd"
│           ├── axplat-aarch64-qemu-virt v0.3.0 (https://github.com/kylin-x-kernel/axplat_crates.git?branch=dev#28d9b734) - https://github.com/arceos-org/axplat_crates (*)
│           └── axcpu feature "fp-simd" (*)
├── axfeat feature "fs-ext4"
│   ├── axfeat v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/api/axfeat) - https://github.com/arceos-org/arceos/tree/main/api/axfeat (*)
│   ├── axfeat feature "axfs"
│   │   └── axfeat v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/api/axfeat) - https://github.com/arceos-org/arceos/tree/main/api/axfeat (*)
│   ├── axfeat feature "fs"
│   │   ├── axfeat v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/api/axfeat) - https://github.com/arceos-org/arceos/tree/main/api/axfeat (*)
│   │   ├── axdriver feature "virtio-blk"
│   │   │   ├── axdriver v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axdriver) - https://github.com/arceos-org/arceos/tree/main/modules/axdriver (*)
│   │   │   ├── axdriver feature "axdriver_virtio"
│   │   │   │   └── axdriver v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axdriver) - https://github.com/arceos-org/arceos/tree/main/modules/axdriver (*)
│   │   │   ├── axdriver feature "block" (*)
│   │   │   ├── axdriver feature "virtio"
│   │   │   │   └── axdriver v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axdriver) - https://github.com/arceos-org/arceos/tree/main/modules/axdriver (*)
│   │   │   └── axdriver_virtio feature "block"
│   │   │       ├── axdriver_virtio v0.1.2 (https://github.com/kylin-x-kernel/axdriver_crates.git?branch=dev#cab57c73) - https://github.com/arceos-org/axdriver_crates (*)
│   │   │       └── axdriver_virtio feature "axdriver_block"
│   │   │           └── axdriver_virtio v0.1.2 (https://github.com/kylin-x-kernel/axdriver_crates.git?branch=dev#cab57c73) - https://github.com/arceos-org/axdriver_crates (*)
│   │   ├── axfeat feature "alloc"
│   │   │   ├── axfeat v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/api/axfeat) - https://github.com/arceos-org/arceos/tree/main/api/axfeat (*)
│   │   │   ├── axfeat feature "axalloc" (*)
│   │   │   └── axruntime feature "alloc"
│   │   │       └── axruntime v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axruntime) - https://github.com/arceos-org/arceos/tree/main/modules/axruntime (*)
│   │   ├── axfeat feature "axdriver"
│   │   │   └── axfeat v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/api/axfeat) - https://github.com/arceos-org/arceos/tree/main/api/axfeat (*)
│   │   ├── axfeat feature "paging"
│   │   │   ├── axfeat v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/api/axfeat) - https://github.com/arceos-org/arceos/tree/main/api/axfeat (*)
│   │   │   ├── axhal feature "paging" (*)
│   │   │   ├── axfeat feature "alloc" (*)
│   │   │   └── axruntime feature "paging"
│   │   │       ├── axruntime v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axruntime) - https://github.com/arceos-org/arceos/tree/main/modules/axruntime (*)
│   │   │       └── axhal feature "paging" (*)
│   │   └── axruntime feature "fs"
│   │       └── axruntime v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axruntime) - https://github.com/arceos-org/arceos/tree/main/modules/axruntime (*)
│   └── axfs feature "ext4"
│       └── axfs v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axfs) -  (*)
├── axfeat feature "irq"
│   ├── axfeat v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/api/axfeat) - https://github.com/arceos-org/arceos/tree/main/api/axfeat (*)
│   ├── axdriver feature "irq"
│   │   └── axdriver v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axdriver) - https://github.com/arceos-org/arceos/tree/main/modules/axdriver (*)
│   ├── axhal feature "irq"
│   │   ├── axhal v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axhal) - https://github.com/arceos-org/arceos/tree/main/modules/axhal (*)
│   │   ├── axplat feature "irq" (*)
│   │   └── axplat-aarch64-qemu-virt feature "irq"
│   │       ├── axplat-aarch64-qemu-virt v0.3.0 (https://github.com/kylin-x-kernel/axplat_crates.git?branch=dev#28d9b734) - https://github.com/arceos-org/axplat_crates (*)
│   │       ├── axplat feature "irq" (*)
│   │       └── axplat-aarch64-peripherals feature "irq" (*)
│   ├── axtask feature "irq"
│   │   └── axtask v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axtask) - https://github.com/arceos-org/arceos/tree/main/modules/axtask (*)
│   └── axruntime feature "irq"
│       ├── axruntime v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axruntime) - https://github.com/arceos-org/arceos/tree/main/modules/axruntime (*)
│       ├── axhal feature "irq" (*)
│       └── axtask feature "irq" (*)
├── axfeat feature "multitask"
│   ├── axfeat v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/api/axfeat) - https://github.com/arceos-org/arceos/tree/main/api/axfeat (*)
│   ├── axsync feature "multitask"
│   │   ├── axsync v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axsync) - https://github.com/arceos-org/arceos/tree/main/modules/axsync (*)
│   │   └── axtask feature "multitask"
│   │       └── axtask v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axtask) - https://github.com/arceos-org/arceos/tree/main/modules/axtask (*)
│   ├── axtask feature "multitask" (*)
│   ├── axfeat feature "alloc" (*)
│   ├── axfeat feature "axsync"
│   │   └── axfeat v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/api/axfeat) - https://github.com/arceos-org/arceos/tree/main/api/axfeat (*)
│   ├── axfeat feature "axtask"
│   │   └── axfeat v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/api/axfeat) - https://github.com/arceos-org/arceos/tree/main/api/axfeat (*)
│   └── axruntime feature "multitask"
│       ├── axruntime v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axruntime) - https://github.com/arceos-org/arceos/tree/main/modules/axruntime (*)
│       ├── axtask feature "multitask" (*)
│       └── axruntime feature "axtask"
│           └── axruntime v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axruntime) - https://github.com/arceos-org/arceos/tree/main/modules/axruntime (*)
├── axfeat feature "net"
│   ├── axfeat v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/api/axfeat) - https://github.com/arceos-org/arceos/tree/main/api/axfeat (*)
│   ├── axdriver feature "virtio-net"
│   │   ├── axdriver v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axdriver) - https://github.com/arceos-org/arceos/tree/main/modules/axdriver (*)
│   │   ├── axdriver feature "axdriver_virtio" (*)
│   │   ├── axdriver feature "net" (*)
│   │   ├── axdriver feature "virtio" (*)
│   │   └── axdriver_virtio feature "net"
│   │       ├── axdriver_virtio v0.1.2 (https://github.com/kylin-x-kernel/axdriver_crates.git?branch=dev#cab57c73) - https://github.com/arceos-org/axdriver_crates (*)
│   │       ├── axdriver_virtio feature "alloc"
│   │       │   ├── axdriver_virtio v0.1.2 (https://github.com/kylin-x-kernel/axdriver_crates.git?branch=dev#cab57c73) - https://github.com/arceos-org/axdriver_crates (*)
│   │       │   └── virtio-drivers feature "alloc" (*)
│   │       └── axdriver_virtio feature "axdriver_net"
│   │           └── axdriver_virtio v0.1.2 (https://github.com/kylin-x-kernel/axdriver_crates.git?branch=dev#cab57c73) - https://github.com/arceos-org/axdriver_crates (*)
│   ├── axfeat feature "alloc" (*)
│   ├── axfeat feature "axdriver" (*)
│   ├── axfeat feature "paging" (*)
│   └── axruntime feature "net"
│       └── axruntime v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axruntime) - https://github.com/arceos-org/arceos/tree/main/modules/axruntime (*)
├── axfeat feature "page-alloc-4g"
│   ├── axfeat v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/api/axfeat) - https://github.com/arceos-org/arceos/tree/main/api/axfeat (*)
│   ├── axalloc feature "page-alloc-4g"
│   │   ├── axalloc v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axalloc) - https://github.com/arceos-org/arceos/tree/main/modules/axalloc (*)
│   │   └── allocator feature "page-alloc-4g"
│   │       └── allocator v0.1.2 (https://github.com/arceos-org/allocator.git?tag=v0.1.2#922e72a7) - https://github.com/arceos-org/allocator (*)
│   └── axfeat feature "axalloc" (*)
├── axfeat feature "rtc"
│   ├── axfeat v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/api/axfeat) - https://github.com/arceos-org/arceos/tree/main/api/axfeat (*)
│   ├── axhal feature "rtc"
│   │   ├── axhal v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axhal) - https://github.com/arceos-org/arceos/tree/main/modules/axhal (*)
│   │   └── axplat-aarch64-qemu-virt feature "rtc"
│   │       └── axplat-aarch64-qemu-virt v0.3.0 (https://github.com/kylin-x-kernel/axplat_crates.git?branch=dev#28d9b734) - https://github.com/arceos-org/axplat_crates (*)
│   └── axruntime feature "rtc"
│       └── axruntime v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axruntime) - https://github.com/arceos-org/arceos/tree/main/modules/axruntime (*)
├── axfeat feature "sched-rr"
│   ├── axfeat v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/api/axfeat) - https://github.com/arceos-org/arceos/tree/main/api/axfeat (*)
│   ├── axtask feature "sched-rr"
│   │   ├── axtask v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axtask) - https://github.com/arceos-org/arceos/tree/main/modules/axtask (*)
│   │   ├── axtask feature "multitask" (*)
│   │   └── axtask feature "preempt"
│   │       ├── axtask v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axtask) - https://github.com/arceos-org/arceos/tree/main/modules/axtask (*)
│   │       ├── kernel_guard feature "preempt"
│   │       │   └── kernel_guard v0.1.3 (https://github.com/kylin-x-kernel/kernel_guard?branch=main#58b0f7b2) - https://github.com/arceos-org/kernel_guard (*)
│   │       ├── percpu feature "preempt"
│   │       │   ├── percpu v0.2.0 (https://github.com/arceos-org/percpu?rev=89c8a54#89c8a54c) - https://github.com/arceos-org/percpu (*)
│   │       │   └── percpu_macros feature "preempt"
│   │       │       └── percpu_macros v0.2.0 (proc-macro) (https://github.com/arceos-org/percpu?rev=89c8a54#89c8a54c) - https://github.com/arceos-org/percpu (*)
│   │       ├── axtask feature "irq" (*)
│   │       └── axtask feature "kernel_guard"
│   │           └── axtask v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axtask) - https://github.com/arceos-org/arceos/tree/main/modules/axtask (*)
│   ├── axfeat feature "axtask" (*)
│   └── axfeat feature "irq" (*)
├── axfeat feature "task-ext"
│   ├── axfeat v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/api/axfeat) - https://github.com/arceos-org/arceos/tree/main/api/axfeat (*)
│   ├── axtask feature "task-ext"
│   │   └── axtask v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axtask) - https://github.com/arceos-org/arceos/tree/main/modules/axtask (*)
│   └── axfeat feature "axtask" (*)
├── axfeat feature "uspace"
│   ├── axfeat v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/api/axfeat) - https://github.com/arceos-org/arceos/tree/main/api/axfeat (*)
│   └── axhal feature "uspace"
│       ├── axhal v0.2.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/arceos/modules/axhal) - https://github.com/arceos-org/arceos/tree/main/modules/axhal (*)
│       ├── axcpu feature "uspace"
│       │   └── axcpu v0.3.0 (https://github.com/kylin-x-kernel/axcpu?branch=dev#df212325) - https://github.com/arceos-org/axcpu (*)
│       └── axhal feature "paging" (*)
├── axfs feature "default" (*)
├── axlog feature "default" (*)
├── axruntime feature "default" (*)
├── axplat-riscv64-visionfive2 feature "default"
│   └── axplat-riscv64-visionfive2 v0.3.0 (https://github.com/Starry-OS/axplat-riscv64-visionfive2.git?tag=dev-v03#55c5ea6c) - 
│       ├── axconfig-macros feature "default" (*)
│       ├── axcpu feature "default" (*)
│       ├── log feature "default" (*)
│       ├── kspin feature "default" (*)
│       ├── lazyinit feature "default" (*)
│       ├── uart_16550 feature "default" (*)
│       ├── axplat feature "default"
│       │   └── axplat v0.3.0 (https://github.com/arceos-org/axplat_crates.git?tag=dev-v03#0df0713b) - https://github.com/arceos-org/axplat_crates
│       │       ├── bitflags feature "default" (*)
│       │       ├── const-str feature "default" (*)
│       │       ├── crate_interface feature "default" (*)
│       │       ├── handler_table feature "default" (*)
│       │       ├── kspin feature "default" (*)
│       │       ├── memory_addr feature "default" (*)
│       │       ├── percpu feature "default" (*)
│       │       └── axplat-macros feature "default"
│       │           └── axplat-macros v0.1.0 (proc-macro) (https://github.com/arceos-org/axplat_crates.git?tag=dev-v03#0df0713b) - https://github.com/arceos-org/axplat_crates
│       │               ├── proc-macro2 feature "default" (*)
│       │               ├── quote feature "default" (*)
│       │               ├── syn feature "default" (*)
│       │               └── syn feature "full" (*)
│       ├── riscv feature "default"
│       │   ├── riscv v0.14.0 - https://github.com/rust-embedded/riscv
│       │   │   ├── paste feature "default" (*)
│       │   │   ├── critical-section feature "default"
│       │   │   │   └── critical-section v1.2.0 - https://github.com/rust-embedded/critical-section
│       │   │   ├── embedded-hal feature "default"
│       │   │   │   └── embedded-hal v1.0.0 - https://github.com/rust-embedded/embedded-hal
│       │   │   ├── riscv-macros feature "default"
│       │   │   │   └── riscv-macros v0.2.0 (proc-macro) - https://github.com/rust-embedded/riscv
│       │   │   │       ├── proc-macro2 feature "default" (*)
│       │   │   │       ├── quote feature "default" (*)
│       │   │   │       └── syn feature "default" (*)
│       │   │   └── riscv-pac feature "default"
│       │   │       └── riscv-pac v0.2.0 - https://github.com/rust-embedded/riscv
│       │   └── riscv feature "riscv-macros"
│       │       └── riscv v0.14.0 - https://github.com/rust-embedded/riscv (*)
│       ├── riscv_goldfish feature "default"
│       │   └── riscv_goldfish v0.1.1 - https://github.com/arceos-org/riscv_goldfish
│       ├── riscv_plic feature "default"
│       │   └── riscv_plic v0.2.0 (https://github.com/arceos-org/riscv_plic.git?tag=dev-v02#e2643d2c) - https://github.com/arceos-org/riscv_plic
│       │       └── tock-registers feature "default"
│       │           ├── tock-registers v0.10.1 - https://github.com/tock/tock-registers
│       │           └── tock-registers feature "register_types"
│       │               └── tock-registers v0.10.1 - https://github.com/tock/tock-registers
│       ├── sbi-rt feature "default"
│       │   └── sbi-rt v0.0.3 - https://github.com/rustsbi/rustsbi
│       │       └── sbi-spec feature "default"
│       │           └── sbi-spec v0.0.7 - https://github.com/rustsbi/rustsbi
│       └── sbi-rt feature "legacy"
│           ├── sbi-rt v0.0.3 - https://github.com/rustsbi/rustsbi (*)
│           └── sbi-spec feature "legacy"
│               └── sbi-spec v0.0.7 - https://github.com/rustsbi/rustsbi
├── axplat-riscv64-visionfive2 feature "fp-simd"
│   ├── axplat-riscv64-visionfive2 v0.3.0 (https://github.com/Starry-OS/axplat-riscv64-visionfive2.git?tag=dev-v03#55c5ea6c) -  (*)
│   └── axcpu feature "fp-simd" (*)
├── axplat-riscv64-visionfive2 feature "irq"
│   ├── axplat-riscv64-visionfive2 v0.3.0 (https://github.com/Starry-OS/axplat-riscv64-visionfive2.git?tag=dev-v03#55c5ea6c) -  (*)
│   └── axplat feature "irq"
│       └── axplat v0.3.0 (https://github.com/arceos-org/axplat_crates.git?tag=dev-v03#0df0713b) - https://github.com/arceos-org/axplat_crates (*)
├── axplat-riscv64-visionfive2 feature "rtc"
│   └── axplat-riscv64-visionfive2 v0.3.0 (https://github.com/Starry-OS/axplat-riscv64-visionfive2.git?tag=dev-v03#55c5ea6c) -  (*)
├── starry-api feature "default" (command-line)
│   └── starry-api v0.1.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/api) - https://github.com/arceos-org/starry-next
│       ├── bitmaps v3.2.1 - https://github.com/bodil/bitmaps
│       ├── chrono v0.4.43 - https://github.com/chronotope/chrono (*)
│       ├── event-listener v5.4.1 - https://github.com/smol-rs/event-listener (*)
│       ├── gimli v0.32.3 - https://github.com/gimli-rs/gimli (*)
│       ├── num_enum v0.7.5 - https://github.com/illicitonion/num_enum
│       │   ├── num_enum_derive v0.7.5 (proc-macro) - https://github.com/illicitonion/num_enum
│       │   │   ├── proc-macro2 feature "default" (*)
│       │   │   ├── quote feature "default" (*)
│       │   │   ├── syn feature "default" (*)
│       │   │   ├── syn feature "derive" (*)
│       │   │   ├── syn feature "extra-traits" (*)
│       │   │   └── syn feature "parsing" (*)
│       │   └── rustversion feature "default" (*)
│       ├── rand_chacha v0.3.1 - https://github.com/rust-random/rand
│       │   ├── ppv-lite86 feature "simd"
│       │   │   └── ppv-lite86 v0.2.21 - https://github.com/cryptocorrosion/cryptocorrosion
│       │   │       ├── zerocopy feature "default"
│       │   │       │   └── zerocopy v0.8.33 - https://github.com/google/zerocopy
│       │   │       │       └── zerocopy-derive feature "default"
│       │   │       │           └── zerocopy-derive v0.8.33 (proc-macro) - https://github.com/google/zerocopy
│       │   │       │               ├── proc-macro2 feature "default" (*)
│       │   │       │               ├── quote feature "default" (*)
│       │   │       │               ├── syn feature "default" (*)
│       │   │       │               └── syn feature "full" (*)
│       │   │       └── zerocopy feature "simd"
│       │   │           └── zerocopy v0.8.33 - https://github.com/google/zerocopy (*)
│       │   └── rand_core feature "default"
│       │       └── rand_core v0.6.4 - https://github.com/rust-random/rand
│       ├── slab v0.4.11 - https://github.com/tokio-rs/slab
│       ├── syscalls v0.7.0 (https://github.com/Starry-OS/syscalls.git?rev=6cbfa16#6cbfa162) - https://github.com/jasonwhite/syscalls
│       ├── axplat-aarch64-crosvm-virt feature "default" (command-line) (*)
│       ├── bitflags feature "default" (*)
│       ├── axbacktrace feature "default" (*)
│       ├── cfg-if feature "default" (*)
│       ├── spin feature "default" (*)
│       ├── kspin feature "default" (*)
│       ├── memory_addr feature "default" (*)
│       ├── linkme feature "default" (*)
│       ├── axdriver feature "default" (*)
│       ├── axalloc feature "default" (*)
│       ├── axerrno feature "default" (*)
│       ├── axconfig feature "default" (*)
│       ├── axhal feature "default" (*)
│       ├── axsync feature "default" (*)
│       ├── axtask feature "default" (*)
│       ├── axpoll feature "default" (*)
│       ├── linux-raw-sys feature "general" (*)
│       ├── linux-raw-sys feature "ioctl"
│       │   └── linux-raw-sys v0.11.0 - https://github.com/sunfishcode/linux-raw-sys
│       ├── linux-raw-sys feature "loop_device"
│       │   └── linux-raw-sys v0.11.0 - https://github.com/sunfishcode/linux-raw-sys
│       ├── linux-raw-sys feature "net" (*)
│       ├── linux-raw-sys feature "no_std" (*)
│       ├── linux-raw-sys feature "prctl" (*)
│       ├── linux-raw-sys feature "system" (*)
│       ├── hashbrown feature "default" (*)
│       ├── axfeat feature "alloc-slab" (*)
│       ├── axfeat feature "default" (*)
│       ├── axfeat feature "fp-simd" (*)
│       ├── axfeat feature "fs-ext4" (*)
│       ├── axfeat feature "irq" (*)
│       ├── axfeat feature "multitask" (*)
│       ├── axfeat feature "net" (*)
│       ├── axfeat feature "page-alloc-4g" (*)
│       ├── axfeat feature "rtc" (*)
│       ├── axfeat feature "sched-rr" (*)
│       ├── axfeat feature "task-ext" (*)
│       ├── axfeat feature "uspace" (*)
│       ├── axdisplay feature "default" (*)
│       ├── axfs feature "default" (*)
│       ├── axfs-ng-vfs feature "default" (*)
│       ├── inherit-methods-macro feature "default" (*)
│       ├── axio feature "default" (*)
│       ├── scope-local feature "default" (*)
│       ├── axinput feature "default" (*)
│       ├── axlog feature "default" (*)
│       ├── axnet feature "default" (*)
│       ├── lazy_static feature "default" (*)
│       ├── lazy_static feature "spin_no_std" (*)
│       ├── ringbuf feature "alloc" (*)
│       ├── axmm feature "default" (*)
│       ├── indoc feature "default" (*)
│       ├── bytemuck feature "default"
│       │   └── bytemuck v1.24.0 - https://github.com/Lokathor/bytemuck
│       │       └── bytemuck_derive feature "default"
│       │           └── bytemuck_derive v1.10.2 (proc-macro) - https://github.com/Lokathor/bytemuck
│       │               ├── proc-macro2 feature "default" (*)
│       │               ├── quote feature "default" (*)
│       │               └── syn feature "default" (*)
│       ├── bytemuck feature "unsound_ptr_pod_impl"
│       │   └── bytemuck v1.24.0 - https://github.com/Lokathor/bytemuck (*)
│       ├── downcast-rs feature "sync"
│       │   └── downcast-rs v2.0.2 - https://github.com/marcianx/downcast-rs
│       ├── flatten_objects feature "default"
│       │   └── flatten_objects v0.2.4 - https://github.com/arceos-org/flatten_objects
│       │       └── bitmaps v3.2.1 - https://github.com/bodil/bitmaps
│       ├── rand feature "alloc"
│       │   └── rand v0.9.2 - https://github.com/rust-random/rand
│       │       └── rand_core v0.9.5 - https://github.com/rust-random/rand
│       ├── rand feature "small_rng"
│       │   └── rand v0.9.2 - https://github.com/rust-random/rand (*)
│       ├── zerocopy feature "default" (*)
│       ├── zerocopy feature "derive"
│       │   ├── zerocopy v0.8.33 - https://github.com/google/zerocopy (*)
│       │   └── zerocopy feature "zerocopy-derive"
│       │       └── zerocopy v0.8.33 - https://github.com/google/zerocopy (*)
│       ├── starry-core feature "default" (command-line)
│       │   └── starry-core v0.1.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/core) - https://github.com/arceos-org/starry-next
│       │       ├── event-listener v5.4.1 - https://github.com/smol-rs/event-listener (*)
│       │       ├── ouroboros v0.18.5 - https://github.com/someguynamedjosh/ouroboros
│       │       │   ├── static_assertions feature "default" (*)
│       │       │   ├── aliasable feature "default"
│       │       │   │   ├── aliasable v0.1.3 - https://github.com/avitex/rust-aliasable
│       │       │   │   └── aliasable feature "alloc"
│       │       │   │       └── aliasable v0.1.3 - https://github.com/avitex/rust-aliasable
│       │       │   └── ouroboros_macro feature "default"
│       │       │       └── ouroboros_macro v0.18.5 (proc-macro) - https://github.com/someguynamedjosh/ouroboros
│       │       │           ├── proc-macro2 feature "default" (*)
│       │       │           ├── quote feature "default" (*)
│       │       │           ├── syn feature "default" (*)
│       │       │           ├── syn feature "full" (*)
│       │       │           ├── heck feature "default"
│       │       │           │   └── heck v0.4.1 - https://github.com/withoutboats/heck
│       │       │           └── proc-macro2-diagnostics feature "default"
│       │       │               ├── proc-macro2-diagnostics v0.10.1 - https://github.com/SergioBenitez/proc-macro2-diagnostics
│       │       │               │   ├── proc-macro2 feature "default" (*)
│       │       │               │   ├── quote feature "default" (*)
│       │       │               │   ├── syn feature "default" (*)
│       │       │               │   └── yansi feature "default"
│       │       │               │       ├── yansi v1.0.1 - https://github.com/SergioBenitez/yansi
│       │       │               │       └── yansi feature "std"
│       │       │               │           ├── yansi v1.0.1 - https://github.com/SergioBenitez/yansi
│       │       │               │           └── yansi feature "alloc"
│       │       │               │               └── yansi v1.0.1 - https://github.com/SergioBenitez/yansi
│       │       │               │   [build-dependencies]
│       │       │               │   └── version_check feature "default"
│       │       │               │       └── version_check v0.9.5 - https://github.com/SergioBenitez/version_check
│       │       │               └── proc-macro2-diagnostics feature "colors"
│       │       │                   ├── proc-macro2-diagnostics v0.10.1 - https://github.com/SergioBenitez/proc-macro2-diagnostics (*)
│       │       │                   └── proc-macro2-diagnostics feature "yansi"
│       │       │                       └── proc-macro2-diagnostics v0.10.1 - https://github.com/SergioBenitez/proc-macro2-diagnostics (*)
│       │       ├── slab v0.4.11 - https://github.com/tokio-rs/slab
│       │       ├── bitflags feature "default" (*)
│       │       ├── axbacktrace feature "default" (*)
│       │       ├── cfg-if feature "default" (*)
│       │       ├── spin feature "default" (*)
│       │       ├── lock_api feature "arc_lock"
│       │       │   └── lock_api v0.4.14 - https://github.com/Amanieu/parking_lot (*)
│       │       ├── lock_api feature "default" (*)
│       │       ├── kspin feature "default" (*)
│       │       ├── kernel_guard feature "default" (*)
│       │       ├── memory_addr feature "default" (*)
│       │       ├── percpu feature "default" (*)
│       │       ├── linkme feature "default" (*)
│       │       ├── axerrno feature "default" (*)
│       │       ├── strum feature "derive" (*)
│       │       ├── axconfig feature "default" (*)
│       │       ├── axhal feature "default" (*)
│       │       ├── axsync feature "default" (*)
│       │       ├── axtask feature "default" (*)
│       │       ├── axpoll feature "default" (*)
│       │       ├── linux-raw-sys feature "general" (*)
│       │       ├── linux-raw-sys feature "net" (*)
│       │       ├── linux-raw-sys feature "no_std" (*)
│       │       ├── linux-raw-sys feature "prctl" (*)
│       │       ├── linux-raw-sys feature "system" (*)
│       │       ├── extern-trait feature "default" (*)
│       │       ├── hashbrown feature "default" (*)
│       │       ├── axfeat feature "alloc-slab" (*)
│       │       ├── axfeat feature "default" (*)
│       │       ├── axfeat feature "fp-simd" (*)
│       │       ├── axfeat feature "fs-ext4" (*)
│       │       ├── axfeat feature "irq" (*)
│       │       ├── axfeat feature "multitask" (*)
│       │       ├── axfeat feature "net" (*)
│       │       ├── axfeat feature "page-alloc-4g" (*)
│       │       ├── axfeat feature "rtc" (*)
│       │       ├── axfeat feature "sched-rr" (*)
│       │       ├── axfeat feature "task-ext" (*)
│       │       ├── axfeat feature "uspace" (*)
│       │       ├── axfs feature "default" (*)
│       │       ├── axfs-ng-vfs feature "default" (*)
│       │       ├── inherit-methods-macro feature "default" (*)
│       │       ├── axio feature "default" (*)
│       │       ├── scope-local feature "default" (*)
│       │       ├── axlog feature "default" (*)
│       │       ├── lazy_static feature "default" (*)
│       │       ├── lazy_static feature "spin_no_std" (*)
│       │       ├── axmm feature "default" (*)
│       │       ├── bytemuck feature "default" (*)
│       │       ├── bytemuck feature "derive"
│       │       │   ├── bytemuck v1.24.0 - https://github.com/Lokathor/bytemuck (*)
│       │       │   └── bytemuck feature "bytemuck_derive"
│       │       │       └── bytemuck v1.24.0 - https://github.com/Lokathor/bytemuck (*)
│       │       ├── bytemuck feature "unsound_ptr_pod_impl" (*)
│       │       ├── kernel-elf-parser feature "default"
│       │       │   └── kernel-elf-parser v0.3.3 (https://github.com/Starry-OS/kernel_elf_parser.git?rev=fdcce74#fdcce740) - https://github.com/Starry-OS/kernel-elf-parser
│       │       │       ├── zerocopy feature "default" (*)
│       │       │       ├── zerocopy feature "derive" (*)
│       │       │       ├── xmas-elf feature "default"
│       │       │       │   └── xmas-elf v0.9.1 - https://github.com/nrc/xmas-elf
│       │       │       │       └── zero feature "default"
│       │       │       │           └── zero v0.1.3 - https://github.com/nrc/zero
│       │       │       └── zero feature "default" (*)
│       │       ├── xmas-elf feature "default" (*)
│       │       ├── starry-process feature "default"
│       │       │   └── starry-process v0.2.0 - https://github.com/Starry-OS/starry-process
│       │       │       ├── kspin feature "default" (*)
│       │       │       ├── lazyinit feature "default" (*)
│       │       │       └── weak-map feature "default"
│       │       │           └── weak-map v0.1.2 - https://github.com/Starry-OS/weak-map
│       │       ├── weak-map feature "default" (*)
│       │       ├── starry-signal feature "default"
│       │       │   └── starry-signal v0.2.3 (https://github.com/Starry-OS/starry-signal.git?tag=dev-v02#0597c169) - https://github.com/Starry-OS/starry-signal
│       │       │       ├── event-listener v5.4.1 - https://github.com/smol-rs/event-listener (*)
│       │       │       ├── bitflags feature "default" (*)
│       │       │       ├── axcpu feature "default" (*)
│       │       │       ├── axcpu feature "uspace" (*)
│       │       │       ├── cfg-if feature "default" (*)
│       │       │       ├── log feature "default" (*)
│       │       │       ├── kspin feature "default" (*)
│       │       │       ├── strum feature "derive" (*)
│       │       │       ├── linux-raw-sys feature "general" (*)
│       │       │       ├── linux-raw-sys feature "no_std" (*)
│       │       │       ├── derive_more feature "full"
│       │       │       │   ├── derive_more v2.1.1 - https://github.com/JelteF/derive_more
│       │       │       │   │   └── derive_more-impl feature "default"
│       │       │       │   │       └── derive_more-impl v2.1.1 (proc-macro) - https://github.com/JelteF/derive_more
│       │       │       │   │           ├── proc-macro2 feature "default" (*)
│       │       │       │   │           ├── quote feature "default" (*)
│       │       │       │   │           ├── syn feature "default" (*)
│       │       │       │   │           ├── convert_case feature "default"
│       │       │       │   │           │   └── convert_case v0.10.0 - https://github.com/rutrum/convert-case
│       │       │       │   │           │       └── unicode-segmentation feature "default"
│       │       │       │   │           │           └── unicode-segmentation v1.12.0 - https://github.com/unicode-rs/unicode-segmentation
│       │       │       │   │           └── unicode-xid feature "default"
│       │       │       │   │               └── unicode-xid v0.2.6 - https://github.com/unicode-rs/unicode-xid
│       │       │       │   │           [build-dependencies]
│       │       │       │   │           └── rustc_version feature "default"
│       │       │       │   │               └── rustc_version v0.4.1 - https://github.com/djc/rustc-version-rs
│       │       │       │   │                   └── semver feature "default"
│       │       │       │   │                       ├── semver v1.0.27 - https://github.com/dtolnay/semver
│       │       │       │   │                       └── semver feature "std"
│       │       │       │   │                           └── semver v1.0.27 - https://github.com/dtolnay/semver
│       │       │       │   ├── derive_more feature "add"
│       │       │       │   │   ├── derive_more v2.1.1 - https://github.com/JelteF/derive_more (*)
│       │       │       │   │   └── derive_more-impl feature "add"
│       │       │       │   │       ├── derive_more-impl v2.1.1 (proc-macro) - https://github.com/JelteF/derive_more (*)
│       │       │       │   │       ├── syn feature "extra-traits" (*)
│       │       │       │   │       └── syn feature "visit"
│       │       │       │   │           └── syn v2.0.114 - https://github.com/dtolnay/syn (*)
│       │       │       │   ├── derive_more feature "add_assign"
│       │       │       │   │   ├── derive_more v2.1.1 - https://github.com/JelteF/derive_more (*)
│       │       │       │   │   └── derive_more-impl feature "add_assign"
│       │       │       │   │       ├── derive_more-impl v2.1.1 (proc-macro) - https://github.com/JelteF/derive_more (*)
│       │       │       │   │       ├── syn feature "extra-traits" (*)
│       │       │       │   │       └── syn feature "visit" (*)
│       │       │       │   ├── derive_more feature "as_ref"
│       │       │       │   │   ├── derive_more v2.1.1 - https://github.com/JelteF/derive_more (*)
│       │       │       │   │   └── derive_more-impl feature "as_ref"
│       │       │       │   │       ├── derive_more-impl v2.1.1 (proc-macro) - https://github.com/JelteF/derive_more (*)
│       │       │       │   │       ├── syn feature "extra-traits" (*)
│       │       │       │   │       └── syn feature "visit" (*)
│       │       │       │   ├── derive_more feature "constructor"
│       │       │       │   │   ├── derive_more v2.1.1 - https://github.com/JelteF/derive_more (*)
│       │       │       │   │   └── derive_more-impl feature "constructor"
│       │       │       │   │       └── derive_more-impl v2.1.1 (proc-macro) - https://github.com/JelteF/derive_more (*)
│       │       │       │   ├── derive_more feature "debug"
│       │       │       │   │   ├── derive_more v2.1.1 - https://github.com/JelteF/derive_more (*)
│       │       │       │   │   └── derive_more-impl feature "debug"
│       │       │       │   │       ├── derive_more-impl v2.1.1 (proc-macro) - https://github.com/JelteF/derive_more (*)
│       │       │       │   │       └── syn feature "extra-traits" (*)
│       │       │       │   ├── derive_more feature "deref"
│       │       │       │   │   ├── derive_more v2.1.1 - https://github.com/JelteF/derive_more (*)
│       │       │       │   │   └── derive_more-impl feature "deref"
│       │       │       │   │       └── derive_more-impl v2.1.1 (proc-macro) - https://github.com/JelteF/derive_more (*)
│       │       │       │   ├── derive_more feature "deref_mut"
│       │       │       │   │   ├── derive_more v2.1.1 - https://github.com/JelteF/derive_more (*)
│       │       │       │   │   └── derive_more-impl feature "deref_mut"
│       │       │       │   │       └── derive_more-impl v2.1.1 (proc-macro) - https://github.com/JelteF/derive_more (*)
│       │       │       │   ├── derive_more feature "display"
│       │       │       │   │   ├── derive_more v2.1.1 - https://github.com/JelteF/derive_more (*)
│       │       │       │   │   └── derive_more-impl feature "display"
│       │       │       │   │       ├── derive_more-impl v2.1.1 (proc-macro) - https://github.com/JelteF/derive_more (*)
│       │       │       │   │       └── syn feature "extra-traits" (*)
│       │       │       │   ├── derive_more feature "eq"
│       │       │       │   │   ├── derive_more v2.1.1 - https://github.com/JelteF/derive_more (*)
│       │       │       │   │   └── derive_more-impl feature "eq"
│       │       │       │   │       ├── derive_more-impl v2.1.1 (proc-macro) - https://github.com/JelteF/derive_more (*)
│       │       │       │   │       ├── syn feature "extra-traits" (*)
│       │       │       │   │       └── syn feature "visit" (*)
│       │       │       │   ├── derive_more feature "error"
│       │       │       │   │   ├── derive_more v2.1.1 - https://github.com/JelteF/derive_more (*)
│       │       │       │   │   └── derive_more-impl feature "error"
│       │       │       │   │       ├── derive_more-impl v2.1.1 (proc-macro) - https://github.com/JelteF/derive_more (*)
│       │       │       │   │       └── syn feature "extra-traits" (*)
│       │       │       │   ├── derive_more feature "from"
│       │       │       │   │   ├── derive_more v2.1.1 - https://github.com/JelteF/derive_more (*)
│       │       │       │   │   └── derive_more-impl feature "from"
│       │       │       │   │       ├── derive_more-impl v2.1.1 (proc-macro) - https://github.com/JelteF/derive_more (*)
│       │       │       │   │       └── syn feature "extra-traits" (*)
│       │       │       │   ├── derive_more feature "from_str"
│       │       │       │   │   ├── derive_more v2.1.1 - https://github.com/JelteF/derive_more (*)
│       │       │       │   │   └── derive_more-impl feature "from_str"
│       │       │       │   │       ├── derive_more-impl v2.1.1 (proc-macro) - https://github.com/JelteF/derive_more (*)
│       │       │       │   │       ├── syn feature "full" (*)
│       │       │       │   │       └── syn feature "visit" (*)
│       │       │       │   ├── derive_more feature "index"
│       │       │       │   │   ├── derive_more v2.1.1 - https://github.com/JelteF/derive_more (*)
│       │       │       │   │   └── derive_more-impl feature "index"
│       │       │       │   │       └── derive_more-impl v2.1.1 (proc-macro) - https://github.com/JelteF/derive_more (*)
│       │       │       │   ├── derive_more feature "index_mut"
│       │       │       │   │   ├── derive_more v2.1.1 - https://github.com/JelteF/derive_more (*)
│       │       │       │   │   └── derive_more-impl feature "index_mut"
│       │       │       │   │       └── derive_more-impl v2.1.1 (proc-macro) - https://github.com/JelteF/derive_more (*)
│       │       │       │   ├── derive_more feature "into"
│       │       │       │   │   ├── derive_more v2.1.1 - https://github.com/JelteF/derive_more (*)
│       │       │       │   │   └── derive_more-impl feature "into"
│       │       │       │   │       ├── derive_more-impl v2.1.1 (proc-macro) - https://github.com/JelteF/derive_more (*)
│       │       │       │   │       ├── syn feature "extra-traits" (*)
│       │       │       │   │       └── syn feature "visit-mut" (*)
│       │       │       │   ├── derive_more feature "into_iterator"
│       │       │       │   │   ├── derive_more v2.1.1 - https://github.com/JelteF/derive_more (*)
│       │       │       │   │   └── derive_more-impl feature "into_iterator"
│       │       │       │   │       └── derive_more-impl v2.1.1 (proc-macro) - https://github.com/JelteF/derive_more (*)
│       │       │       │   ├── derive_more feature "is_variant"
│       │       │       │   │   ├── derive_more v2.1.1 - https://github.com/JelteF/derive_more (*)
│       │       │       │   │   └── derive_more-impl feature "is_variant"
│       │       │       │   │       └── derive_more-impl v2.1.1 (proc-macro) - https://github.com/JelteF/derive_more (*)
│       │       │       │   ├── derive_more feature "mul"
│       │       │       │   │   ├── derive_more v2.1.1 - https://github.com/JelteF/derive_more (*)
│       │       │       │   │   └── derive_more-impl feature "mul"
│       │       │       │   │       ├── derive_more-impl v2.1.1 (proc-macro) - https://github.com/JelteF/derive_more (*)
│       │       │       │   │       ├── syn feature "extra-traits" (*)
│       │       │       │   │       └── syn feature "visit" (*)
│       │       │       │   ├── derive_more feature "mul_assign"
│       │       │       │   │   ├── derive_more v2.1.1 - https://github.com/JelteF/derive_more (*)
│       │       │       │   │   └── derive_more-impl feature "mul_assign"
│       │       │       │   │       ├── derive_more-impl v2.1.1 (proc-macro) - https://github.com/JelteF/derive_more (*)
│       │       │       │   │       ├── syn feature "extra-traits" (*)
│       │       │       │   │       └── syn feature "visit" (*)
│       │       │       │   ├── derive_more feature "not"
│       │       │       │   │   ├── derive_more v2.1.1 - https://github.com/JelteF/derive_more (*)
│       │       │       │   │   └── derive_more-impl feature "not"
│       │       │       │   │       ├── derive_more-impl v2.1.1 (proc-macro) - https://github.com/JelteF/derive_more (*)
│       │       │       │   │       └── syn feature "extra-traits" (*)
│       │       │       │   ├── derive_more feature "sum"
│       │       │       │   │   ├── derive_more v2.1.1 - https://github.com/JelteF/derive_more (*)
│       │       │       │   │   └── derive_more-impl feature "sum"
│       │       │       │   │       └── derive_more-impl v2.1.1 (proc-macro) - https://github.com/JelteF/derive_more (*)
│       │       │       │   ├── derive_more feature "try_from"
│       │       │       │   │   ├── derive_more v2.1.1 - https://github.com/JelteF/derive_more (*)
│       │       │       │   │   └── derive_more-impl feature "try_from"
│       │       │       │   │       └── derive_more-impl v2.1.1 (proc-macro) - https://github.com/JelteF/derive_more (*)
│       │       │       │   ├── derive_more feature "try_into"
│       │       │       │   │   ├── derive_more v2.1.1 - https://github.com/JelteF/derive_more (*)
│       │       │       │   │   └── derive_more-impl feature "try_into"
│       │       │       │   │       ├── derive_more-impl v2.1.1 (proc-macro) - https://github.com/JelteF/derive_more (*)
│       │       │       │   │       ├── syn feature "extra-traits" (*)
│       │       │       │   │       ├── syn feature "full" (*)
│       │       │       │   │       └── syn feature "visit-mut" (*)
│       │       │       │   ├── derive_more feature "try_unwrap"
│       │       │       │   │   ├── derive_more v2.1.1 - https://github.com/JelteF/derive_more (*)
│       │       │       │   │   └── derive_more-impl feature "try_unwrap"
│       │       │       │   │       └── derive_more-impl v2.1.1 (proc-macro) - https://github.com/JelteF/derive_more (*)
│       │       │       │   └── derive_more feature "unwrap"
│       │       │       │       ├── derive_more v2.1.1 - https://github.com/JelteF/derive_more (*)
│       │       │       │       └── derive_more-impl feature "unwrap"
│       │       │       │           └── derive_more-impl v2.1.1 (proc-macro) - https://github.com/JelteF/derive_more (*)
│       │       │       └── starry-vm feature "default"
│       │       │           ├── starry-vm v0.2.0 - https://github.com/Starry-OS/starry-vm
│       │       │           │   ├── axerrno feature "default" (*)
│       │       │           │   ├── extern-trait feature "default" (*)
│       │       │           │   ├── bytemuck feature "align_offset"
│       │       │           │   │   └── bytemuck v1.24.0 - https://github.com/Lokathor/bytemuck (*)
│       │       │           │   ├── bytemuck feature "const_zeroed"
│       │       │           │   │   └── bytemuck v1.24.0 - https://github.com/Lokathor/bytemuck (*)
│       │       │           │   ├── bytemuck feature "default" (*)
│       │       │           │   └── bytemuck feature "zeroable_maybe_uninit"
│       │       │           │       └── bytemuck v1.24.0 - https://github.com/Lokathor/bytemuck (*)
│       │       │           └── starry-vm feature "alloc"
│       │       │               └── starry-vm v0.2.0 - https://github.com/Starry-OS/starry-vm (*)
│       │       └── starry-vm feature "default" (*)
│       ├── starry-process feature "default" (*)
│       ├── starry-signal feature "default" (*)
│       ├── starry-vm feature "default" (*)
│       └── tee_raw_sys feature "default"
│           └── tee_raw_sys v0.1.0 (https://github.com/kylin-x-kernel/tee_raw_sys.git#59db449b) - 
├── starry-core feature "default" (command-line) (*)
├── starry-process feature "default" (*)
└── starry-signal feature "default" (*)

starry-api v0.1.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/api) - https://github.com/arceos-org/starry-next (*)

starry-core v0.1.0 (/Users/debin/Desktop/Codes/amd64/StarryOS/core) - https://github.com/arceos-org/starry-next (*)
