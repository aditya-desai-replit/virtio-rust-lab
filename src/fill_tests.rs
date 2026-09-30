use super::{fill_chain, Descriptor};

const WRITE: u16 = 2;

fn writable(addr: u64, len: u32) -> Descriptor {
    Descriptor::new(addr, len, WRITE, 0)
}

fn assert_rejected_without_writes(chain: &[Descriptor]) {
    let mut memory: Vec<u8> = (0..16).collect();
    let before = memory.clone();

    assert!(fill_chain(&mut memory, chain, 0xA5).is_err());
    assert_eq!(memory, before, "an error must leave all memory unchanged");
}

#[test]
fn fills_one_buffer_and_preserves_surrounding_bytes() {
    let mut memory = [0xCC; 8];
    let chain = [writable(2, 3)];

    let written = fill_chain(&mut memory, &chain, 0xA5).unwrap();

    assert_eq!(written, 3);
    assert_eq!(memory, [0xCC, 0xCC, 0xA5, 0xA5, 0xA5, 0xCC, 0xCC, 0xCC]);
}

#[test]
fn fills_two_separate_buffers_and_returns_their_total_length() {
    let mut memory = [0; 12];
    let chain = [writable(2, 3), writable(8, 2)];

    let written = fill_chain(&mut memory, &chain, 0xA5).unwrap();

    assert_eq!(written, 5);
    assert_eq!(memory, [0, 0, 0xA5, 0xA5, 0xA5, 0, 0, 0, 0xA5, 0xA5, 0, 0]);
}

#[test]
fn uses_the_supplied_value_including_zero() {
    for value in [0, 0x3C, 0xFF] {
        let mut memory = [0xCC; 4];
        let written = fill_chain(&mut memory, &[writable(1, 2)], value).unwrap();

        assert_eq!(written, 2);
        assert_eq!(memory, [0xCC, value, value, 0xCC]);
    }
}

#[test]
fn accepts_a_buffer_ending_exactly_at_memory_end() {
    let mut memory = [0; 8];

    let written = fill_chain(&mut memory, &[writable(4, 4)], 0x7E).unwrap();

    assert_eq!(written, 4);
    assert_eq!(memory, [0, 0, 0, 0, 0x7E, 0x7E, 0x7E, 0x7E]);
}

#[test]
fn empty_chain_returns_zero_without_changes() {
    let mut memory = [0xCC; 8];

    assert_eq!(fill_chain(&mut memory, &[], 0xA5).unwrap(), 0);
    assert_eq!(memory, [0xCC; 8]);
}

#[test]
fn zero_length_buffer_does_not_stop_later_buffers() {
    let mut memory = [0; 8];
    let chain = [writable(8, 0), writable(2, 2)];

    assert_eq!(fill_chain(&mut memory, &chain, 0xA5).unwrap(), 2);
    assert_eq!(memory, [0, 0, 0xA5, 0xA5, 0, 0, 0, 0]);
}

#[test]
fn accepts_an_empty_writable_buffer_in_empty_memory() {
    let mut memory = [];

    assert_eq!(fill_chain(&mut memory, &[writable(0, 0)], 0xA5).unwrap(), 0);
}

#[test]
fn does_not_follow_next_in_an_already_decoded_chain() {
    let mut memory = [0; 4];
    let chain = [Descriptor::new(1, 2, WRITE | 1, u16::MAX)];

    assert_eq!(fill_chain(&mut memory, &chain, 0xA5).unwrap(), 2);
    assert_eq!(memory, [0, 0xA5, 0xA5, 0]);
}

#[test]
fn rejects_a_read_only_buffer_without_writes() {
    assert_rejected_without_writes(&[Descriptor::new(0, 4, 0, 0)]);
}

#[test]
fn invalid_later_buffer_leaves_earlier_valid_buffer_unchanged() {
    assert_rejected_without_writes(&[writable(0, 4), writable(15, 2)]);
}

#[test]
fn read_only_later_buffer_leaves_earlier_valid_buffer_unchanged() {
    let chain = [writable(0, 4), Descriptor::new(8, 2, 0, 0)];
    assert_rejected_without_writes(&chain);
}

#[test]
fn overflowing_later_buffer_leaves_earlier_valid_buffer_unchanged() {
    assert_rejected_without_writes(&[writable(0, 4), writable(u64::MAX - 7, 16)]);
}

#[test]
fn invalid_empty_later_buffer_is_not_silently_skipped() {
    assert_rejected_without_writes(&[writable(0, 4), writable(17, 0)]);
}
