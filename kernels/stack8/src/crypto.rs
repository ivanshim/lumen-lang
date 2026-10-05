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
