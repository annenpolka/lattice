//! Committed `stdlib/lattice-stdlib.wasm` must match the checked-in source stamp.

fn fnv_hex(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    format!("{hash:016x}")
}

fn stamp_value(stamp: &str, key: &str) -> String {
    for line in stamp.lines() {
        if let Some((k, v)) = line.split_once('=')
            && k.trim() == key
        {
            return v.trim().to_string();
        }
    }
    panic!("missing {key} in stdlib stamp");
}

#[test]
fn vendored_wasm_matches_source_stamp() {
    let stamp = include_str!("../../../stdlib/lattice-stdlib.stamp");
    let wasm = include_bytes!("../../../stdlib/lattice-stdlib.wasm");
    let guest = include_str!("../../../crates/lattice-stdlib-guest/src/lib.rs");
    let wit = include_str!("../../../wit/lattice/stdlib.wit");
    assert_eq!(fnv_hex(wasm), stamp_value(stamp, "wasm"));
    assert_eq!(fnv_hex(guest.as_bytes()), stamp_value(stamp, "guest"));
    assert_eq!(fnv_hex(wit.as_bytes()), stamp_value(stamp, "wit"));
}
