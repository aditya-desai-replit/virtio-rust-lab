fn main() {
    println!("virtio-rust-lab: implement one small piece at a time.");
}

#[derive(Debug, PartialEq, Eq, Clone)]
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
    let start = usize::try_from(desc_addr).map_err(|_| QueueError)?;
    let end = start.checked_add(16).ok_or(QueueError)?;
    let bytes = memory.get(start..end).ok_or(QueueError)?;

    let addr = u64::from_le_bytes(bytes[0..8].try_into().map_err(|_| QueueError)?);
    let len = u32::from_le_bytes(bytes[8..12].try_into().map_err(|_| QueueError)?);
    let flags = u16::from_le_bytes(bytes[12..14].try_into().map_err(|_| QueueError)?);
    let next = u16::from_le_bytes(bytes[14..16].try_into().map_err(|_| QueueError)?);
    Ok(Descriptor::new(addr, len, flags, next))
}

fn has_next(d: Descriptor) -> bool {
    (d.flags & 1) == 1
}

fn read_chain(
    memory: &[u8],
    table_addr: u64,
    queue_size: u16,
    head: u16,
) -> Result<Vec<Descriptor>, QueueError> {
    let mut num_seen = 0;
    let mut descriptors = Vec::new();
    let mut d = read_descriptor(memory, table_addr, queue_size, head)?;
    descriptors.push(d.clone());
    num_seen += 1;

    while has_next(d.clone()) && num_seen < queue_size {
        d = read_descriptor(memory, table_addr, queue_size, d.next)?;
        descriptors.push(d.clone());
        num_seen += 1;
    }

    if num_seen == queue_size && has_next(d.clone()) {
        return Err(QueueError);
    }

    Ok(descriptors)
}

fn can_write(d: &Descriptor) -> bool {
    d.flags & 2 == 2
}

fn rng_buffer_range(
    memory_len: usize,
    descriptor: &Descriptor,
) -> Result<std::ops::Range<usize>, QueueError> {
    if !can_write(descriptor) {
        return Err(QueueError);
    }
    let start = usize::try_from(descriptor.addr).map_err(|_| QueueError)?;
    let end = start
        .checked_add(descriptor.len as usize)
        .ok_or(QueueError)?;
    if end > memory_len {
        return Err(QueueError);
    }
    return Ok(start..end);
}

fn pop_available(
    memory: &[u8],
    avail_addr: u64,
    queue_size: u16,
    next_avail: &mut u16,
) -> Result<Option<u16>, QueueError> {
    if 0 == queue_size {
        return Err(QueueError);
    }

    let avail_addr = usize::try_from(avail_addr).map_err(|_| QueueError)?;
    let avail_start = avail_addr.checked_add(4).ok_or(QueueError)?;
    let avail_end = avail_start
        .checked_add((queue_size as usize) * 2)
        .ok_or(QueueError)?;
    let bytes = memory.get(avail_start..avail_end).ok_or(QueueError)?;
    let slot = *next_avail % queue_size;
    let idx = usize::from(slot) * 2;

    let guest_idx = u16::from_le_bytes(
        memory
            .get(avail_addr + 2..avail_addr + 4)
            .ok_or(QueueError)?
            .try_into()
            .map_err(|_| QueueError)?,
    );

    let pending = guest_idx.wrapping_sub(*next_avail);
    if pending == 0 {
        return Ok(None);
    }
    if pending > queue_size {
        return Err(QueueError);
    }

    let fd = u16::from_le_bytes(
        bytes
            .get(idx..idx + 2)
            .ok_or(QueueError)?
            .try_into()
            .map_err(|_| QueueError)?,
    );
    if fd >= queue_size {
        return Err(QueueError);
    }
    *next_avail = next_avail.wrapping_add(1);
    Ok(Some(fd))
}

fn fill_chain(memory: &mut [u8], chain: &[Descriptor], value: u8) -> Result<usize, QueueError> {
    let ranges: Vec<_> = chain
        .iter()
        .map(|d| rng_buffer_range(memory.len(), d))
        .collect::<Result<Vec<_>, QueueError>>()?;
    let filled = ranges.iter().try_fold(0usize, |filled, range| {
        filled.checked_add(range.len()).ok_or(QueueError)
    })?;
    for range in ranges {
        memory[range.clone()].fill(value);
    }
    Ok(filled)
}

#[cfg(test)]
mod descriptor_tests;

#[cfg(test)]
mod chain_tests;

#[cfg(test)]
mod buffer_tests;

#[cfg(test)]
mod available_tests;

#[cfg(test)]
mod fill_tests;
