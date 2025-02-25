use bitcoin::{
    opcodes::{
        all::{OP_CHECKSIG, OP_ENDIF, OP_IF, OP_RETURN},
        OP_0,
    },
    script::PushBytes,
    script::Script,
    script::ScriptBuf,
    secp256k1::Keypair,
    Address, PrivateKey, XOnlyPublicKey,
};
use std::error::Error as StdError;
use std::fmt;
use wasm_bindgen::prelude::*;
use web_sys::{window, Performance};
use js_sys::Math;
use serde::Serialize;

#[derive(Debug)]
pub enum Error {
    Script(String),
    Other(String),
    Serialization(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Script(e) => write!(f, "Script error: {}", e),
            Error::Other(s) => write!(f, "{}", s),
            Error::Serialization(s) => write!(f, "Serialization error: {}", s),
        }
    }
}

impl StdError for Error {}

impl From<ciborium::ser::Error<std::io::Error>> for Error {
    fn from(err: ciborium::ser::Error<std::io::Error>) -> Self {
        Error::Serialization(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn append_mint_update_reveal_script(
    op_type: &str,
    child_node_xonly_pubkey: &XOnlyPublicKey,
    payload: &[u8]
) -> Result<ScriptBuf> {
    let b = Script::builder()
    .push_x_only_key(child_node_xonly_pubkey)
    .push_opcode(OP_CHECKSIG)
    .push_opcode(OP_0)
    .push_opcode(OP_IF)
    .push_slice(<&PushBytes>::try_from("atom".as_bytes()).unwrap())
    .push_slice(<&PushBytes>::try_from(op_type.as_bytes()).unwrap());
    let script = payload
        .chunks(520)
        .try_fold(b, |b, c| {
            Ok::<_, Error>(b.push_slice(<&PushBytes>::try_from(c).map_err(|e| Error::Script(e.to_string()))?))
        })?
        .push_opcode(OP_ENDIF)
        .into_script();

    Ok(script)
}

#[cfg(target_arch = "wasm32")]
pub fn time_nonce() -> (u64, u64) {
    let window = window().expect("should have a window in this context");
    let date = js_sys::Date::new_0();
    let now = (date.get_time() / 1000.0) as u64;  // 转换为秒
    let random = (Math::random() * 10_000_000.0) as u64;
    (now, random)
}

#[cfg(target_arch = "wasm32")]
pub fn cbor<T>(v: &T) -> Result<Vec<u8>>
where
    T: Serialize,
{
    let mut cbor = Vec::new();
    ciborium::ser::into_writer(v, &mut cbor)
        .map_err(|e| Error::Serialization(e.to_string()))?;
    Ok(cbor)
}