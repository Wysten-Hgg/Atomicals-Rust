// Worker entry point
(function() {
    let initialized = false;
    let wasmModule = null;
    let initializationPromise = null;
    const MAX_RETRIES = 3;
    const RETRY_DELAY = 1000; // 1 second
    const MAX_SEQUENCE = 0xffffffff;
    const BATCH_SIZE = 100000; // 增加批次大小

    function log(message, ...args) {
        console.log('[Worker]', message, ...args);
    }
    
    function error(message, ...args) {
        console.error('[Worker Error]', message, ...args);
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
            const module = await import('./atomicals_rs.js');
            log('Successfully loaded atomicals_rs.js');

            // 初始化 WASM 模块
            await module.default();
            log('WASM module initialized successfully');

            // 验证必要的函数是否存在
            if (typeof module.mine_nonce_range !== 'function') {
                throw new Error('mine_nonce_range function not found in WASM module');
            }

            return module;
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
            
            // 如果收到停止信号，立即停止
            if (e.data.type === 'stop') {
                log('Received stop signal, terminating worker');
                return;
            }
            
            log('Received mining task:', { 
                start_nonce, 
                end_nonce: end_nonce || MAX_SEQUENCE,
                bitwork 
            });
            log('Transaction hex:', tx_wrapper.hex.substring(0, 50) + '...');

            // 创建 WasmTransaction 实例
            const wasmTx = new wasmModule.WasmTransaction(tx_wrapper.hex);
            log('Created WasmTransaction instance');
            
            let currentNonce = start_nonce;
            const targetEnd = end_nonce || MAX_SEQUENCE;

            function mineNextBatch() {
                const batchEnd = Math.min(currentNonce + BATCH_SIZE, targetEnd);
                log(`Mining batch: nonce range ${currentNonce}-${batchEnd}, target prefix: ${bitwork}`);
                
                try {
                    // 调用 WASM 模块的挖矿函数
                    log(`Calling mine_nonce_range with params:`, {
                        currentNonce,
                        batchEnd,
                        bitwork,
                        total_range: targetEnd - start_nonce
                    });

                    const result = wasmModule.mine_nonce_range(
                        wasmTx,
                        currentNonce,
                        batchEnd,
                        bitwork
                    );

                    log(`Batch result: ${result} (type: ${typeof result})`);

                    if (result !== undefined && result !== null) {
                        log(`Found valid nonce: ${result}, matches required prefix: ${bitwork}`);
                        self.postMessage({
                            type: 'success',
                            success: true,
                            nonce: result,
                            tx: tx_wrapper,
                            finished: true
                        });
                        return;  // 找到有效 nonce 后立即返回
                    } else {
                        log(`No valid nonce found in range ${currentNonce}-${batchEnd} for prefix ${bitwork}`);
                        currentNonce = batchEnd;
                        
                        if (currentNonce < targetEnd) {
                            const progress = Math.floor((currentNonce - start_nonce) / (targetEnd - start_nonce) * 100);
                            log(`Mining progress: ${progress}%, scheduling next batch starting at ${currentNonce}`);
                            self.postMessage({
                                type: 'progress',
                                success: false,
                                nonce: null,
                                tx: null,
                                progress: {
                                    current: currentNonce,
                                    total: targetEnd,
                                    percent: progress
                                }
                            });
                            setTimeout(mineNextBatch, 0);
                        } else {
                            log(`Completed search through entire nonce range (${start_nonce} to ${targetEnd}) without finding a solution`);
                            self.postMessage({
                                type: 'exhausted',
                                success: false,
                                nonce: null,
                                tx: null,
                                finished: true,
                                exhausted: true
                            });
                        }
                    }
                } catch (err) {
                    error(`Error in batch ${currentNonce}-${batchEnd}:`, err);
                    error('Error details:', {
                        message: err.message,
                        stack: err.stack,
                        currentNonce,
                        batchEnd,
                        bitwork
                    });
                    self.postMessage({
                        type: 'error',
                        success: false,
                        nonce: null,
                        tx: null,
                        error: err.message
                    });
                }
            }

            // 开始挖矿
            log('Starting mining process with configuration:', {
                start_nonce,
                end_nonce: targetEnd,
                bitwork,
                batch_size: BATCH_SIZE,
                max_sequence: MAX_SEQUENCE
            });
            mineNextBatch();
        } catch (err) {
            error('Error processing mining task:', err);
            self.postMessage({
                type: 'error',
                success: false,
                nonce: null,
                tx: null,
                error: err.message
            });
        }
    };
})();
