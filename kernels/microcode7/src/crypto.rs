use sha3::digest::{Digest, ExtendableOutput, Update, XofReader};

pub fn calculate(algorithm: &str, input: &[u8], count: usize, tree: &[u64], secret: &[u8], seasoning: &[u8], personal: &[u8]) -> Result<Vec<u8>, String> {
    let refused = || String::from("ValueError: invalid BLAKE2 parameters");
    if tree.len() != 7 { return Err(String::from("TypeError: expected seven tree parameters")); }
    match algorithm {
        "blake2b" | "blake2s" => {
            let limit = if algorithm.ends_with('b') { 64 } else { 32 };
            if !(1..=limit).contains(&count) || secret.len() > limit { return Err(refused()); }
            if personal.len() > limit / 4 || seasoning.len() > limit / 4 { return Err(refused()); }
            if tree[0] > 255 || !(1..=255).contains(&tree[1]) || tree[4] > 255 { return Err(refused()); }
            if tree[2] > 4294967295 || tree[5] > limit as u64 { return Err(refused()); }
            if limit == 32 && tree[3] >= 281474976710656 { return Err(refused()); }
        }
        "shake_128" | "shake_256" if count >= 536870912 => return Err(String::from("ValueError: length is too large")),
        _ => (),
    }
    if algorithm == "md5" { return Ok(md5::Md5::digest(input).into_iter().collect()); }
    if algorithm == "sha1" { return Ok(sha1::Sha1::digest(input).into_iter().collect()); }
    if algorithm == "sha224" { return Ok(sha2::Sha224::digest(input).into_iter().collect()); }
    if algorithm == "sha256" { return Ok(sha2::Sha256::digest(input).into_iter().collect()); }
    if algorithm == "sha384" { return Ok(sha2::Sha384::digest(input).into_iter().collect()); }
    if algorithm == "sha512" { return Ok(sha2::Sha512::digest(input).into_iter().collect()); }
    if algorithm == "sha3_224" { return Ok(sha3::Sha3_224::digest(input).into_iter().collect()); }
    if algorithm == "sha3_256" { return Ok(sha3::Sha3_256::digest(input).into_iter().collect()); }
    if algorithm == "sha3_384" { return Ok(sha3::Sha3_384::digest(input).into_iter().collect()); }
    if algorithm == "sha3_512" { return Ok(sha3::Sha3_512::digest(input).into_iter().collect()); }
    if algorithm == "shake_128" {
        let mut sponge = sha3::Shake128::default();
        Update::update(&mut sponge, input);
        let mut output = sponge.finalize_xof();
        let mut bytes = vec![0_u8; count];
        XofReader::read(&mut output, &mut bytes);
        return Ok(bytes);
    }
    if algorithm == "shake_256" {
        let mut sponge = sha3::Shake256::default();
        Update::update(&mut sponge, input);
        let mut output = sponge.finalize_xof();
        let mut bytes = vec![0_u8; count];
        XofReader::read(&mut output, &mut bytes);
        return Ok(bytes);
    }
    if algorithm == "blake2b" {
        let mut parameters = blake2b_simd::Params::new();
        parameters.key(secret);
        parameters.personal(personal).salt(seasoning);
        parameters.hash_length(count);
        parameters.node_offset(tree[3]).node_depth(tree[4] as u8);
        parameters.max_leaf_length(tree[2] as u32);
        parameters.max_depth(tree[1] as u8).fanout(tree[0] as u8);
        parameters.last_node(tree[6] != 0).inner_hash_length(tree[5] as usize);
        return Ok(parameters.hash(input).as_bytes().into());
    }
    if algorithm == "blake2s" {
        let mut parameters = blake2s_simd::Params::new();
        parameters.key(secret);
        parameters.personal(personal).salt(seasoning);
        parameters.hash_length(count);
        parameters.node_offset(tree[3]).node_depth(tree[4] as u8);
        parameters.max_leaf_length(tree[2] as u32);
        parameters.max_depth(tree[1] as u8).fanout(tree[0] as u8);
        parameters.last_node(tree[6] != 0).inner_hash_length(tree[5] as usize);
        return Ok(parameters.hash(input).as_bytes().into());
    }
    Err(format!("ValueError: unsupported hash type {algorithm}"))
}

pub fn constant_time(a: &[u8], b: &[u8]) -> bool {
    use constant_time_eq::constant_time_eq as compare;
    compare(a, b)
}

fn digest_of(algorithm: &str, input: &[u8]) -> Result<Vec<u8>, String> {
    if algorithm == "md5" { return Ok(md5::Md5::digest(input).to_vec()); }
    if algorithm == "sha1" { return Ok(sha1::Sha1::digest(input).to_vec()); }
    if algorithm == "sha224" { return Ok(sha2::Sha224::digest(input).to_vec()); }
    if algorithm == "sha256" { return Ok(sha2::Sha256::digest(input).to_vec()); }
    if algorithm == "sha384" { return Ok(sha2::Sha384::digest(input).to_vec()); }
    if algorithm == "sha512" { return Ok(sha2::Sha512::digest(input).to_vec()); }
    if algorithm == "sha3_224" { return Ok(sha3::Sha3_224::digest(input).to_vec()); }
    if algorithm == "sha3_256" { return Ok(sha3::Sha3_256::digest(input).to_vec()); }
    if algorithm == "sha3_384" { return Ok(sha3::Sha3_384::digest(input).to_vec()); }
    if algorithm == "sha3_512" { return Ok(sha3::Sha3_512::digest(input).to_vec()); }
    Err(format!("ValueError: unsupported hash type {algorithm}"))
}

fn shape_of(algorithm: &str) -> Result<(usize, usize), String> {
    let pair = match algorithm {
        "md5" => (64, 16),
        "sha1" => (64, 20),
        "sha224" => (64, 28),
        "sha256" => (64, 32),
        "sha384" => (128, 48),
        "sha512" => (128, 64),
        "sha3_224" => (144, 28),
        "sha3_256" => (136, 32),
        "sha3_384" => (104, 48),
        "sha3_512" => (72, 64),
        _ => return Err(format!("ValueError: unsupported hash type {algorithm}")),
    };
    Ok(pair)
}

fn sign(width: usize, secret: &[u8], message: &[u8], algorithm: &str) -> Result<Vec<u8>, String> {
    let mut prepared = Vec::new();
    if secret.len() > width {
        prepared.extend(digest_of(algorithm, secret)?);
    } else {
        prepared.extend_from_slice(secret);
    }
    while prepared.len() < width { prepared.push(0); }
    let mut inner_block = Vec::with_capacity(width + message.len());
    let mut outer_block = Vec::with_capacity(width + 64);
    let mut cursor = 0;
    while cursor < width {
        inner_block.push(prepared[cursor] ^ 0x36);
        outer_block.push(prepared[cursor] ^ 0x5c);
        cursor += 1;
    }
    inner_block.extend_from_slice(message);
    outer_block.extend(digest_of(algorithm, &inner_block)?);
    digest_of(algorithm, &outer_block)
}

pub fn stretch_key(algorithm: &str, secret: &[u8], seasoning: &[u8], rounds: usize, length: usize) -> Result<Vec<u8>, String> {
    if rounds == 0 { return Err(String::from("ValueError: iteration value must be greater than 0")); }
    if length == 0 { return Err(String::from("ValueError: key length must be greater than 0")); }
    let (width, piece) = shape_of(algorithm)?;
    let blocks = (length + piece - 1) / piece;
    let mut output = Vec::new();
    if output.try_reserve_exact(blocks.saturating_mul(piece)).is_err() { return Err(String::from("MemoryError: ")); }
    let mut counter = 1u32;
    while (counter as usize) <= blocks {
        let mut body = Vec::with_capacity(seasoning.len() + 4);
        body.extend_from_slice(seasoning);
        body.extend_from_slice(&counter.to_be_bytes());
        let mut chain = sign(width, secret, &body, algorithm)?;
        let mut folded = chain.clone();
        let mut turn = 1usize;
        while turn < rounds {
            chain = sign(width, secret, &chain, algorithm)?;
            let mut slot = 0;
            while slot < folded.len() {
                folded[slot] ^= chain[slot];
                slot += 1;
            }
            turn += 1;
        }
        output.extend_from_slice(&folded);
        counter += 1;
    }
    output.truncate(length);
    Ok(output)
}

fn stir(words: &mut [u32; 16], into: usize, lhs: usize, rhs: usize, bits: u32) {
    let sum = words[lhs].wrapping_add(words[rhs]);
    words[into] ^= sum.rotate_left(bits);
}

fn column_pass(words: &mut [u32; 16]) {
    stir(words, 4, 0, 12, 7);
    stir(words, 8, 4, 0, 9);
    stir(words, 12, 8, 4, 13);
    stir(words, 0, 12, 8, 18);
    stir(words, 9, 5, 1, 7);
    stir(words, 13, 9, 5, 9);
    stir(words, 1, 13, 9, 13);
    stir(words, 5, 1, 13, 18);
    stir(words, 14, 10, 6, 7);
    stir(words, 2, 14, 10, 9);
    stir(words, 6, 2, 14, 13);
    stir(words, 10, 6, 2, 18);
    stir(words, 3, 15, 11, 7);
    stir(words, 7, 3, 15, 9);
    stir(words, 11, 7, 3, 13);
    stir(words, 15, 11, 7, 18);
}

fn row_pass(words: &mut [u32; 16]) {
    stir(words, 1, 0, 3, 7);
    stir(words, 2, 1, 0, 9);
    stir(words, 3, 2, 1, 13);
    stir(words, 0, 3, 2, 18);
    stir(words, 6, 5, 4, 7);
    stir(words, 7, 6, 5, 9);
    stir(words, 4, 7, 6, 13);
    stir(words, 5, 4, 7, 18);
    stir(words, 11, 10, 9, 7);
    stir(words, 8, 11, 10, 9);
    stir(words, 9, 8, 11, 13);
    stir(words, 10, 9, 8, 18);
    stir(words, 12, 15, 14, 7);
    stir(words, 13, 12, 15, 9);
    stir(words, 14, 13, 12, 13);
    stir(words, 15, 14, 13, 18);
}

fn salsa_bytes(chunk: &mut [u8]) {
    let mut words = [0u32; 16];
    let mut index = 0;
    while index < 16 {
        let at = index * 4;
        words[index] = u32::from_le_bytes([chunk[at], chunk[at + 1], chunk[at + 2], chunk[at + 3]]);
        index += 1;
    }
    let original = words;
    let mut pass = 0;
    while pass < 4 {
        column_pass(&mut words);
        row_pass(&mut words);
        pass += 1;
    }
    index = 0;
    while index < 16 {
        let at = index * 4;
        let combined = words[index].wrapping_add(original[index]);
        chunk[at..at + 4].copy_from_slice(&combined.to_le_bytes());
        index += 1;
    }
}

fn reorder(area: &mut [u8]) {
    let blocks = area.len() / 64;
    let mut state = [0u8; 64];
    let last = area.len() - 64;
    let mut slot = 0;
    while slot < 64 {
        state[slot] = area[last + slot];
        slot += 1;
    }
    let mut scratch = vec![0u8; area.len()];
    let mut block_index = 0;
    while block_index < blocks {
        slot = 0;
        while slot < 64 {
            state[slot] ^= area[block_index * 64 + slot];
            slot += 1;
        }
        salsa_bytes(&mut state);
        scratch[block_index * 64..block_index * 64 + 64].copy_from_slice(&state);
        block_index += 1;
    }
    let half = blocks / 2;
    block_index = 0;
    while block_index < half {
        let even_at = block_index * 64;
        let odd_at = (half + block_index) * 64;
        area[even_at..even_at + 64].copy_from_slice(&scratch[(block_index * 2) * 64..(block_index * 2) * 64 + 64]);
        area[odd_at..odd_at + 64].copy_from_slice(&scratch[(block_index * 2 + 1) * 64..(block_index * 2 + 1) * 64 + 64]);
        block_index += 1;
    }
}

fn lane(area: &[u8], modulus: usize) -> usize {
    let origin = area.len() - 64;
    let mut raw = [0u8; 8];
    let mut slot = 0;
    while slot < 8 {
        raw[slot] = area[origin + slot];
        slot += 1;
    }
    (u64::from_le_bytes(raw) % modulus as u64) as usize
}

fn scramble(area: &mut [u8], iterations: usize) -> Result<(), String> {
    let width = area.len();
    let cells = iterations.checked_mul(width).ok_or_else(|| String::from("MemoryError: "))?;
    let mut tape = Vec::new();
    if tape.try_reserve_exact(cells).is_err() { return Err(String::from("MemoryError: ")); }
    tape.resize(cells, 0u8);
    let mut step = 0;
    while step < iterations {
        let at = step * width;
        tape[at..at + width].copy_from_slice(area);
        reorder(area);
        step += 1;
    }
    step = 0;
    while step < iterations {
        let at = lane(area, iterations) * width;
        let mut slot = 0;
        while slot < width {
            area[slot] ^= tape[at + slot];
            slot += 1;
        }
        reorder(area);
        step += 1;
    }
    Ok(())
}

pub fn memory_hard(secret: &[u8], seasoning: &[u8], n: usize, r: usize, p: usize, length: usize) -> Result<Vec<u8>, String> {
    if n < 2 || !n.is_power_of_two() { return Err(String::from("ValueError: n must be a power of 2")); }
    if r == 0 || p == 0 { return Err(String::from("ValueError: Invalid parameter combination for n, r, p, maxmem.")); }
    let width = r.checked_mul(128).ok_or_else(|| String::from("MemoryError: "))?;
    let total = width.checked_mul(p).ok_or_else(|| String::from("MemoryError: "))?;
    let mut buffer = stretch_key("sha256", secret, seasoning, 1, total)?;
    let mut index = 0;
    while index < p {
        let start = index * width;
        scramble(&mut buffer[start..start + width], n)?;
        index += 1;
    }
    stretch_key("sha256", secret, &buffer, 1, length)
}
