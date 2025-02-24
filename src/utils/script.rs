use bitcoin::{
	opcodes::{
		all::{OP_CHECKSIG, OP_ENDIF, OP_IF, OP_RETURN},
		OP_0,
	},
	script::PushBytes,
	secp256k1::Keypair,
	Address, PrivateKey, Script, ScriptBuf, XOnlyPublicKey,
};
use std::error::Error as StdError;
use std::fmt;

#[derive(Debug)]
pub enum Error {
    Script(String),
    Other(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Script(e) => write!(f, "Script error: {}", e),
            Error::Other(s) => write!(f, "{}", s),
        }
    }
}

impl StdError for Error {}

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
