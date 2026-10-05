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
