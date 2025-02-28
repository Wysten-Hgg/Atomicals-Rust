use serde::{Serialize, Deserialize};
use bitcoin::Amount;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealmConfig {
    /// Realm名称
    pub name: String,
    /// Commit交易的工作量证明要求
    pub bitworkc: Option<String>,
    /// Reveal交易的工作量证明要求
    pub bitworkr: Option<String>,
    /// 可选的容器关系
    pub container: Option<String>,
    /// 可选的父Realm
    pub parent: Option<String>,
    /// 可选的父Realm所有者
    pub parent_owner: Option<String>,
    /// 输出金额（聪）
    pub sats_output: u64,
}

impl RealmConfig {
    pub fn new(name: String) -> Self {
        Self {
            name,
            bitworkc: None,
            bitworkr: None,
            container: None,
            parent: None,
            parent_owner: None,
            sats_output: 546, // 最小粉尘限额
        }
    }

    /// 验证Realm名称格式
    pub fn validate_name(&self) -> Result<(), &'static str> {
        // 名称长度检查（建议3-63个字符）
        if self.name.len() < 3 || self.name.len() > 63 {
            return Err("Realm name must be between 3 and 63 characters");
        }

        // 字符集检查（只允许小写字母、数字和连字符）
        if !self.name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
            return Err("Realm name can only contain lowercase letters, numbers, and hyphens");
        }

        // 不能以连字符开头或结尾
        if self.name.starts_with('-') || self.name.ends_with('-') {
            return Err("Realm name cannot start or end with a hyphen");
        }

        // 不能有连续的连字符
        if self.name.contains("--") {
            return Err("Realm name cannot contain consecutive hyphens");
        }

        Ok(())
    }

    /// 设置commit交易的工作量证明
    pub fn with_bitworkc(mut self, bitworkc: String) -> Self {
        self.bitworkc = Some(bitworkc);
        self
    }

    /// 设置reveal交易的工作量证明
    pub fn with_bitworkr(mut self, bitworkr: String) -> Self {
        self.bitworkr = Some(bitworkr);
        self
    }

    /// 设置容器
    pub fn with_container(mut self, container: String) -> Self {
        self.container = Some(container);
        self
    }

    /// 设置父Realm
    pub fn with_parent(mut self, parent: String, parent_owner: Option<String>) -> Self {
        self.parent = Some(parent);
        self.parent_owner = parent_owner;
        self
    }

    /// 设置输出金额
    pub fn with_sats_output(mut self, sats: u64) -> Self {
        self.sats_output = sats;
        self
    }
}
