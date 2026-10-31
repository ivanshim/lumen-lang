// Original bzip2 codec written from the published format description:
// run-length stage, Burrows-Wheeler sort, move-to-front, Huffman groups.
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::sync::OnceLock;

const BLOCK_MAGIC: u64 = 0x3141_5926_5359;
const END_MAGIC: u64 = 0x1772_4538_5090;
const GROUP_SPAN: usize = 50;
pub const INVALID: &str = "Invalid data stream";

fn crc_table() -> &'static [u32; 256] {
    static TABLE: OnceLock<[u32; 256]> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = [0u32; 256];
        for (i, slot) in table.iter_mut().enumerate() {
            let mut c = (i as u32) << 24;
            for _ in 0..8 { c = if c & 0x8000_0000 != 0 { (c << 1) ^ 0x04c1_1db7 } else { c << 1 }; }
            *slot = c;
        }
        table
    })
}
fn crc_step(crc: u32, byte: u8) -> u32 { (crc << 8) ^ crc_table()[((crc >> 24) as u8 ^ byte) as usize] }

// ---------------------------------------------------------------- writing

struct BitSink { bytes: Vec<u8>, pending: u64, held: u32 }
impl BitSink {
    fn put(&mut self, width: u32, value: u32) {
        self.pending = (self.pending << width) | value as u64;
        self.held += width;
        while self.held >= 8 {
            self.bytes.push((self.pending >> (self.held - 8)) as u8);
            self.held -= 8;
        }
        self.pending &= (1u64 << self.held) - 1;
    }
    fn put48(&mut self, value: u64) { self.put(24, (value >> 24) as u32); self.put(24, (value & 0xff_ffff) as u32); }
    fn pad(&mut self) { if self.held > 0 { self.put(8 - self.held, 0); } }
}

pub struct Compressor {
    level: usize, block: Vec<u8>, block_crc: u32, total_crc: u32,
    run_byte: u8, run_len: usize, sink: BitSink, started: bool,
}

/// Cyclic rotation order. An induced suffix array of the doubled block: the
/// comparison sort's many rounds took seconds per tar block in a debug build.
fn sorted_rotations(data: &[u8]) -> Vec<u32> {
    let n = data.len();
    if n <= 1 {
        return (0..n as u32).collect();
    }
    let mut text = Vec::with_capacity(n * 2 + 1);
    for _ in 0..2 {
        text.extend(data.iter().map(|&byte| byte as u32 + 1));
    }
    text.push(0);
    let mut ranks = vec![0i32; text.len()];
    rotation_ranks(&text, &mut ranks, 257);
    let mut order = Vec::with_capacity(n);
    for rank in ranks {
        if (rank as usize) < n {
            order.push(rank as u32);
        }
    }
    order
}

fn bucket_bounds(text: &[u32], alphabet: usize) -> (Vec<usize>, Vec<usize>) {
    let mut count = vec![0usize; alphabet];
    for &symbol in text {
        count[symbol as usize] += 1;
    }
    let mut head = vec![0usize; alphabet];
    let mut tail = vec![0usize; alphabet];
    let mut sum = 0usize;
    for symbol in 0..alphabet {
        head[symbol] = sum;
        sum += count[symbol];
        tail[symbol] = sum;
    }
    (head, tail)
}

fn seed_lms(text: &[u32], ranks: &mut [i32], lms: &[usize], tail: &[usize]) {
    ranks.fill(-1);
    let mut cursor = tail.to_vec();
    for &index in lms.iter().rev() {
        let symbol = text[index] as usize;
        cursor[symbol] -= 1;
        ranks[cursor[symbol]] = index as i32;
    }
}

fn induce_l(text: &[u32], ranks: &mut [i32], shorter: &[u8], head: &[usize]) {
    let mut cursor = head.to_vec();
    for slot in 0..ranks.len() {
        let index = ranks[slot];
        if index <= 0 {
            continue;
        }
        let previous = (index as usize) - 1;
        if shorter[previous] == 0 {
            let symbol = text[previous] as usize;
            ranks[cursor[symbol]] = previous as i32;
            cursor[symbol] += 1;
        }
    }
}

fn induce_s(text: &[u32], ranks: &mut [i32], shorter: &[u8], tail: &[usize]) {
    let mut cursor = tail.to_vec();
    for slot in (0..ranks.len()).rev() {
        let index = ranks[slot];
        if index <= 0 {
            continue;
        }
        let previous = (index as usize) - 1;
        if shorter[previous] == 1 {
            let symbol = text[previous] as usize;
            cursor[symbol] -= 1;
            ranks[cursor[symbol]] = previous as i32;
        }
    }
}

fn lms_index(shorter: &[u8], index: usize) -> bool {
    index > 0 && shorter[index] == 1 && shorter[index - 1] == 0
}

fn same_lms(text: &[u32], shorter: &[u8], left: usize, right: usize) -> bool {
    if text[left] != text[right] {
        return false;
    }
    let mut step = 1usize;
    loop {
        let left_done = left + step >= text.len();
        let right_done = right + step >= text.len();
        if left_done || right_done {
            return left_done && right_done;
        }
        if text[left + step] != text[right + step] {
            return false;
        }
        let left_lms = lms_index(shorter, left + step);
        let right_lms = lms_index(shorter, right + step);
        if left_lms || right_lms {
            return left_lms && right_lms;
        }
        step += 1;
    }
}

fn rotation_ranks(text: &[u32], ranks: &mut [i32], alphabet: usize) {
    let n = text.len();
    if n == 1 {
        ranks[0] = 0;
        return;
    }
    let mut shorter = vec![0u8; n];
    shorter[n - 1] = 1;
    for index in (0..n - 1).rev() {
        shorter[index] = u8::from(
            text[index] < text[index + 1] || (text[index] == text[index + 1] && shorter[index + 1] == 1),
        );
    }
    let mut lms = Vec::new();
    for index in 1..n {
        if lms_index(&shorter, index) {
            lms.push(index);
        }
    }
    let (head, tail) = bucket_bounds(text, alphabet);
    seed_lms(text, ranks, &lms, &tail);
    induce_l(text, ranks, &shorter, &head);
    induce_s(text, ranks, &shorter, &tail);

    let mut name_of = vec![-1i32; n];
    let mut rank = 0i32;
    let mut previous = -1i32;
    let mut repeated = false;
    for &index in ranks.iter() {
        if index < 0 || !lms_index(&shorter, index as usize) {
            continue;
        }
        if previous >= 0 {
            if same_lms(text, &shorter, previous as usize, index as usize) {
                repeated = true;
            } else {
                rank += 1;
            }
        }
        name_of[index as usize] = rank;
        previous = index;
    }
    let count = lms.len();
    if repeated {
        let mut reduced = vec![0u32; count];
        let mut position = Vec::with_capacity(count);
        let mut at = 0usize;
        for index in 0..n {
            if name_of[index] >= 0 {
                reduced[at] = name_of[index] as u32;
                position.push(index);
                at += 1;
            }
        }
        let mut reduced_ranks = vec![0i32; count];
        rotation_ranks(&reduced, &mut reduced_ranks, rank as usize + 1);
        for (slot, reduced_rank) in reduced_ranks.iter().enumerate() {
            lms[slot] = position[*reduced_rank as usize];
        }
    } else {
        let mut ordered = vec![0usize; count];
        for index in 0..n {
            let name = name_of[index];
            if name >= 0 {
                ordered[name as usize] = index;
            }
        }
        lms = ordered;
    }
    seed_lms(text, ranks, &lms, &tail);
    induce_l(text, ranks, &shorter, &head);
    induce_s(text, ranks, &shorter, &tail);
}

fn code_lengths(freq: &[u32], limit: u32) -> Vec<u8> {
    let n = freq.len();
    let mut weight: Vec<u64> = freq.iter().map(|&f| f.max(1) as u64).collect();
    loop {
        let mut parent = vec![usize::MAX; 2 * n];
        let mut heap: BinaryHeap<Reverse<(u64, usize)>> = weight.iter().enumerate().map(|(i, &w)| Reverse((w, i))).collect();
        let mut next = n;
        while heap.len() > 1 {
            let Reverse((wa, a)) = heap.pop().unwrap();
            let Reverse((wb, b)) = heap.pop().unwrap();
            parent[a] = next; parent[b] = next;
            heap.push(Reverse((wa + wb, next)));
            next += 1;
        }
        let depth = |mut at: usize| { let mut d = 0u32; while parent[at] != usize::MAX { at = parent[at]; d += 1; } d };
        let lengths: Vec<u32> = (0..n).map(depth).collect();
        if lengths.iter().all(|&d| d <= limit) { return lengths.into_iter().map(|d| d as u8).collect(); }
        for w in weight.iter_mut() { *w = 1 + *w / 2; }
    }
}

fn assign_codes(lengths: &[u8]) -> Vec<u32> {
    let mut codes = vec![0u32; lengths.len()];
    let (low, high) = (*lengths.iter().min().unwrap(), *lengths.iter().max().unwrap());
    let mut code = 0u32;
    for width in low..=high {
        for (i, &l) in lengths.iter().enumerate() { if l == width { codes[i] = code; code += 1; } }
        code <<= 1;
    }
    codes
}

impl Compressor {
    pub fn new(level: usize) -> Self {
        Compressor { level, block: Vec::new(), block_crc: !0, total_crc: 0, run_byte: 0, run_len: 0,
            sink: BitSink { bytes: Vec::new(), pending: 0, held: 0 }, started: false }
    }
    fn begin(&mut self) {
        if self.started { return; }
        self.started = true;
        for b in [b'B', b'Z', b'h', b'0' + self.level as u8] { self.sink.put(8, b as u32); }
    }
    pub fn feed(&mut self, data: &[u8]) -> Vec<u8> {
        self.begin();
        for &b in data {
            if self.run_len > 0 && b == self.run_byte && self.run_len < 255 { self.run_len += 1; continue; }
            self.close_run();
            self.run_byte = b; self.run_len = 1;
        }
        std::mem::take(&mut self.sink.bytes)
    }
    pub fn finish(&mut self) -> Vec<u8> {
        self.begin();
        self.close_run();
        self.close_block();
        self.sink.put48(END_MAGIC);
        self.sink.put(32, self.total_crc);
        self.sink.pad();
        std::mem::take(&mut self.sink.bytes)
    }
    fn close_run(&mut self) {
        if self.run_len == 0 { return; }
        if self.block.len() + 5 > 100_000 * self.level - 19 { self.close_block(); }
        for _ in 0..self.run_len { self.block_crc = crc_step(self.block_crc, self.run_byte); }
        let shown = self.run_len.min(4);
        self.block.extend(std::iter::repeat(self.run_byte).take(shown));
        if self.run_len >= 4 { self.block.push((self.run_len - 4) as u8); }
        self.run_len = 0;
    }
    fn close_block(&mut self) {
        if self.block.is_empty() { return; }
        let data = std::mem::take(&mut self.block);
        let crc = !self.block_crc;
        self.block_crc = !0;
        self.total_crc = self.total_crc.rotate_left(1) ^ crc;
        let n = data.len();
        let order = sorted_rotations(&data);
        let origin = order.iter().position(|&i| i == 0).unwrap();
        let last: Vec<u8> = order.iter().map(|&i| data[(i as usize + n - 1) % n]).collect();

        let mut used = [false; 256];
        for &b in &data { used[b as usize] = true; }
        let mut slot = [0u8; 256];
        let mut recency: Vec<u8> = Vec::new();
        for v in 0..256 { if used[v] { slot[v] = recency.len() as u8; recency.push(v as u8); } }
        let alpha = recency.len() + 2;
        let end = (alpha - 1) as u16;
        // Move-to-front with zero runs written in bijective base two.
        let mut symbols: Vec<u16> = Vec::with_capacity(n + 1);
        let mut zeros = 0usize;
        let mut order_of: Vec<u8> = (0..recency.len()).map(|v| v as u8).collect();
        let flush_zeros = |symbols: &mut Vec<u16>, zeros: &mut usize| {
            if *zeros > 0 {
                let mut z = *zeros - 1;
                loop { symbols.push((z & 1) as u16); if z < 2 { break; } z = (z - 2) / 2; }
                *zeros = 0;
            }
        };
        for &c in &last {
            let s = slot[c as usize];
            let at = order_of.iter().position(|&x| x == s).unwrap();
            if at == 0 { zeros += 1; continue; }
            flush_zeros(&mut symbols, &mut zeros);
            order_of.remove(at); order_of.insert(0, s);
            symbols.push(at as u16 + 1);
        }
        flush_zeros(&mut symbols, &mut zeros);
        symbols.push(end);

        let groups = match symbols.len() { 0..=199 => 2, 200..=599 => 3, 600..=1199 => 4, 1200..=2399 => 5, _ => 6 };
        let mut freq = vec![0u32; alpha];
        for &s in &symbols { freq[s as usize] += 1; }
        let mut lengths = vec![vec![15u8; alpha]; groups];
        let (mut remaining, mut start, mut left) = (symbols.len() as i64, 0i64, groups);
        while left > 0 {
            let target = remaining / left as i64;
            let (mut stop, mut got) = (start - 1, 0i64);
            while got < target && stop < alpha as i64 - 1 { stop += 1; got += freq[stop as usize] as i64; }
            if stop > start && left != groups && left != 1 && (groups - left) % 2 == 1 { got -= freq[stop as usize] as i64; stop -= 1; }
            for v in start..=stop.max(start - 1) { lengths[left - 1][v as usize] = 0; }
            left -= 1; start = stop + 1; remaining -= got;
        }
        let mut selectors: Vec<u8> = Vec::new();
        for _ in 0..4 {
            let mut tally = vec![vec![0u32; alpha]; groups];
            selectors.clear();
            for chunk in symbols.chunks(GROUP_SPAN) {
                let best = (0..groups).min_by_key(|&t| chunk.iter().map(|&s| lengths[t][s as usize] as u32).sum::<u32>()).unwrap();
                selectors.push(best as u8);
                for &s in chunk { tally[best][s as usize] += 1; }
            }
            for t in 0..groups { lengths[t] = code_lengths(&tally[t], 17); }
        }
        let codes: Vec<Vec<u32>> = lengths.iter().map(|l| assign_codes(l)).collect();

        let sink = &mut self.sink;
        sink.put48(BLOCK_MAGIC);
        sink.put(32, crc);
        sink.put(1, 0);
        sink.put(24, origin as u32);
        let mut ranges = 0u32;
        for r in 0..16 { if (0..16).any(|k| used[r * 16 + k]) { ranges |= 1 << (15 - r); } }
        sink.put(16, ranges);
        for r in 0..16 {
            if ranges & (1 << (15 - r)) != 0 {
                let mut bits = 0u32;
                for k in 0..16 { if used[r * 16 + k] { bits |= 1 << (15 - k); } }
                sink.put(16, bits);
            }
        }
        sink.put(3, groups as u32);
        sink.put(15, selectors.len() as u32);
        let mut line: Vec<u8> = (0..groups as u8).collect();
        for &sel in &selectors {
            let at = line.iter().position(|&x| x == sel).unwrap();
            line.remove(at); line.insert(0, sel);
            for _ in 0..at { sink.put(1, 1); }
            sink.put(1, 0);
        }
        for table in &lengths {
            let mut cur = table[0] as i32;
            sink.put(5, cur as u32);
            for &l in table {
                while cur < l as i32 { sink.put(2, 2); cur += 1; }
                while cur > l as i32 { sink.put(2, 3); cur -= 1; }
                sink.put(1, 0);
            }
        }
        for (chunk, &sel) in symbols.chunks(GROUP_SPAN).zip(&selectors) {
            for &s in chunk { sink.put(lengths[sel as usize][s as usize] as u32, codes[sel as usize][s as usize]); }
        }
    }
}

// ---------------------------------------------------------------- reading

struct Table { count: [u32; 22], first: [u32; 22], offset: [u32; 22], perm: Vec<u16>, deepest: usize }

struct Block {
    crc: u32, origin: usize, unseq: Vec<u8>, selectors: Vec<u8>, tables: Vec<Table>,
    recency: Vec<u8>, column: Vec<u32>, tally: [u32; 256], at_selector: usize, left_in_group: usize,
    run: usize, run_weight: usize,
}

enum Stage { Header, Magic, Tables, Symbols(Box<Block>), Done }

pub struct Decompressor {
    buf: Vec<u8>, at: usize, stage: Stage, level: usize, total_crc: u32,
    pending: Vec<u8>, given: usize, finished: bool,
}

fn build_table(lengths: &[u8]) -> Table {
    let mut table = Table { count: [0; 22], first: [0; 22], offset: [0; 22], perm: Vec::new(), deepest: 0 };
    for &l in lengths { table.count[l as usize] += 1; table.deepest = table.deepest.max(l as usize); }
    let (mut code, mut seen) = (0u32, 0u32);
    for l in 1..=20 {
        table.first[l] = code; table.offset[l] = seen;
        code = (code + table.count[l]) << 1; seen += table.count[l];
    }
    for l in 1..=20u8 { for (s, &sl) in lengths.iter().enumerate() { if sl == l { table.perm.push(s as u16); } } }
    table
}

impl Decompressor {
    pub fn new() -> Self {
        Decompressor { buf: Vec::new(), at: 0, stage: Stage::Header, level: 0, total_crc: 0, pending: Vec::new(), given: 0, finished: false }
    }
    fn avail(&self) -> usize { self.buf.len() * 8 - self.at }
    fn peek(&self, width: u32) -> u32 {
        let (byte, sh) = (self.at >> 3, (self.at & 7) as u32);
        let mut v = 0u64;
        for k in 0..5 { v = (v << 8) | *self.buf.get(byte + k).unwrap_or(&0) as u64; }
        ((v >> (40 - sh - width)) & ((1u64 << width) - 1)) as u32
    }
    fn take(&mut self, width: u32) -> Option<u32> {
        if self.avail() < width as usize { return None; }
        let v = self.peek(width); self.at += width as usize; Some(v)
    }
    fn take48(&mut self) -> Option<u64> { let a = self.take(24)? as u64; Some((a << 24) | self.take(24)? as u64) }
    pub fn eof(&self) -> bool { self.finished && self.given == self.pending.len() }
    pub fn unused(&self) -> Vec<u8> { self.buf[(self.at + 7) / 8..].to_vec() }

    /// Decode up to `limit` bytes (negative: all); `.1` is true when the call stopped for lack of input.
    pub fn feed(&mut self, data: &[u8], limit: i64) -> Result<(Vec<u8>, bool), &'static str> {
        if self.finished { return Ok((Vec::new(), false)); }
        self.buf.extend_from_slice(data);
        let mut out = Vec::new();
        let stalled = loop {
            let room = if limit < 0 { usize::MAX } else { limit as usize - out.len() };
            let take = room.min(self.pending.len() - self.given);
            out.extend_from_slice(&self.pending[self.given..self.given + take]);
            self.given += take;
            if limit >= 0 && out.len() >= limit as usize { break false; }
            if self.given < self.pending.len() { continue; }
            self.pending.clear(); self.given = 0;
            if self.finished { break false; }
            if !self.advance()? { break true; }
        };
        let used = self.at / 8;
        if !matches!(self.stage, Stage::Done) { self.buf.drain(..used); self.at -= used * 8; }
        Ok((out, stalled && !self.finished))
    }

    /// Run the state machine until a block is produced, the stream ends or input runs out.
    fn advance(&mut self) -> Result<bool, &'static str> {
        loop {
            match &mut self.stage {
                Stage::Header => {
                    if self.avail() < 32 { return Ok(false); }
                    let head = self.peek(32);
                    let digit = (head & 0xff) as u8;
                    if head >> 8 != 0x42_5a68 || !(b'1'..=b'9').contains(&digit) { return Err(INVALID); }
                    self.at += 32; self.level = (digit - b'0') as usize; self.stage = Stage::Magic;
                }
                Stage::Magic => {
                    if self.avail() < 48 { return Ok(false); }
                    let start = self.at;
                    let magic = self.take48().unwrap();
                    if magic == BLOCK_MAGIC { self.stage = Stage::Tables; }
                    else if magic == END_MAGIC {
                        let Some(stored) = self.take(32) else { self.at = start; return Ok(false); };
                        if stored != self.total_crc { return Err(INVALID); }
                        self.stage = Stage::Done; self.finished = true;
                        self.at = (self.at + 7) & !7;
                        return Ok(true);
                    } else { return Err(INVALID); }
                }
                Stage::Tables => {
                    let start = self.at;
                    match self.read_tables()? {
                        Some(block) => self.stage = Stage::Symbols(Box::new(block)),
                        None => { self.at = start; return Ok(false); }
                    }
                }
                Stage::Symbols(_) => {
                    if !self.read_symbols()? { return Ok(false); }
                    if self.pending.is_empty() { continue; }
                    return Ok(true);
                }
                Stage::Done => return Ok(false),
            }
        }
    }

    fn read_tables(&mut self) -> Result<Option<Block>, &'static str> {
        let Some(crc) = self.take(32) else { return Ok(None) };
        let Some(randomised) = self.take(1) else { return Ok(None) };
        if randomised != 0 { return Err(INVALID); }
        let Some(origin) = self.take(24) else { return Ok(None) };
        let Some(ranges) = self.take(16) else { return Ok(None) };
        let mut unseq = Vec::new();
        for r in 0..16 {
            if ranges & (1 << (15 - r)) != 0 {
                let Some(bits) = self.take(16) else { return Ok(None) };
                for k in 0..16 { if bits & (1 << (15 - k)) != 0 { unseq.push((r * 16 + k) as u8); } }
            }
        }
        if unseq.is_empty() { return Err(INVALID); }
        let alpha = unseq.len() + 2;
        let Some(groups) = self.take(3) else { return Ok(None) };
        if !(2..=6).contains(&groups) { return Err(INVALID); }
        let Some(count) = self.take(15) else { return Ok(None) };
        if count < 1 { return Err(INVALID); }
        let mut line: Vec<u8> = (0..groups as u8).collect();
        let mut selectors = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let mut at = 0usize;
            loop {
                let Some(bit) = self.take(1) else { return Ok(None) };
                if bit == 0 { break; }
                at += 1;
                if at >= groups as usize { return Err(INVALID); }
            }
            let v = line.remove(at); line.insert(0, v);
            selectors.push(v);
        }
        let mut tables = Vec::new();
        for _ in 0..groups {
            let Some(start) = self.take(5) else { return Ok(None) };
            let mut cur = start as i32;
            let mut lengths = vec![0u8; alpha];
            for slot in lengths.iter_mut() {
                loop {
                    if !(1..=20).contains(&cur) { return Err(INVALID); }
                    let Some(bit) = self.take(1) else { return Ok(None) };
                    if bit == 0 { break; }
                    let Some(dir) = self.take(1) else { return Ok(None) };
                    cur += if dir == 0 { 1 } else { -1 };
                }
                *slot = cur as u8;
            }
            tables.push(build_table(&lengths));
        }
        Ok(Some(Block { crc, origin: origin as usize, recency: (0..unseq.len()).map(|v| v as u8).collect(), unseq, selectors, tables,
            column: Vec::new(), tally: [0; 256], at_selector: 0, left_in_group: GROUP_SPAN, run: 0, run_weight: 1 }))
    }

    /// Decode symbols of the current block; true once the block was completed into `pending`.
    fn read_symbols(&mut self) -> Result<bool, &'static str> {
        let Stage::Symbols(mut block) = std::mem::replace(&mut self.stage, Stage::Done) else { unreachable!() };
        let capacity = self.level * 100_000;
        let end = block.unseq.len() as u16 + 1;
        loop {
            if block.left_in_group == 0 {
                block.at_selector += 1;
                if block.at_selector >= block.selectors.len() { self.stage = Stage::Done; return Err(INVALID); }
                block.left_in_group = GROUP_SPAN;
            }
            let table = &block.tables[block.selectors[block.at_selector] as usize];
            let seen = self.avail().min(20) as u32;
            let window = if seen == 0 { 0 } else { self.peek(seen) << (20 - seen) };
            let mut found = None;
            for width in 1..=table.deepest {
                if width as u32 > seen { break; }
                let code = window >> (20 - width);
                if code >= table.first[width] && code - table.first[width] < table.count[width] {
                    found = Some((width, table.perm[(table.offset[width] + code - table.first[width]) as usize]));
                    break;
                }
            }
            let Some((width, sym)) = found else {
                if seen >= table.deepest as u32 { self.stage = Stage::Done; return Err(INVALID); }
                self.stage = Stage::Symbols(block);
                return Ok(false);
            };
            self.at += width;
            block.left_in_group -= 1;
            if sym <= 1 {
                if block.run_weight > capacity { self.stage = Stage::Done; return Err(INVALID); }
                block.run += block.run_weight << sym; block.run_weight <<= 1;
                continue;
            }
            if block.run > 0 {
                let byte = block.unseq[block.recency[0] as usize];
                if block.column.len() + block.run > capacity { self.stage = Stage::Done; return Err(INVALID); }
                block.tally[byte as usize] += block.run as u32;
                let run = block.run;
                block.column.extend(std::iter::repeat(byte as u32).take(run));
                block.run = 0; block.run_weight = 1;
            }
            if sym == end { break; }
            let v = block.recency.remove(sym as usize - 1);
            block.recency.insert(0, v);
            let byte = block.unseq[v as usize];
            if block.column.len() >= capacity { self.stage = Stage::Done; return Err(INVALID); }
            block.tally[byte as usize] += 1;
            block.column.push(byte as u32);
        }
        let result = self.finish_block(*block);
        self.stage = if result.is_ok() { Stage::Magic } else { Stage::Done };
        result.map(|()| true)
    }

    fn finish_block(&mut self, mut block: Block) -> Result<(), &'static str> {
        let n = block.column.len();
        if n == 0 || block.origin >= n { return Err(INVALID); }
        let mut start = [0usize; 256];
        let mut sum = 0;
        for v in 0..256 { start[v] = sum; sum += block.tally[v] as usize; }
        for i in 0..n {
            let b = (block.column[i] & 0xff) as usize;
            block.column[start[b]] |= (i as u32) << 8;
            start[b] += 1;
        }
        let mut pos = (block.column[block.origin] >> 8) as usize;
        let (mut crc, mut run, mut last) = (!0u32, 0usize, -1i32);
        let mut produced = Vec::with_capacity(n);
        let emit = |byte: u8, copies: usize, produced: &mut Vec<u8>, crc: &mut u32| {
            for _ in 0..copies { produced.push(byte); *crc = crc_step(*crc, byte); }
        };
        for _ in 0..n {
            let entry = block.column[pos];
            let byte = (entry & 0xff) as u8;
            pos = (entry >> 8) as usize;
            if run == 4 {
                emit(last as u8, byte as usize, &mut produced, &mut crc);
                run = 0; last = -1;
                continue;
            }
            if byte as i32 == last { run += 1; } else { run = 1; last = byte as i32; }
            emit(byte, 1, &mut produced, &mut crc);
        }
        if !crc != block.crc { return Err(INVALID); }
        self.total_crc = self.total_crc.rotate_left(1) ^ block.crc;
        self.pending = produced;
        self.given = 0;
        Ok(())
    }
}
