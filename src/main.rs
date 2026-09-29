fn main() {
    println!("virtio-rust-lab: implement one small piece at a time.");
}

#[derive(Debug, PartialEq, Eq)]
struct Descriptor {
    addr: u64,
    len: u32,
    flags: u16,
    next: u16,
}

impl Descriptor {
    fn new(addr: u64, len: u32, flags: u16, next: u16) -> Self {
        Descriptor {
            addr,
            len,
            flags,
            next,
        }
    }
}

#[derive(Debug)]
struct QueueError;

fn read_descriptor(
    memory: &[u8],
    table_addr: u64,
    queue_size: u16,
    index: u16,
) -> Result<Descriptor, QueueError> {
    if index >= queue_size {
        return Err(QueueError);
    }
    let desc_addr = table_addr
        .checked_add((index as u64).checked_mul(16).ok_or(QueueError)?) 
        .ok_or(QueueError)?;
    let desc_addr = desc_addr as usize;
    let start = usize::try_from(desc_addr).map_err(|_| QueueError)?;
    let end = start.checked_add(16).ok_or(QueueError)?;
    let bytes = memory.get(start..end).ok_or(QueueError)?;

    let addr = u64::from_le_bytes(
        bytes[0..8]
        .try_into()
        .map_err(|_| QueueError)?,
    );
    let len = u32::from_le_bytes(
        bytes[8..12]
        .try_into()
        .map_err(|_| QueueError)?,
    );
    let flags = u16::from_le_bytes(
        bytes[12..14]
        .try_into()
        .map_err(|_| QueueError)?,
    );
    let next = u16::from_le_bytes(
        bytes[14..16]
        .try_into()
        .map_err(|_| QueueError)?,
    );
    Ok(Descriptor::new(addr, len, flags, next))
}

#[cfg(test)]
mod descriptor_tests;
