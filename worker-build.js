const fs = require('fs');
const path = require('path');

// 确保 pkg 目录存在
if (!fs.existsSync('pkg')) {
    fs.mkdirSync('pkg');
}

// 复制并重命名 worker 文件
fs.copyFileSync(
    path.join('pkg', 'atomicals_rs.js'),
    path.join('pkg', 'atomicals_rs_worker.js')
);

// 修改 worker 文件以支持 Worker 环境
const workerContent = fs.readFileSync(
    path.join('pkg', 'atomicals_rs_worker.js'),
    'utf8'
);

const modifiedContent = `
// Worker 环境初始化
import * as wasm from './atomicals_rs_bg.wasm';
import { worker_entry } from './atomicals_rs.js';

// 初始化 WASM
wasm_bindgen('./atomicals_rs_bg.wasm')
    .then(() => {
        // 启动 worker
        worker_entry();
    })
    .catch(e => {
        console.error('Failed to initialize WASM worker:', e);
    });
`;

fs.writeFileSync(
    path.join('pkg', 'atomicals_rs_worker.js'),
    modifiedContent
);
