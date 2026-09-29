use super::{read_descriptor, Descriptor};

fn encode(descriptor: &Descriptor) -> [u8; 16] {
    let mut bytes = [0; 16];
    bytes[0..8].copy_from_slice(&descriptor.addr.to_le_bytes());
    bytes[8..12].copy_from_slice(&descriptor.len.to_le_bytes());
    bytes[12..14].copy_from_slice(&descriptor.flags.to_le_bytes());
    bytes[14..16].copy_from_slice(&descriptor.next.to_le_bytes());
    bytes
}

#[test]
fn reads_slot_seven_at_table_base_plus_seven_times_sixteen() {
    let expected = Descriptor {
        addr: 0x1000,
        len: 16,
        flags: 2,
        next: 0,
    };
    let mut memory = vec![0; 0x180];
    memory[0x170..0x180].copy_from_slice(&encode(&expected));

    let actual = read_descriptor(&memory, 0x100, 8, 7)
        .expect("descriptor 7 fits in guest memory; payload validation comes later");

    assert_eq!(actual, expected);
}

#[test]
fn decodes_every_field_as_little_endian_without_following_next() {
    let memory = [
        0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x01, 0x78, 0x56, 0x34, 0x12, 0x03, 0x00, 0x34,
        0x12,
    ];
    let expected = Descriptor {
        addr: 0x0123_4567_89ab_cdef,
        len: 0x1234_5678,
        flags: 3,
        next: 0x1234,
    };

    let actual = read_descriptor(&memory, 0, 1, 0)
        .expect("decode fields only; do not validate the payload or follow NEXT yet");

    assert_eq!(actual, expected);
}

#[test]
fn accepts_a_descriptor_ending_exactly_at_memory_end() {
    let memory = [0; 16];
    let actual = read_descriptor(&memory, 0, 1, 0).expect("all 16 bytes are present");

    assert_eq!(
        actual,
        Descriptor {
            addr: 0,
            len: 0,
            flags: 0,
            next: 0,
        }
    );
}

#[test]
fn rejects_index_equal_to_queue_size_even_when_memory_is_large_enough() {
    let memory = [0; 256];
    assert!(read_descriptor(&memory, 0, 8, 8).is_err());
}

#[test]
fn rejects_index_greater_than_queue_size() {
    let memory = [0; 256];
    assert!(read_descriptor(&memory, 0, 8, 9).is_err());
}

#[test]
fn rejects_a_descriptor_missing_its_last_byte() {
    let memory = [0; 15];
    assert!(read_descriptor(&memory, 0, 1, 0).is_err());
}

#[test]
fn rejects_a_table_starting_beyond_guest_memory() {
    let memory = [0; 32];
    assert!(read_descriptor(&memory, 64, 1, 0).is_err());
}

#[test]
fn rejects_overflow_when_locating_an_indexed_descriptor() {
    let memory = [0; 32];
    assert!(read_descriptor(&memory, u64::MAX - 15, 2, 1).is_err());
}

#[test]
fn rejects_overflow_when_calculating_the_descriptor_end() {
    let memory = [0; 32];
    assert!(read_descriptor(&memory, u64::MAX - 7, 1, 0).is_err());
}
