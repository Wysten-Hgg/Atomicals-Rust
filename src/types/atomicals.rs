use serde::{Serialize, Deserialize};
use serde_json::Value;
use bitcoin::Script;

/// Atomicals 协议支持的操作类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AtomicalsOperation {
    Ft,     // 固定供应量代币
    Dft,    // 可动态铸造的代币
    Nft,    // 非同质化代币
    Dmt,    // 可动态铸造的NFT
    Dat,    // 数据存储
    Mod,    // 修改操作
    Evt,    // 事件
    #[serde(rename = "sl")]
    Seal,   // 封装
}

/// Atomicals 协议的 Payload 结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomicalsPayload {
    /// 操作类型
    pub op: AtomicalsOperation,
    
    /// Ticker 名称 (用于 FT/DFT)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tick: Option<String>,
    
    /// 发行总量 (用于 FT)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amt: Option<u64>,
    
    /// 元数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Value>,
    
    /// 参数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Value>,
    
    /// 初始化数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub init: Option<Value>,
    
    /// 上下文数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ctx: Option<Value>,
}

impl AtomicalsPayload {
    /// 创建一个新的 FT 铸造 payload
    pub fn new_ft(ticker: String, amount: u64) -> Self {
        Self {
            op: AtomicalsOperation::Ft,
            tick: Some(ticker),
            amt: Some(amount),
            meta: None,
            args: None,
            init: None,
            ctx: None,
        }
    }

    /// 创建一个新的 FT 铸造 payload (已存在的 FT)
    pub fn new_mint_ft(ticker: String) -> Self {
        Self {
            op: AtomicalsOperation::Ft,
            tick: Some(ticker),
            amt: None,
            meta: None,
            args: None,
            init: None,
            ctx: None,
        }
    }

    /// 设置元数据
    pub fn with_meta(mut self, meta: Value) -> Self {
        self.meta = Some(meta);
        self
    }

    /// 设置参数
    pub fn with_args(mut self, args: Value) -> Self {
        self.args = Some(args);
        self
    }

    /// 设置初始化数据
    pub fn with_init(mut self, init: Value) -> Self {
        self.init = Some(init);
        self
    }

    /// 设置上下文数据
    pub fn with_ctx(mut self, ctx: Value) -> Self {
        self.ctx = Some(ctx);
        self
    }
}

/// Atomicals 协议的常量定义
pub const ATOMICALS_PROTOCOL_ENVELOPE: &[u8] = b"atom";
pub const ATOMICALS_PROTOCOL_VERSION: u8 = 1;
