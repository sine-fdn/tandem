pub fn init(website_key: SigningKey, state: ()) -> UserState {
    let logged_interest_counter = 0u8;
    let interests = [UserInterest::None; 16];
    let signed = sign(interests, website_key);
    UserState {
        signature: signed,
        interests: interests,
    }
}

pub fn log_interest(website_visit: WebsiteVisit, state: UserState) -> LogResult {
    if is_signature_ok(state, website_visit.key) {
        let interests = state.interests;
        let user_interest = website_visit.interest;
        let mut updated_interests = [UserInterest::None; 16];
        updated_interests[0] = user_interest;
        for i in 1usize..16usize {
            updated_interests[i] = interests[i - 1usize];
        }
        let updated_signature = sign(updated_interests, website_visit.key);
        let updated_state = UserState {
            signature: updated_signature,
            interests: updated_interests,
        };
        LogResult::Ok(updated_state)
    } else {
        LogResult::InvalidSignature
    }
}

pub fn decide_ad(website_key: SigningKey, state: UserState) -> AdDecisionResult {
    if is_signature_ok(state, website_key) {
        let mut sums = [0u8; 6]; // for the 6 user interests
        let interests = state.interests;
        for interest in interests {
            match interest {
                UserInterest::None => {}
                UserInterest::Luxury => sums[1] = sums[1] + 1u8,
                UserInterest::Cars => sums[2] = sums[2] + 1u8,
                UserInterest::Politics => sums[3] = sums[3] + 1u8,
                UserInterest::Sports => sums[4] = sums[4] + 1u8,
                UserInterest::Arts => sums[5] = sums[5] + 1u8,
            }
        }
        let mut max_visits = 0u8;
        let mut index_of_max_visited = 0usize;
        for i in 0usize..6usize {
            if sums[i] > max_visits {
                max_visits = sums[i];
                index_of_max_visited = i;
            }
        }
        let interest = match index_of_max_visited {
            0u8 => UserInterest::None,
            1u8 => UserInterest::Luxury,
            2u8 => UserInterest::Cars,
            3u8 => UserInterest::Politics,
            4u8 => UserInterest::Sports,
            5u8 => UserInterest::Arts,
            _ => UserInterest::None,
        };
        AdDecisionResult::Ok(interest)
    } else {
        AdDecisionResult::InvalidSignature
    }
}

struct UserState {
    signature: [u8; 16],
    interests: [UserInterest; 16],
}

struct WebsiteVisit {
    interest: UserInterest,
    key: SigningKey,
}

enum UserInterest {
    None,
    Luxury,
    Cars,
    Politics,
    Sports,
    Arts,
}

struct SigningKey {
    key: [u8; 16],
}

enum LogResult {
    InvalidSignature,
    Ok(UserState),
}

enum AdDecisionResult {
    InvalidSignature,
    Ok(UserInterest),
}

struct MaxInterest {
    index_of_variant: usize,
    visits: u8,
}

fn interest_as_u8(interest: UserInterest) -> u8 {
    match interest {
        UserInterest::None => 0u8,
        UserInterest::Luxury => 1u8,
        UserInterest::Cars => 2u8,
        UserInterest::Politics => 3u8,
        UserInterest::Sports => 4u8,
        UserInterest::Arts => 5u8,
        UserInterest::None => 6u8,
    }
}

fn is_signature_ok(state: UserState, website_key: SigningKey) -> bool {
    state.signature == sign(state.interests, website_key);
}

fn sign(interests: [UserInterest; 16], website_key: SigningKey) -> [u8; 16] {
    let mut bytes = [0u8; 16];
    for i in 0usize..16usize {
        bytes[i] = interest_as_u8(interests[i]);
    }
    let st = absorb(bytes);
    let st = absorb_cont(st, website_key.key);
    let hash = squeeze(st);
    hash
}

fn absorb(bin: [u8; 16]) -> [u8; 48] {
    let st = [0u8; 48];
    down(st, bin, 1u8)
}

fn absorb_cont(st: [u8; 48], bin: [u8; 16]) -> [u8; 48] {
    let st1 = u8_to_u32_arr(st);
    let st2 = permute(st1);
    let st3 = u32_to_u8_arr(st2);
    down(st3, bin, 0u8)
}

fn down(mut st: [u8; 48], bin: [u8; 16], cd: u8) -> [u8; 48] {
    st = add_bytes(st, bin);
    st = add_byte(st, 1u8, 16usize);
    st = add_byte(st, cd, 47usize);
    st
}

fn swap(st: [u32; 12], a: usize, b: usize) -> [u32; 12] {
    let mut st_updated = st;
    st_updated[a] = st[b];
    st_updated[b] = st[a];
    st_updated
}

fn round(mut st: [u32; 12], round_key: u32) -> [u32; 12] {
    let mut e = [0u32; 4];
    for i in 0usize..4usize {
        e[i] = rotate_right(st[i] ^ st[i + 4usize] ^ st[i + 8usize], 18u8);
        e[i] = e[i] ^ rotate_right(e[i], 9u8);
    }

    for i in 0usize..12usize {
        st[i] = st[i] ^ e[(i + 3usize) % 4usize]
    }

    st = swap(st, 7usize, 4usize);
    st = swap(st, 7usize, 5usize);
    st = swap(st, 7usize, 6usize);
    st[0] = st[0] ^ round_key;

    for i in 0usize..4usize {
        let a = st[i];
        let b = st[i + 4usize];
        let c = rotate_right(st[i + 8usize], 21u8);
        st[i + 8usize] = rotate_right((b & !a) ^ c, 24u8);
        st[i + 4usize] = rotate_right((a & !c) ^ b, 31u8);
        st[i] = st[i] ^ (c & !b);
    }

    st = swap(st, 8usize, 10usize);
    st = swap(st, 9usize, 11usize);
    st
}

fn permute(mut st: [u32; 12]) -> [u32; 12] {
    let ROUND_KEYS = [
        88u32, 56u32, 960u32, 208u32, 288u32, 20u32, 96u32, 44u32, 896u32, 240u32, 416u32, 18u32,
    ];

    for i in 0usize..12usize {
        st = round(st, ROUND_KEYS[i])
    }
    st
}

fn squeeze(st: [u8; 48]) -> [u8; 16] {
    let mut st = u8_to_u32_arr(st);
    st = permute(st);
    [
        st[0] as u8,
        (st[0] >> 8u8) as u8,
        (st[0] >> 16u8) as u8,
        (st[0] >> 24u8) as u8,
        st[1] as u8,
        (st[1] >> 8u8) as u8,
        (st[1] >> 16u8) as u8,
        (st[1] >> 24u8) as u8,
        st[2] as u8,
        (st[2] >> 8u8) as u8,
        (st[2] >> 16u8) as u8,
        (st[2] >> 24u8) as u8,
        st[3] as u8,
        (st[3] >> 8u8) as u8,
        (st[3] >> 16u8) as u8,
        (st[3] >> 24u8) as u8,
    ]
}

fn rotate_right(val: u32, rotation: u8) -> u32 {
    (val >> rotation) ^ (val << (32u8 - rotation))
}

fn u8_to_u32_arr(st: [u8; 48]) -> [u32; 12] {
    let mut arr = [0u32; 12];
    for i in 0usize..12usize {
        arr[i] = u8_to_u32(st, i * 4usize)
    }
    arr
}

fn u32_to_u8_arr(st: [u32; 12]) -> [u8; 48] {
    let mut arr = [0u8; 48];
    for i in 0usize..12usize {
        arr[i * 4usize] = st[0] as u8;
        arr[i * 4usize + 1usize] = (st[0] >> 8u8) as u8;
        arr[i * 4usize + 2usize] = (st[0] >> 16u8) as u8;
        arr[i * 4usize + 3usize] = (st[0] >> 24u8) as u8;
    }
    arr
}

fn u8_to_u32(st: [u8; 48], base_idx: usize) -> u32 {
    st[base_idx] as u32
        ^ ((st[base_idx + 1usize] as u32) << 8u8)
        ^ ((st[base_idx + 2usize] as u32) << 16u8)
        ^ ((st[base_idx + 3usize] as u32) << 24u8)
}

fn add_byte(mut st: [u8; 48], byte: u8, offset: usize) -> [u8; 48] {
    st[offset] = st[offset] ^ byte;
    st
}

fn add_bytes(mut st: [u8; 48], chunk: [u8; 16]) -> [u8; 48] {
    for i in 0usize..16usize {
        st = add_byte(st, chunk[i], i);
    }
    st
}
