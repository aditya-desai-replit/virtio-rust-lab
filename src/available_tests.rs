use super::pop_available;

const AVAIL_ADDR: u64 = 0x100;
const QUEUE_SIZE: u16 = 4;

fn available_ring(idx: u16, heads: [u16; 4]) -> Vec<u8> {
    let mut memory = vec![0; AVAIL_ADDR as usize + 4 + 2 * QUEUE_SIZE as usize];
    let base = AVAIL_ADDR as usize;
    memory[base + 2..base + 4].copy_from_slice(&idx.to_le_bytes());
    for (slot, head) in heads.iter().enumerate() {
        let start = base + 4 + 2 * slot;
        memory[start..start + 2].copy_from_slice(&head.to_le_bytes());
    }
    memory
}

fn assert_rejected(memory: &[u8], addr: u64, size: u16, initial: u16) {
    let mut next = initial;
    assert!(pop_available(memory, addr, size, &mut next).is_err());
    assert_eq!(next, initial, "an error must not consume an entry");
}

#[test]
fn empty_queue_returns_none_without_advancing() {
    let memory = available_ring(0, [u16::MAX; 4]);
    let mut next = 0;

    assert_eq!(
        pop_available(&memory, AVAIL_ADDR, QUEUE_SIZE, &mut next).unwrap(),
        None
    );
    assert_eq!(next, 0);
}

#[test]
fn pops_only_one_head_per_call() {
    let memory = available_ring(2, [3, 1, 0, 0]);
    let mut next = 0;

    assert_eq!(
        pop_available(&memory, AVAIL_ADDR, QUEUE_SIZE, &mut next).unwrap(),
        Some(3)
    );
    assert_eq!(next, 1);
}

#[test]
fn successive_calls_drain_entries_then_return_none() {
    let memory = available_ring(2, [3, 1, 0, 0]);
    let mut next = 0;

    assert_eq!(
        pop_available(&memory, AVAIL_ADDR, QUEUE_SIZE, &mut next).unwrap(),
        Some(3)
    );
    assert_eq!(next, 1);
    assert_eq!(
        pop_available(&memory, AVAIL_ADDR, QUEUE_SIZE, &mut next).unwrap(),
        Some(1)
    );
    assert_eq!(next, 2);
    assert_eq!(
        pop_available(&memory, AVAIL_ADDR, QUEUE_SIZE, &mut next).unwrap(),
        None
    );
    assert_eq!(next, 2);
}

#[test]
fn uses_counter_modulo_queue_size_to_select_slot() {
    let memory = available_ring(6, [0, 3, 0, 0]);
    let mut next = 5;

    assert_eq!(
        pop_available(&memory, AVAIL_ADDR, QUEUE_SIZE, &mut next).unwrap(),
        Some(3)
    );
    assert_eq!(next, 6);
}

#[test]
fn wraps_from_last_ring_slot_to_first() {
    let memory = available_ring(5, [1, 0, 0, 2]);
    let mut next = 3;

    assert_eq!(
        pop_available(&memory, AVAIL_ADDR, QUEUE_SIZE, &mut next).unwrap(),
        Some(2)
    );
    assert_eq!(next, 4);
    assert_eq!(
        pop_available(&memory, AVAIL_ADDR, QUEUE_SIZE, &mut next).unwrap(),
        Some(1)
    );
    assert_eq!(next, 5);
}

#[test]
fn wraps_u16_counter_and_pending_count() {
    let memory = available_ring(1, [2, 0, 0, 3]);
    let mut next = u16::MAX;

    assert_eq!(
        pop_available(&memory, AVAIL_ADDR, QUEUE_SIZE, &mut next).unwrap(),
        Some(3)
    );
    assert_eq!(next, 0);
    assert_eq!(
        pop_available(&memory, AVAIL_ADDR, QUEUE_SIZE, &mut next).unwrap(),
        Some(2)
    );
    assert_eq!(next, 1);
    assert_eq!(
        pop_available(&memory, AVAIL_ADDR, QUEUE_SIZE, &mut next).unwrap(),
        None
    );
}

#[test]
fn accepts_exactly_queue_size_pending_entries() {
    let memory = available_ring(4, [3, 2, 1, 0]);
    let mut next = 0;

    assert_eq!(
        pop_available(&memory, AVAIL_ADDR, QUEUE_SIZE, &mut next).unwrap(),
        Some(3)
    );
    assert_eq!(next, 1);
}

#[test]
fn rejects_more_pending_entries_than_capacity() {
    let memory = available_ring(10, [0; 4]);
    assert_rejected(&memory, AVAIL_ADDR, QUEUE_SIZE, 5);
}

#[test]
fn rejects_an_out_of_range_head_without_advancing() {
    let memory = available_ring(6, [0, QUEUE_SIZE, 0, 0]);
    assert_rejected(&memory, AVAIL_ADDR, QUEUE_SIZE, 5);
}

#[test]
fn rejects_zero_queue_size() {
    let memory = available_ring(0, [0; 4]);
    assert_rejected(&memory, AVAIL_ADDR, 0, 0);
}

#[test]
fn rejects_a_truncated_guest_idx() {
    let mut memory = available_ring(0, [0; 4]);
    memory.truncate(AVAIL_ADDR as usize + 3);
    assert_rejected(&memory, AVAIL_ADDR, QUEUE_SIZE, 0);
}

#[test]
fn rejects_a_truncated_selected_entry_without_advancing() {
    let mut memory = available_ring(4, [0, 0, 0, 2]);
    memory.pop();
    assert_rejected(&memory, AVAIL_ADDR, QUEUE_SIZE, 3);
}

#[test]
fn rejects_an_unmapped_available_ring() {
    assert_rejected(&[0; 16], AVAIL_ADDR, QUEUE_SIZE, 0);
}

#[test]
fn rejects_address_overflow_without_panicking() {
    assert_rejected(&[0; 16], u64::MAX - 1, QUEUE_SIZE, 0);
}

#[test]
fn rejects_large_addresses_instead_of_truncating_them() {
    let memory = available_ring(1, [2, 0, 0, 0]);
    assert_rejected(&memory, 0x1_0000_0100, QUEUE_SIZE, 0);
}

#[test]
fn notification_flags_do_not_hide_available_work() {
    let mut memory = available_ring(1, [2, 0, 0, 0]);
    let base = AVAIL_ADDR as usize;
    memory[base..base + 2].copy_from_slice(&1u16.to_le_bytes());
    let mut next = 0;

    assert_eq!(
        pop_available(&memory, AVAIL_ADDR, QUEUE_SIZE, &mut next).unwrap(),
        Some(2)
    );
    assert_eq!(next, 1);
}

#[test]
fn reads_idx_and_head_as_little_endian_u16_values() {
    let queue_size = 512;
    let base = AVAIL_ADDR as usize;
    let mut memory = vec![0; base + 4 + 2 * usize::from(queue_size)];
    memory[base + 2..base + 4].copy_from_slice(&257u16.to_le_bytes());
    let slot = base + 4 + 2 * 256;
    memory[slot..slot + 2].copy_from_slice(&300u16.to_le_bytes());
    let mut next = 256;

    assert_eq!(
        pop_available(&memory, AVAIL_ADDR, queue_size, &mut next).unwrap(),
        Some(300)
    );
    assert_eq!(next, 257);
}
