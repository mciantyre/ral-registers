//! Test that the read, write, modify, and reset macros work with
//! register arrays. This test also demonstrates what a RAL code
//! generator is expected to produce to support register arrays.

#![allow(non_upper_case_globals, non_snake_case)] // Macro conventions.

use ral_registers as ral;

/// A peripheral module.
mod periph {

    #[repr(C)]
    pub struct RegisterBlock {
        /// NEW: A (contiguous) register array is expressed with
        /// a normal Rust array. The code generator selects the
        /// proper register type.
        pub MY_ARRAY: [ral_registers::RWRegister<u32>; 3],
    }

    /// The register module resembles the register module for
    /// a scalar register. The macros distinguish the index
    /// operator from the register name to look up items in
    /// this module.
    pub mod MY_ARRAY {
        pub mod FIELD_A {
            pub const offset: u32 = 0;
            pub const mask: u32 = 0x7F << offset;
            pub mod R {}
            pub mod W {}
            pub mod RW {}
        }
        pub mod FIELD_B {
            pub const offset: u32 = 27;
            pub const mask: u32 = 0b11 << offset;
            pub mod R {}
            pub mod W {}
            pub mod RW {}
        }
    }

    /// The reset value is still expressed as a scalar.
    pub struct ResetValues {
        pub MY_ARRAY: u32,
    }

    pub mod INST {
        pub const reset: super::ResetValues = super::ResetValues { MY_ARRAY: 42 };
    }
}

fn register_block() -> periph::RegisterBlock {
    // Safety: bitpattern of zero is fine.
    use std::mem::MaybeUninit;
    unsafe { MaybeUninit::zeroed().assume_init() }
}

#[test]
fn read_register() {
    let inst = register_block();
    inst.MY_ARRAY[1].write(u32::MAX);

    // Direct read:
    assert_eq!(ral::read_reg!(periph, &inst, MY_ARRAY[1]), u32::MAX);

    // Individual field reads:
    assert_eq!(ral::read_reg!(periph, &inst, MY_ARRAY[1], FIELD_A), 0x7F);
    assert_eq!(ral::read_reg!(periph, &inst, MY_ARRAY[1], FIELD_B), 0b11);

    // Tuple field reads:
    assert_eq!(
        ral::read_reg!(periph, &inst, MY_ARRAY[1], FIELD_A, FIELD_B),
        (0x7F, 0b11)
    );

    // Boolean expressions:
    assert!(ral::read_reg!(periph, &inst, MY_ARRAY[1], FIELD_A == 0x7F));

    // Indices by expression:
    for idx in [0usize, 200] {
        assert_eq!(
            ral::read_reg!(periph, &inst, MY_ARRAY[idx / 100], FIELD_A),
            0
        );
        assert_eq!(
            ral::read_reg!(periph, &inst, MY_ARRAY[idx / 100], FIELD_B),
            0
        );
    }
}

#[should_panic]
#[test]
fn read_register_out_of_bounds() {
    let inst = register_block();
    ral::read_reg!(periph, &inst, MY_ARRAY[42]);
}

#[test]
fn write_register() {
    let inst = register_block();

    // 1:1 write:field:
    ral::write_reg!(periph, &inst, MY_ARRAY[1], FIELD_A: u32::MAX);
    assert_eq!(inst.MY_ARRAY[1].read(), 0x7F);
    ral::write_reg!(periph, &inst, MY_ARRAY[1], FIELD_B: u32::MAX);
    assert_eq!(inst.MY_ARRAY[1].read(), 0b11 << 27);

    // 1:N write:field:
    ral::write_reg!(
        periph,
        &inst,
        MY_ARRAY[1],
        FIELD_A: u32::MAX,
        FIELD_B: u32::MAX
    );
    assert_eq!(inst.MY_ARRAY[1].read(), (0b11 << 27) | 0x7F);

    // Direct write:
    ral::write_reg!(periph, &inst, MY_ARRAY[1], 0xAAAAAAAA);
    assert_eq!(inst.MY_ARRAY[1].read(), 0xAAAAAAAA);

    // Indices by expressions:
    for idx in [|| 0usize, || 2] {
        ral::write_reg!(periph, &inst, MY_ARRAY[idx()], FIELD_A: 1, FIELD_B: 2);
        assert_eq!(
            ral::read_reg!(periph, &inst, MY_ARRAY[idx()], FIELD_A, FIELD_B),
            (1, 2)
        );
    }
}

#[should_panic]
#[test]
fn write_regsiter_out_of_bounds() {
    let inst = register_block();
    ral::write_reg!(
        periph,
        &inst,
        MY_ARRAY[{
            const IDX: usize = 42;
            IDX
        }],
        7
    );
}

#[test]
fn modify_register() {
    let inst = register_block();

    // RMW individual fields:
    ral::modify_reg!(periph, &inst, MY_ARRAY[1], FIELD_A: u32::MAX);
    assert_eq!(inst.MY_ARRAY[1].read(), 0x7F);
    ral::modify_reg!(periph, &inst, MY_ARRAY[1], FIELD_B: u32::MAX);
    assert_eq!(inst.MY_ARRAY[1].read(), 0x7F | (0b11 << 27));

    // RMW multiple fields:
    ral::modify_reg!(periph, &inst, MY_ARRAY[1], FIELD_A: 2, FIELD_B: 2);
    assert_eq!(inst.MY_ARRAY[1].read(), 2 | (2 << 27));

    // RMW whole register:
    ral::modify_reg!(periph, &inst, MY_ARRAY[1], |reg| {
        assert_eq!(reg, 2 | (2 << 27));
        reg | u32::MAX
    });
    assert_eq!(inst.MY_ARRAY[1].read(), u32::MAX);

    // Indices by expression:
    for idx in ["0", "2"] {
        ral::modify_reg!(periph, &inst, MY_ARRAY[idx.parse::<usize>().unwrap()], FIELD_A: 1, FIELD_B: 2);
    }
}

#[should_panic]
#[test]
fn modify_register_out_of_bounds() {
    let inst = register_block();
    ral::modify_reg!(periph, &inst, MY_ARRAY[{ || 42 }()], |_| 0);
}

#[test]
fn reset_register() {
    let inst = register_block();

    // Entire register:
    inst.MY_ARRAY[1].write(u32::MAX);
    ral::reset_reg!(periph, &inst, INST, MY_ARRAY[1]);
    assert_eq!(inst.MY_ARRAY[1].read(), 42);

    // Field in register:
    inst.MY_ARRAY[1].write(u32::MAX);
    ral::reset_reg!(periph, &inst, INST, MY_ARRAY[1], FIELD_B);
    assert_eq!(inst.MY_ARRAY[1].read(), u32::MAX & !(0b11 << 27));
    ral::reset_reg!(periph, &inst, INST, MY_ARRAY[1], FIELD_A);
    assert_eq!(
        inst.MY_ARRAY[1].read(),
        u32::MAX & !(0b11 << 27) & !0x7F | 42
    );

    // Fields in register:
    inst.MY_ARRAY[1].write(u32::MAX);
    ral::reset_reg!(periph, &inst, INST, MY_ARRAY[1], FIELD_B, FIELD_A);
    assert_eq!(
        inst.MY_ARRAY[1].read(),
        u32::MAX & !(0b11 << 27) & !0x7F | 42
    );

    // Indices by expression:
    ral::reset_reg!(periph, &inst, INST, MY_ARRAY[inst.MY_ARRAY.len() - 3]);
    ral::reset_reg!(periph, &inst, INST, MY_ARRAY[inst.MY_ARRAY.len() - 1]);
}

#[should_panic]
#[test]
fn reset_register_out_of_bounds() {
    let inst = register_block();
    ral::reset_reg!(periph, &inst, INST, MY_ARRAY[inst.MY_ARRAY.len()]);
}
