use super::{rng_buffer_range, Descriptor};

const NEXT: u16 = 1;
const WRITE: u16 = 2;

fn descriptor(addr: u64, len: u32, flags: u16) -> Descriptor {
    Descriptor {
        addr,
        len,
        flags,
        next: 0,
    }
}

#[test]
fn accepts_a_writable_buffer_inside_memory() {
    let d = descriptor(0x1000, 16, WRITE);

    assert_eq!(rng_buffer_range(8192, &d).unwrap(), 0x1000..0x1010);
}

#[test]
fn accepts_a_buffer_ending_exactly_at_memory_end() {
    let d = descriptor(48, 16, WRITE);

    assert_eq!(rng_buffer_range(64, &d).unwrap(), 48..64);
}

#[test]
fn accepts_a_buffer_starting_at_zero() {
    let d = descriptor(0, 16, WRITE);

    assert_eq!(rng_buffer_range(64, &d).unwrap(), 0..16);
}

#[test]
fn accepts_write_even_when_next_is_also_set() {
    let d = Descriptor {
        addr: 16,
        len: 8,
        flags: NEXT | WRITE,
        next: 7,
    };

    assert_eq!(rng_buffer_range(64, &d).unwrap(), 16..24);
}

#[test]
fn rejects_a_device_readable_only_buffer() {
    let d = descriptor(16, 8, 0);

    assert!(rng_buffer_range(64, &d).is_err());
}

#[test]
fn next_flag_does_not_grant_write_permission() {
    let d = descriptor(16, 8, NEXT);

    assert!(rng_buffer_range(64, &d).is_err());
}

#[test]
fn rejects_a_buffer_extending_one_byte_past_memory() {
    let d = descriptor(49, 16, WRITE);

    assert!(rng_buffer_range(64, &d).is_err());
}

#[test]
fn rejects_a_nonempty_buffer_starting_at_memory_end() {
    let d = descriptor(64, 1, WRITE);

    assert!(rng_buffer_range(64, &d).is_err());
}

#[test]
fn rejects_address_plus_length_overflow() {
    let d = descriptor(u64::MAX - 7, 16, WRITE);

    assert!(rng_buffer_range(usize::MAX, &d).is_err());
}

#[test]
fn accepts_an_empty_writable_buffer_at_memory_end() {
    let d = descriptor(64, 0, WRITE);

    assert_eq!(rng_buffer_range(64, &d).unwrap(), 64..64);
}

#[test]
fn rejects_an_empty_buffer_starting_beyond_memory() {
    let d = descriptor(65, 0, WRITE);

    assert!(rng_buffer_range(64, &d).is_err());
}

#[test]
fn accepts_an_empty_writable_buffer_in_empty_memory() {
    let d = descriptor(0, 0, WRITE);

    assert_eq!(rng_buffer_range(0, &d).unwrap(), 0..0);
}

#[test]
fn rejects_an_empty_buffer_without_write_permission() {
    let d = descriptor(64, 0, 0);

    assert!(rng_buffer_range(64, &d).is_err());
}

#[test]
fn rejects_a_large_address_instead_of_truncating_it() {
    let d = descriptor(0x1_0000_0010, 16, WRITE);

    assert!(rng_buffer_range(64, &d).is_err());
}
