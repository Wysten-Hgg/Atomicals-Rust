fn main() {
    // 告诉 Rust 编译器，如果任何这些文件发生变化，就重新运行构建脚本
    println!("cargo:rerun-if-changed=src/workers/mining_worker.rs");
    println!("cargo:rerun-if-changed=src/operations/mining.rs");
    
    // 设置 WASM 特定的编译标志
    #[cfg(target_arch = "wasm32")]
    {
        println!("cargo:rustc-cfg=target_arch=\"wasm32\"");
    }
}
