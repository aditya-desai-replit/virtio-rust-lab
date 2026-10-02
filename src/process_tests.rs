use super::{process_one, Queue};

const TABLE: u64 = 0x100;
const AVAIL: u64 = 0x200;
const USED: u64 = 0x300;
const SIZE: u16 = 4;
const NEXT: u16 = 1;
const WRITE: u16 = 2;
const FILL: u8 = 0xA5;

fn new_memory() -> Vec<u8> {
    vec![0; 0x600]
}

fn queue(next_avail: u16, next_used: u16) -> Queue {
    Queue {
        table_addr: TABLE,
        avail_addr: AVAIL,
        used_addr: USED,
        size: SIZE,
        next_avail,
        next_used,
    }
}

fn put_desc(memory: &mut [u8], index: u16, addr: u64, len: u32, flags: u16, next: u16) {
    let start = TABLE as usize + usize::from(index) * 16;
    memory[start..start + 8].copy_from_slice(&addr.to_le_bytes());
    memory[start + 8..start + 12].copy_from_slice(&len.to_le_bytes());
    memory[start + 12..start + 14].copy_from_slice(&flags.to_le_bytes());
    memory[start + 14..start + 16].copy_from_slice(&next.to_le_bytes());
}

fn offer(memory: &mut [u8], slot: u16, head: u16) {
    let start = AVAIL as usize + 4 + usize::from(slot) * 2;
    memory[start..start + 2].copy_from_slice(&head.to_le_bytes());
}

fn set_avail_idx(memory: &mut [u8], idx: u16) {
    let start = AVAIL as usize + 2;
    memory[start..start + 2].copy_from_slice(&idx.to_le_bytes());
}

fn set_used_idx(memory: &mut [u8], idx: u16) {
    let start = USED as usize + 2;
    memory[start..start + 2].copy_from_slice(&idx.to_le_bytes());
}

fn used_idx(memory: &[u8]) -> u16 {
    let start = USED as usize + 2;
    u16::from_le_bytes(memory[start..start + 2].try_into().unwrap())
}

fn used_entry(memory: &[u8], slot: u16) -> (u32, u32) {
    let start = USED as usize + 4 + usize::from(slot) * 8;
    let id = u32::from_le_bytes(memory[start..start + 4].try_into().unwrap());
    let len = u32::from_le_bytes(memory[start + 4..start + 8].try_into().unwrap());
    (id, len)
}

#[test]
fn empty_queue_returns_false_without_changes() {
    let mut memory = new_memory();
    let before = memory.clone();
    let mut q = queue(0, 0);

    assert!(!process_one(&mut memory, &mut q, FILL).unwrap());
    assert_eq!((q.next_avail, q.next_used), (0, 0));
    assert_eq!(memory, before);
}

#[test]
fn completes_a_single_buffer_request() {
    let mut memory = new_memory();
    put_desc(&mut memory, 3, 0x400, 16, WRITE, 0);
    offer(&mut memory, 0, 3);
    set_avail_idx(&mut memory, 1);
    let mut q = queue(0, 0);

    assert!(process_one(&mut memory, &mut q, FILL).unwrap());
    assert_eq!(&memory[0x400..0x410], &[FILL; 16]);
    assert_eq!(memory[0x3FF], 0);
    assert_eq!(memory[0x410], 0);
    assert_eq!(used_entry(&memory, 0), (3, 16));
    assert_eq!(used_idx(&memory), 1);
    assert_eq!((q.next_avail, q.next_used), (1, 1));
}

#[test]
fn completes_a_chain_with_total_bytes_written() {
    let mut memory = new_memory();
    put_desc(&mut memory, 3, 0x400, 16, NEXT | WRITE, 1);
    put_desc(&mut memory, 1, 0x500, 8, WRITE, 0);
    offer(&mut memory, 0, 3);
    set_avail_idx(&mut memory, 1);
    let mut q = queue(0, 0);

    assert!(process_one(&mut memory, &mut q, FILL).unwrap());
    assert_eq!(&memory[0x400..0x410], &[FILL; 16]);
    assert_eq!(&memory[0x500..0x508], &[FILL; 8]);
    assert_eq!(used_entry(&memory, 0), (3, 24));
    assert_eq!(used_idx(&memory), 1);
}

#[test]
fn processes_one_request_per_call() {
    let mut memory = new_memory();
    put_desc(&mut memory, 2, 0x400, 4, WRITE, 0);
    put_desc(&mut memory, 0, 0x480, 6, WRITE, 0);
    offer(&mut memory, 0, 2);
    offer(&mut memory, 1, 0);
    set_avail_idx(&mut memory, 2);
    let mut q = queue(0, 0);

    assert!(process_one(&mut memory, &mut q, FILL).unwrap());
    assert_eq!(used_idx(&memory), 1);
    assert_eq!(used_entry(&memory, 0), (2, 4));
    assert_eq!((q.next_avail, q.next_used), (1, 1));

    assert!(process_one(&mut memory, &mut q, FILL).unwrap());
    assert_eq!(used_idx(&memory), 2);
    assert_eq!(used_entry(&memory, 1), (0, 6));
    assert_eq!((q.next_avail, q.next_used), (2, 2));

    assert!(!process_one(&mut memory, &mut q, FILL).unwrap());
    assert_eq!((q.next_avail, q.next_used), (2, 2));
}

#[test]
fn wraps_ring_slots_and_keeps_counting() {
    let mut memory = new_memory();
    put_desc(&mut memory, 1, 0x400, 4, WRITE, 0);
    put_desc(&mut memory, 2, 0x480, 4, WRITE, 0);
    offer(&mut memory, 3, 1);
    offer(&mut memory, 0, 2);
    set_avail_idx(&mut memory, 5);
    set_used_idx(&mut memory, 3);
    let mut q = queue(3, 3);

    assert!(process_one(&mut memory, &mut q, FILL).unwrap());
    assert!(process_one(&mut memory, &mut q, FILL).unwrap());
    assert_eq!(used_entry(&memory, 3), (1, 4));
    assert_eq!(used_entry(&memory, 0), (2, 4));
    assert_eq!(used_idx(&memory), 5);
    assert_eq!((q.next_avail, q.next_used), (5, 5));
}

#[test]
fn malformed_cycle_completes_with_zero_bytes() {
    let mut memory = new_memory();
    put_desc(&mut memory, 1, 0x400, 4, NEXT | WRITE, 2);
    put_desc(&mut memory, 2, 0x480, 4, NEXT | WRITE, 1);
    offer(&mut memory, 0, 1);
    set_avail_idx(&mut memory, 1);
    let before = memory.clone();
    let mut q = queue(0, 0);

    assert!(process_one(&mut memory, &mut q, FILL).unwrap());
    assert_eq!(used_entry(&memory, 0), (1, 0));
    assert_eq!(used_idx(&memory), 1);
    assert_eq!((q.next_avail, q.next_used), (1, 1));
    assert_eq!(&memory[0x400..0x600], &before[0x400..0x600]);
}

#[test]
fn invalid_later_buffer_completes_with_zero_bytes_and_no_payload_writes() {
    let mut memory = new_memory();
    put_desc(&mut memory, 0, 0x400, 4, NEXT | WRITE, 1);
    put_desc(&mut memory, 1, 0x5FE, 4, WRITE, 0);
    offer(&mut memory, 0, 0);
    set_avail_idx(&mut memory, 1);
    let before = memory.clone();
    let mut q = queue(0, 0);

    assert!(process_one(&mut memory, &mut q, FILL).unwrap());
    assert_eq!(used_entry(&memory, 0), (0, 0));
    assert_eq!(used_idx(&memory), 1);
    assert_eq!(&memory[0x400..0x600], &before[0x400..0x600]);
}

#[test]
fn read_only_buffer_completes_with_zero_bytes() {
    let mut memory = new_memory();
    put_desc(&mut memory, 0, 0x400, 4, 0, 0);
    offer(&mut memory, 0, 0);
    set_avail_idx(&mut memory, 1);
    let mut q = queue(0, 0);

    assert!(process_one(&mut memory, &mut q, FILL).unwrap());
    assert_eq!(used_entry(&memory, 0), (0, 0));
    assert_eq!(&memory[0x400..0x404], &[0; 4]);
}

#[test]
fn malformed_request_does_not_block_the_next_one() {
    let mut memory = new_memory();
    put_desc(&mut memory, 1, 0x400, 4, NEXT | WRITE, 1);
    put_desc(&mut memory, 3, 0x480, 8, WRITE, 0);
    offer(&mut memory, 0, 1);
    offer(&mut memory, 1, 3);
    set_avail_idx(&mut memory, 2);
    let mut q = queue(0, 0);

    assert!(process_one(&mut memory, &mut q, FILL).unwrap());
    assert!(process_one(&mut memory, &mut q, FILL).unwrap());
    assert_eq!(used_entry(&memory, 0), (1, 0));
    assert_eq!(used_entry(&memory, 1), (3, 8));
    assert_eq!(&memory[0x480..0x488], &[FILL; 8]);
    assert_eq!((q.next_avail, q.next_used), (2, 2));
}

#[test]
fn invalid_available_head_is_fatal() {
    let mut memory = new_memory();
    offer(&mut memory, 0, SIZE);
    set_avail_idx(&mut memory, 1);
    let before = memory.clone();
    let mut q = queue(0, 0);

    assert!(process_one(&mut memory, &mut q, FILL).is_err());
    assert_eq!((q.next_avail, q.next_used), (0, 0));
    assert_eq!(memory, before);
}

#[test]
fn unwritable_used_ring_is_fatal() {
    let mut memory = new_memory();
    put_desc(&mut memory, 0, 0x400, 4, WRITE, 0);
    offer(&mut memory, 0, 0);
    set_avail_idx(&mut memory, 1);
    let mut q = queue(0, 0);
    q.used_addr = 0x5F0;

    assert!(process_one(&mut memory, &mut q, FILL).is_err());
}
