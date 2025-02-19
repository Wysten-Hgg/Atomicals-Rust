// Worker entry point
(function() {
    let initialized = false;
    let wasmModule = null;
    let initializationPromise = null;
    const MAX_RETRIES = 3;
    const RETRY_DELAY = 1000; // 1 second

    function log(msg, data) {
        console.log(`[Worker] ${msg}`, data || '');
    }

    function error(msg, err) {
        console.error(`[Worker] ${msg}`, err || '');
    }

    // Set up global error handler
    self.onerror = function(err) {
        error('Global error:', err);
    };

    // 加载 WASM 模块的函数
    async function loadWasmModule(attempt = 1) {
        log(`Loading WASM module (attempt ${attempt}/${MAX_RETRIES})...`);
        
        try {
            // 使用动态 import 加载模块
            log('Loading atomicals_rs.js...');
            const wasmModule = await import('./atomicals_rs.js');
            log('Successfully loaded atomicals_rs.js');

            // 初始化 WASM 模块
            await wasmModule.default();
            log('WASM module initialized successfully');

            // 验证必要的函数是否存在
            if (typeof wasmModule.mine_nonce_range !== 'function') {
                throw new Error('mine_nonce_range function not found in WASM module');
            }

            return wasmModule;
        } catch (err) {
            error(`Failed to load WASM module (attempt ${attempt}):`, err);
            
            if (attempt < MAX_RETRIES) {
                log(`Retrying in ${RETRY_DELAY}ms...`);
                await new Promise(resolve => setTimeout(resolve, RETRY_DELAY));
                return loadWasmModule(attempt + 1);
            }
            
            throw new Error(`Failed to load WASM module after ${MAX_RETRIES} attempts: ${err.message}`);
        }
    }

    // 初始化 Worker
    async function initializeWorker() {
        if (initialized) {
            return;
        }

        try {
            wasmModule = await loadWasmModule();
            initialized = true;
            log('Worker initialized successfully');
        } catch (err) {
            error('Failed to initialize worker:', err);
            initialized = false;
            wasmModule = null;
            throw err;
        }
    }

    // 启动初始化
    initializationPromise = initializeWorker();

    // 处理挖矿消息
    self.onmessage = async function(e) {
        try {
            // 等待初始化完成
            await initializationPromise;

            if (!initialized || !wasmModule) {
                throw new Error('Worker not initialized');
            }

            const { tx_wrapper, start_nonce, end_nonce, bitwork } = e.data;
            log('Received mining task:', { start_nonce, end_nonce, bitwork });

            // 创建 WasmTransaction 实例
            const wasmTx = new wasmModule.WasmTransaction(tx_wrapper.hex);
            
            // 执行挖矿
            const result = wasmModule.mine_nonce_range(
                wasmTx,
                start_nonce,
                end_nonce,
                bitwork
            );

            // 发送结果
            if (result !== null) {
                log('Found valid nonce:', result);
                self.postMessage({
                    success: true,
                    nonce: result,
                    tx: null
                });
            } else {
                log('No valid nonce found in range');
                self.postMessage({
                    success: false,
                    nonce: null,
                    tx: null
                });
            }
        } catch (err) {
            error('Error processing mining task:', err);
            self.postMessage({
                success: false,
                nonce: null,
                tx: null,
                error: err.message
            });
        }
    };
})();
