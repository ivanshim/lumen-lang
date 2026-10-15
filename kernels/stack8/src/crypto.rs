use sha2::Digest;
use sha3::digest::{ExtendableOutput, Update, XofReader};

pub fn hash(name: &str, data: &[u8], size: usize, numbers: &[u64], key: &[u8], salt: &[u8], person: &[u8]) -> Result<Vec<u8>, String> {
    if numbers.len() != 7 { return Err("TypeError: expected seven tree parameters".into()); }
    if name.starts_with("blake2") {
        let max = if name == "blake2b" { 64 } else { 32 };
        if size == 0 || size > max || key.len() > max || salt.len() > max / 4 || person.len() > max / 4
            || numbers[0] > 255 || numbers[1] == 0 || numbers[1] > 255 || numbers[2] > u32::MAX as u64
            || numbers[4] > 255 || numbers[5] > max as u64 || (max == 32 && numbers[3] > 0xffffffffffff) {
            return Err("ValueError: invalid BLAKE2 parameters".into());
        }
    }
    if name.starts_with("shake_") && size >= 1 << 29 { return Err("ValueError: length is too large".into()); }
    let digest = match name {
        "md5" => md5::Md5::digest(data).to_vec(),
        "sha1" => sha1::Sha1::digest(data).to_vec(),
        "sha224" => sha2::Sha224::digest(data).to_vec(),
        "sha256" => sha2::Sha256::digest(data).to_vec(),
        "sha384" => sha2::Sha384::digest(data).to_vec(),
        "sha512" => sha2::Sha512::digest(data).to_vec(),
        "sha3_224" => sha3::Sha3_224::digest(data).to_vec(),
        "sha3_256" => sha3::Sha3_256::digest(data).to_vec(),
        "sha3_384" => sha3::Sha3_384::digest(data).to_vec(),
        "sha3_512" => sha3::Sha3_512::digest(data).to_vec(),
        "shake_128" => {
            let mut state = sha3::Shake128::default();
            state.update(data);
            let mut reader = state.finalize_xof();
            let mut result = vec![0; size];
            reader.read(&mut result);
            result
        }
        "shake_256" => {
            let mut state = sha3::Shake256::default();
            state.update(data);
            let mut reader = state.finalize_xof();
            let mut result = vec![0; size];
            reader.read(&mut result);
            result
        }
        "blake2b" => {
            let mut setup = blake2b_simd::Params::new();
            setup.hash_length(size).key(key).salt(salt).personal(person)
                .fanout(numbers[0] as u8).max_depth(numbers[1] as u8)
                .max_leaf_length(numbers[2] as u32).node_offset(numbers[3])
                .node_depth(numbers[4] as u8).inner_hash_length(numbers[5] as usize)
                .last_node(numbers[6] != 0);
            setup.hash(data).as_bytes().to_vec()
        }
        "blake2s" => {
            let mut setup = blake2s_simd::Params::new();
            setup.hash_length(size).key(key).salt(salt).personal(person)
                .fanout(numbers[0] as u8).max_depth(numbers[1] as u8)
                .max_leaf_length(numbers[2] as u32).node_offset(numbers[3])
                .node_depth(numbers[4] as u8).inner_hash_length(numbers[5] as usize)
                .last_node(numbers[6] != 0);
            setup.hash(data).as_bytes().to_vec()
        }
        _ => return Err(format!("ValueError: unsupported hash type {name}")),
    };
    Ok(digest)
}

pub fn equal(left: &[u8], right: &[u8]) -> bool {
    constant_time_eq::constant_time_eq(left, right)
}

fn sum_md5(data: &[u8]) -> Vec<u8> { md5::Md5::digest(data).to_vec() }
fn sum_sha1(data: &[u8]) -> Vec<u8> { sha1::Sha1::digest(data).to_vec() }
fn sum_sha224(data: &[u8]) -> Vec<u8> { sha2::Sha224::digest(data).to_vec() }
fn sum_sha256(data: &[u8]) -> Vec<u8> { sha2::Sha256::digest(data).to_vec() }
fn sum_sha384(data: &[u8]) -> Vec<u8> { sha2::Sha384::digest(data).to_vec() }
fn sum_sha512(data: &[u8]) -> Vec<u8> { sha2::Sha512::digest(data).to_vec() }
fn sum_sha3_224(data: &[u8]) -> Vec<u8> { sha3::Sha3_224::digest(data).to_vec() }
fn sum_sha3_256(data: &[u8]) -> Vec<u8> { sha3::Sha3_256::digest(data).to_vec() }
fn sum_sha3_384(data: &[u8]) -> Vec<u8> { sha3::Sha3_384::digest(data).to_vec() }
fn sum_sha3_512(data: &[u8]) -> Vec<u8> { sha3::Sha3_512::digest(data).to_vec() }

fn identify(name: &str) -> Result<(usize, usize, fn(&[u8]) -> Vec<u8>), String> {
    Ok(match name {
        "md5" => (64, 16, sum_md5 as fn(&[u8]) -> Vec<u8>),
        "sha1" => (64, 20, sum_sha1),
        "sha224" => (64, 28, sum_sha224),
        "sha256" => (64, 32, sum_sha256),
        "sha384" => (128, 48, sum_sha384),
        "sha512" => (128, 64, sum_sha512),
        "sha3_224" => (144, 28, sum_sha3_224),
        "sha3_256" => (136, 32, sum_sha3_256),
        "sha3_384" => (104, 48, sum_sha3_384),
        "sha3_512" => (72, 64, sum_sha3_512),
        other => return Err(format!("ValueError: unsupported hash type {other}")),
    })
}

fn keyed(block: usize, key: &[u8], message: &[u8], hash: fn(&[u8]) -> Vec<u8>) -> Vec<u8> {
    let mut material = if key.len() > block { hash(key) } else { key.to_vec() };
    material.resize(block, 0);
    let mut inner = Vec::with_capacity(block + message.len());
    let mut outer = Vec::with_capacity(block + 64);
    for byte in &material {
        inner.push(byte ^ 0x36);
        outer.push(byte ^ 0x5c);
    }
    inner.extend_from_slice(message);
    outer.extend_from_slice(&hash(&inner));
    hash(&outer)
}

pub fn pbkdf2_hmac(name: &str, password: &[u8], salt: &[u8], iterations: usize, dk_len: usize) -> Result<Vec<u8>, String> {
    if iterations == 0 { return Err("ValueError: iteration value must be greater than 0".into()); }
    if dk_len == 0 { return Err("ValueError: key length must be greater than 0".into()); }
    let (block, length, hash) = identify(name)?;
    let pieces = (dk_len + length - 1) / length;
    let mut derived = Vec::new();
    derived.try_reserve_exact(pieces.saturating_mul(length)).map_err(|_| "MemoryError: ".to_string())?;
    for counter in 1..=pieces {
        let mut seed = Vec::with_capacity(salt.len() + 4);
        seed.extend_from_slice(salt);
        seed.extend_from_slice(&(counter as u32).to_be_bytes());
        let mut mixed = keyed(block, password, &seed, hash);
        let mut folded = mixed.clone();
        for _round in 1..iterations {
            mixed = keyed(block, password, &mixed, hash);
            for (left, right) in folded.iter_mut().zip(mixed.iter()) { *left ^= *right; }
        }
        derived.extend_from_slice(&folded);
    }
    derived.truncate(dk_len);
    Ok(derived)
}

fn salsa_block(block: &mut [u8; 64]) {
    let mut word = [0u32; 16];
    for index in 0..16 {
        let start = index * 4;
        word[index] = u32::from_le_bytes([block[start], block[start + 1], block[start + 2], block[start + 3]]);
    }
    let saved = word;
    const STEP: [(usize, usize, usize, u32); 32] = [
        (4, 0, 12, 7), (8, 4, 0, 9), (12, 8, 4, 13), (0, 12, 8, 18),
        (9, 5, 1, 7), (13, 9, 5, 9), (1, 13, 9, 13), (5, 1, 13, 18),
        (14, 10, 6, 7), (2, 14, 10, 9), (6, 2, 14, 13), (10, 6, 2, 18),
        (3, 15, 11, 7), (7, 3, 15, 9), (11, 7, 3, 13), (15, 11, 7, 18),
        (1, 0, 3, 7), (2, 1, 0, 9), (3, 2, 1, 13), (0, 3, 2, 18),
        (6, 5, 4, 7), (7, 6, 5, 9), (4, 7, 6, 13), (5, 4, 7, 18),
        (11, 10, 9, 7), (8, 11, 10, 9), (9, 8, 11, 13), (10, 9, 8, 18),
        (12, 15, 14, 7), (13, 12, 15, 9), (14, 13, 12, 13), (15, 14, 13, 18),
    ];
    for _pair in 0..4 {
        for (dest, left, right, bits) in STEP {
            word[dest] ^= word[left].wrapping_add(word[right]).rotate_left(bits);
        }
    }
    for index in 0..16 {
        let start = index * 4;
        block[start..start + 4].copy_from_slice(&word[index].wrapping_add(saved[index]).to_le_bytes());
    }
}

fn mix_block(data: &mut [u8]) {
    let count = data.len() / 64;
    let mut x = [0u8; 64];
    x.copy_from_slice(&data[data.len() - 64..]);
    let mut produced = vec![0u8; data.len()];
    for index in 0..count {
        for slot in 0..64 { x[slot] ^= data[index * 64 + slot]; }
        salsa_block(&mut x);
        produced[index * 64..index * 64 + 64].copy_from_slice(&x);
    }
    let half = count / 2;
    for index in 0..half {
        let even = index * 64;
        let odd = (half + index) * 64;
        data[even..even + 64].copy_from_slice(&produced[2 * index * 64..2 * index * 64 + 64]);
        data[odd..odd + 64].copy_from_slice(&produced[(2 * index + 1) * 64..(2 * index + 1) * 64 + 64]);
    }
}

fn choose(block: &[u8], n: usize) -> usize {
    let tail = block.len() - 64;
    let mut limb = [0u8; 8];
    limb.copy_from_slice(&block[tail..tail + 8]);
    (u64::from_le_bytes(limb) % n as u64) as usize
}

fn smix(block: &mut [u8], n: usize) -> Result<(), String> {
    let width = block.len();
    let cells = n.checked_mul(width).ok_or("MemoryError: ")?;
    let mut memory = Vec::new();
    memory.try_reserve_exact(cells).map_err(|_| "MemoryError: ".to_string())?;
    memory.resize(cells, 0);
    for step in 0..n {
        let at = step * width;
        memory[at..at + width].copy_from_slice(block);
        mix_block(block);
    }
    for _step in 0..n {
        let at = choose(block, n) * width;
        for slot in 0..width { block[slot] ^= memory[at + slot]; }
        mix_block(block);
    }
    Ok(())
}

pub fn scrypt_kdf(password: &[u8], salt: &[u8], n: usize, r: usize, p: usize, dk_len: usize) -> Result<Vec<u8>, String> {
    if n < 2 || !n.is_power_of_two() { return Err("ValueError: n must be a power of 2".into()); }
    if r == 0 || p == 0 { return Err("ValueError: Invalid parameter combination for n, r, p, maxmem.".into()); }
    let width = r.checked_mul(128).ok_or("MemoryError: ")?;
    let total = width.checked_mul(p).ok_or("MemoryError: ")?;
    let mut buffer = pbkdf2_hmac("sha256", password, salt, 1, total)?;
    for index in 0..p {
        let start = index * width;
        smix(&mut buffer[start..start + width], n)?;
    }
    pbkdf2_hmac("sha256", password, &buffer, 1, dk_len)
}
