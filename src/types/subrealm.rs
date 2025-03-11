use serde::{Serialize, Deserialize};
use bitcoin::Amount;
use bitcoin::ScriptBuf;

/// Subrealm 铸造类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SubrealmClaimType {
    /// 直接铸造（拥有父 Realm）
    Direct,
    /// 规则铸造（不拥有父 Realm）
    Rule,
}

impl SubrealmClaimType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SubrealmClaimType::Direct => "direct",
            SubrealmClaimType::Rule => "rule",
        }
    }
}

/// Subrealm 规则输出
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleOutput {
    /// 支付金额（聪）
    pub v: u64,
    /// 可选的 token id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

/// Subrealm 规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubrealmRule {
    /// 规则模式（正则表达式）
    pub p: String,
    /// 输出规则映射 (script -> RuleOutput)
    pub o: std::collections::HashMap<String, RuleOutput>,
    /// Commit 交易的工作量证明要求
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitworkc: Option<String>,
    /// Reveal 交易的工作量证明要求
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitworkr: Option<String>,
}

/// Subrealm 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubrealmConfig {
    /// Subrealm 完整名称，如 "example.sub"
    pub name: String,
    /// 父 Realm 的 Atomical ID
    pub parent_realm_id: String,
    /// 铸造类型：直接或规则
    pub claim_type: SubrealmClaimType,
    /// Commit 交易的工作量证明要求
    pub bitworkc: Option<String>,
    /// Reveal 交易的工作量证明要求
    pub bitworkr: Option<String>,
    /// 可选的容器关系
    pub container: Option<String>,
    /// 输出金额（聪）
    pub sats_output: u64,
    /// 可选的元数据
    pub meta: Option<serde_json::Value>,
    /// 可选的上下文数据
    pub ctx: Option<serde_json::Value>,
    /// 可选的初始化数据
    pub init: Option<serde_json::Value>,
    /// 手续费率（聪/字节）
    pub fee_rate: Option<f64>,
    /// 规则铸造的支付输出
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_outputs: Option<Vec<(ScriptBuf, RuleOutput)>>,
}

impl SubrealmConfig {
    pub fn new(name: String, parent_realm_id: String, claim_type: SubrealmClaimType) -> Self {
        Self {
            name,
            parent_realm_id,
            claim_type,
            bitworkc: None,
            bitworkr: None,
            container: None,
            sats_output: 546, // 最小粉尘限额
            meta: None,
            ctx: None,
            init: None,
            fee_rate: None,
            rule_outputs: None,
        }
    }

    /// 验证 Subrealm 名称格式
    pub fn validate_name(&self) -> Result<(), &'static str> {
        // 检查是否包含点号（必须是子域名）
        if !self.name.contains('.') {
            return Err("Subrealm must contain a dot separator");
        }
        
        // 分割并验证子域名部分
        let parts: Vec<&str> = self.name.split('.').collect();
        let subrealm_part = parts[parts.len() - 1];
        
        // 名称长度检查
        if subrealm_part.len() < 1 || subrealm_part.len() > 64 {
            return Err("Subrealm name must be between 1 and 64 characters");
        }

        // 字符集检查（只允许小写字母、数字和连字符）
        if !subrealm_part.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
            return Err("Subrealm name can only contain lowercase letters, numbers, and hyphens");
        }

        // 不能以连字符开头或结尾
        if subrealm_part.starts_with('-') || subrealm_part.ends_with('-') {
            return Err("Subrealm name cannot start or end with a hyphen");
        }

        Ok(())
    }

    /// 设置规则铸造的支付输出
    pub fn with_rule_outputs(mut self, outputs: Vec<(ScriptBuf, RuleOutput)>) -> Self {
        self.rule_outputs = Some(outputs);
        self
    }

    /// 设置 commit 交易的工作量证明
    pub fn with_bitworkc(mut self, bitworkc: String) -> Self {
        self.bitworkc = Some(bitworkc);
        self
    }

    /// 设置 reveal 交易的工作量证明
    pub fn with_bitworkr(mut self, bitworkr: String) -> Self {
        self.bitworkr = Some(bitworkr);
        self
    }

    /// 设置容器
    pub fn with_container(mut self, container: String) -> Self {
        self.container = Some(container);
        self
    }

    /// 设置输出金额
    pub fn with_sats_output(mut self, sats: u64) -> Self {
        self.sats_output = sats;
        self
    }

    /// 设置元数据
    pub fn with_meta(mut self, meta: serde_json::Value) -> Self {
        self.meta = Some(meta);
        self
    }

    /// 设置上下文数据
    pub fn with_ctx(mut self, ctx: serde_json::Value) -> Self {
        self.ctx = Some(ctx);
        self
    }

    /// 设置初始化数据
    pub fn with_init(mut self, init: serde_json::Value) -> Self {
        self.init = Some(init);
        self
    }

    /// 设置手续费率
    pub fn with_fee_rate(mut self, fee_rate: f64) -> Self {
        self.fee_rate = Some(fee_rate);
        self
    }
}
