use bs58::{decode, encode};
use uuid::{Builder, Uuid};

pub fn decode_uuid(encoded: &str) -> decode::Result<Uuid> {
    let decoded: [u8; 16] = [0; 16];
    decode(encoded).onto(decoded)?;
    Ok(Builder::from_bytes(decoded).into_uuid())
}

pub fn encode_uuid(uuid: Uuid) -> String {
    encode(uuid.as_bytes()).into_string()
}
