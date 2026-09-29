use super::{read_chain, Descriptor};

const TABLE_ADDR: u64 = 0x100;
const QUEUE_SIZE: u16 = 8;
const NEXT: u16 = 1;
const WRITE: u16 = 2;

fn guest_memory() -> Vec<u8> {
    vec![0; TABLE_ADDR as usize + QUEUE_SIZE as usize * 16]
}

fn put_descriptor(memory: &mut [u8], index: u16, flags: u16, next: u16) -> Descriptor {
    let descriptor = Descriptor {
        addr: 0x1000 + u64::from(index) * 0x100,
        len: 16 + u32::from(index),
        flags,
        next,
    };
    let start = TABLE_ADDR as usize + usize::from(index) * 16;
    let bytes = &mut memory[start..start + 16];
    bytes[0..8].copy_from_slice(&descriptor.addr.to_le_bytes());
    bytes[8..12].copy_from_slice(&descriptor.len.to_le_bytes());
    bytes[12..14].copy_from_slice(&descriptor.flags.to_le_bytes());
    bytes[14..16].copy_from_slice(&descriptor.next.to_le_bytes());
    descriptor
}

#[test]
fn reads_a_single_descriptor() {
    let mut memory = guest_memory();
    let expected = put_descriptor(&mut memory, 7, WRITE, 0);

    let chain = read_chain(&memory, TABLE_ADDR, QUEUE_SIZE, 7).unwrap();

    assert_eq!(chain, vec![expected]);
}

#[test]
fn ignores_garbage_next_when_next_flag_is_unset() {
    let mut memory = guest_memory();
    let expected = put_descriptor(&mut memory, 2, WRITE, u16::MAX);

    let chain = read_chain(&memory, TABLE_ADDR, QUEUE_SIZE, 2).unwrap();

    assert_eq!(chain, vec![expected]);
}

#[test]
fn follows_noncontiguous_indices_in_chain_order() {
    let mut memory = guest_memory();
    let first = put_descriptor(&mut memory, 7, NEXT, 2);
    let second = put_descriptor(&mut memory, 2, NEXT | WRITE, 5);
    let third = put_descriptor(&mut memory, 5, WRITE, 0);

    let chain = read_chain(&memory, TABLE_ADDR, QUEUE_SIZE, 7).unwrap();

    assert_eq!(chain, vec![first, second, third]);
}

#[test]
fn accepts_a_terminating_chain_of_exactly_queue_size_descriptors() {
    let mut memory = guest_memory();
    let expected: Vec<_> = (0..QUEUE_SIZE)
        .map(|index| {
            let flags = if index + 1 < QUEUE_SIZE { NEXT } else { 0 };
            put_descriptor(&mut memory, index, flags, index + 1)
        })
        .collect();

    let chain = read_chain(&memory, TABLE_ADDR, QUEUE_SIZE, 0).unwrap();

    assert_eq!(chain, expected);
}

#[test]
fn rejects_an_out_of_range_head() {
    let memory = guest_memory();

    assert!(read_chain(&memory, TABLE_ADDR, QUEUE_SIZE, QUEUE_SIZE).is_err());
}

#[test]
fn rejects_an_out_of_range_next_index() {
    let mut memory = guest_memory();
    put_descriptor(&mut memory, 0, NEXT, QUEUE_SIZE);

    assert!(read_chain(&memory, TABLE_ADDR, QUEUE_SIZE, 0).is_err());
}

#[test]
fn rejects_a_self_loop() {
    let mut memory = guest_memory();
    put_descriptor(&mut memory, 7, NEXT, 7);

    assert!(read_chain(&memory, TABLE_ADDR, QUEUE_SIZE, 7).is_err());
}

#[test]
fn rejects_a_two_descriptor_cycle() {
    let mut memory = guest_memory();
    put_descriptor(&mut memory, 7, NEXT, 2);
    put_descriptor(&mut memory, 2, NEXT, 7);

    assert!(read_chain(&memory, TABLE_ADDR, QUEUE_SIZE, 7).is_err());
}

#[test]
fn rejects_a_full_ring_cycle() {
    let mut memory = guest_memory();
    for index in 0..QUEUE_SIZE {
        put_descriptor(&mut memory, index, NEXT, (index + 1) % QUEUE_SIZE);
    }

    assert!(read_chain(&memory, TABLE_ADDR, QUEUE_SIZE, 0).is_err());
}

#[test]
fn propagates_a_truncated_tail_descriptor_error() {
    let mut memory = guest_memory();
    put_descriptor(&mut memory, 0, NEXT, 7);
    put_descriptor(&mut memory, 7, WRITE, 0);
    memory.pop();

    assert!(read_chain(&memory, TABLE_ADDR, QUEUE_SIZE, 0).is_err());
}

#[test]
fn rejects_a_head_in_a_zero_capacity_queue() {
    let memory = guest_memory();

    assert!(read_chain(&memory, TABLE_ADDR, 0, 0).is_err());
}
