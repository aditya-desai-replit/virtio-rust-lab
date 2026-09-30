use super::push_used;

const USED_ADDR: u64 = 0x100;
const QUEUE_SIZE: u16 = 4;

fn guest_memory(counter: u16) -> Vec<u8> {
    let base = USED_ADDR as usize;
    let mut memory = vec![0xCC; base + 4 + usize::from(QUEUE_SIZE) * 8 + 8];
    memory[base..base + 2].copy_from_slice(&0x1234u16.to_le_bytes());
    memory[base + 2..base + 4].copy_from_slice(&counter.to_le_bytes());
    memory
}

fn expect_completion(memory: &mut [u8], initial: u16, head: u16, written: u32) {
    let mut expected = memory.to_vec();
    let base = USED_ADDR as usize;
    let entry = base + 4 + usize::from(initial % QUEUE_SIZE) * 8;
    let published = initial.wrapping_add(1);
    expected[entry..entry + 4].copy_from_slice(&u32::from(head).to_le_bytes());
    expected[entry + 4..entry + 8].copy_from_slice(&written.to_le_bytes());
    expected[base + 2..base + 4].copy_from_slice(&published.to_le_bytes());
    let mut next = initial;

    push_used(memory, USED_ADDR, QUEUE_SIZE, &mut next, head, written).unwrap();

    assert_eq!(next, published);
    assert_eq!(
        memory,
        expected.as_slice(),
        "only the selected entry and header idx may change"
    );
}

fn expect_rejection(memory: &mut [u8], addr: u64, size: u16, initial: u16, head: u16) {
    let before = memory.to_vec();
    let mut next = initial;

    assert!(push_used(memory, addr, size, &mut next, head, 16).is_err());
    assert_eq!(next, initial, "an error must leave the counter unchanged");
    assert_eq!(
        memory,
        before.as_slice(),
        "an error must not write anything"
    );
}

#[test]
fn publishes_first_completion_in_little_endian() {
    let mut memory = guest_memory(0);

    expect_completion(&mut memory, 0, 3, 0x1234_5678);

    let base = USED_ADDR as usize;
    assert_eq!(
        &memory[base + 4..base + 12],
        &[3, 0, 0, 0, 0x78, 0x56, 0x34, 0x12]
    );
    assert_eq!(&memory[base + 2..base + 4], &[1, 0]);
}

#[test]
fn slot_one_can_complete_descriptor_three_after_five_completions() {
    let mut memory = guest_memory(5);

    expect_completion(&mut memory, 5, 3, 16);
}

#[test]
fn successive_completions_preserve_previous_entries() {
    let mut memory = guest_memory(0);

    expect_completion(&mut memory, 0, 3, 16);
    expect_completion(&mut memory, 1, 0, 8);
}

#[test]
fn wraps_from_last_ring_slot_to_first_without_resetting_counter() {
    let mut memory = guest_memory(3);

    expect_completion(&mut memory, 3, 2, 16);
    expect_completion(&mut memory, 4, 1, 32);
}

#[test]
fn wraps_u16_counter_to_zero() {
    let mut memory = guest_memory(u16::MAX);

    expect_completion(&mut memory, u16::MAX, 2, 16);
    expect_completion(&mut memory, 0, 3, 8);
}

#[test]
fn publishes_both_bytes_of_the_counter() {
    let mut memory = guest_memory(255);

    expect_completion(&mut memory, 255, 0, 1);

    let base = USED_ADDR as usize;
    assert_eq!(&memory[base + 2..base + 4], &[0, 1]);
}

#[test]
fn zero_bytes_written_is_still_a_completion() {
    let mut memory = guest_memory(0);

    expect_completion(&mut memory, 0, 1, 0);
}

#[test]
fn preserves_the_full_u32_written_count() {
    let mut memory = guest_memory(0);

    expect_completion(&mut memory, 0, 2, u32::MAX);
}

#[test]
fn accepts_an_entry_ending_exactly_at_memory_end() {
    let mut memory = guest_memory(3);
    memory.truncate(USED_ADDR as usize + 4 + usize::from(QUEUE_SIZE) * 8);

    expect_completion(&mut memory, 3, 0, 16);
}

#[test]
fn rejects_zero_queue_size_without_changes() {
    expect_rejection(&mut guest_memory(0), USED_ADDR, 0, 0, 0);
}

#[test]
fn rejects_head_equal_to_queue_size_without_changes() {
    expect_rejection(&mut guest_memory(5), USED_ADDR, QUEUE_SIZE, 5, QUEUE_SIZE);
}

#[test]
fn rejects_head_above_queue_size_without_changes() {
    expect_rejection(&mut guest_memory(5), USED_ADDR, QUEUE_SIZE, 5, u16::MAX);
}

#[test]
fn rejects_truncated_idx_without_changes() {
    let mut memory = guest_memory(0);
    memory.truncate(USED_ADDR as usize + 3);

    expect_rejection(&mut memory, USED_ADDR, QUEUE_SIZE, 0, 1);
}

#[test]
fn truncated_len_field_does_not_partially_write_the_id() {
    let mut memory = guest_memory(3);
    memory.truncate(USED_ADDR as usize + 4 + 3 * 8 + 7);

    expect_rejection(&mut memory, USED_ADDR, QUEUE_SIZE, 3, 1);
}

#[test]
fn rejects_an_unmapped_ring_without_changes() {
    expect_rejection(&mut [0xCC; 16], USED_ADDR, QUEUE_SIZE, 0, 1);
}

#[test]
fn rejects_address_overflow_without_changes() {
    expect_rejection(&mut guest_memory(0), u64::MAX - 1, QUEUE_SIZE, 0, 1);
}

#[test]
fn rejects_large_addresses_instead_of_truncating_them() {
    expect_rejection(&mut guest_memory(0), 0x1_0000_0100, QUEUE_SIZE, 0, 1);
}
