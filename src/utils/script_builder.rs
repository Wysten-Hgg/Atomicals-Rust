use bitcoin::ScriptBuf;
use bitcoin::opcodes::all::*;
use bitcoin::script::PushBytesBuf;
use crate::types::atomicals::AtomicalsPayload;
use crate::errors::{Error, Result};
use serde_json::to_vec;

/// 构建 Atomicals 协议的 OP_RETURN 输出
pub fn build_atomicals_op_return(payload: &AtomicalsPayload) -> Result<ScriptBuf> {
    // 序列化 payload 为 JSON
    let payload_json = to_vec(payload)?;
    
    // 构建 OP_RETURN 输出
    let mut builder = ScriptBuf::builder();
    
    // OP_RETURN
    builder = builder.push_opcode(OP_RETURN);
    
    // 添加协议标识 "atom"
    builder = builder.push_slice(b"atom");
    
    // 添加协议版本
    builder = builder.push_slice(&[0x01]);
    
    // 添加 payload
    // 由于 payload_json 可能超过 PushBytes 的最大长度限制，我们需要分块处理
    let max_chunk_size = 520; // Bitcoin 脚本的最大 push 大小
    for chunk in payload_json.chunks(max_chunk_size) {
        let mut push_buf = PushBytesBuf::new();
        push_buf.extend_from_slice(chunk)
            .map_err(|_| Error::InvalidAmount("Payload chunk too large".into()))?;
        builder = builder.push_slice(&push_buf);
    }
    
    Ok(builder.into_script())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_build_atomicals_op_return() {
        let payload = AtomicalsPayload {
            op: crate::types::atomicals::AtomicalsOperation::Ft,
            tick: Some("TEST".to_string()),
            amt: None,
            meta: Some(json!({
                "name": "Test Token",
                "description": "A test token"
            })),
            args: None,
            init: None,
            ctx: None,
        };

        let script = build_atomicals_op_return(&payload).unwrap();
        
        // 验证脚本开始于 OP_RETURN
        assert_eq!(script.as_bytes()[0], OP_RETURN.into_u8());
        
        // 验证协议标识
        assert_eq!(&script.as_bytes()[1..5], b"atom");
        
        // 验证协议版本
        assert_eq!(script.as_bytes()[5], 0x01);
    }
}
