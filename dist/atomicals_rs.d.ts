/* tslint:disable */
/* eslint-disable */
export function mine_nonce_range(tx_wrapper: WasmTransaction, start_nonce: number, end_nonce: number, bitwork: string): number | undefined;
export function mine_transaction(tx_wrapper: WasmTransaction, bitwork_wrapper: WasmBitworkInfo, options: MiningOptions): Promise<any>;
export class AtomicalsWasm {
  free(): void;
  constructor();
  mint_ft(tick: string, mint_amount: bigint, bitwork_c?: string | null, bitwork_r?: string | null, num_workers?: number | null, batch_size?: number | null): Promise<any>;
}
export class MiningOptions {
  free(): void;
  constructor();
  num_workers: number;
  batch_size: number;
}
export class UnisatProvider {
  free(): void;
  constructor();
}
export class WasmBitworkInfo {
  free(): void;
  constructor(difficulty: string, prefix: string);
  get_difficulty(): string;
  get_prefix(): string;
  get_ext(): string | undefined;
  set_ext(ext?: string | null): void;
}
export class WasmTransaction {
  free(): void;
  constructor(hex: string);
  to_hex(): string;
  static from_hex(hex: string): WasmTransaction;
}
export class WizzProvider {
  free(): void;
  constructor();
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_miningoptions_free: (a: number, b: number) => void;
  readonly __wbg_get_miningoptions_num_workers: (a: number) => number;
  readonly __wbg_set_miningoptions_num_workers: (a: number, b: number) => void;
  readonly __wbg_get_miningoptions_batch_size: (a: number) => number;
  readonly __wbg_set_miningoptions_batch_size: (a: number, b: number) => void;
  readonly miningoptions_new: () => number;
  readonly mine_nonce_range: (a: number, b: number, c: number, d: number, e: number) => number;
  readonly mine_transaction: (a: number, b: number, c: number) => any;
  readonly __wbg_wasmtransaction_free: (a: number, b: number) => void;
  readonly wasmtransaction_to_hex: (a: number) => [number, number];
  readonly wasmtransaction_from_hex: (a: number, b: number) => number;
  readonly __wbg_wasmbitworkinfo_free: (a: number, b: number) => void;
  readonly wasmbitworkinfo_new: (a: number, b: number, c: number, d: number) => number;
  readonly wasmbitworkinfo_get_difficulty: (a: number) => [number, number];
  readonly wasmbitworkinfo_get_prefix: (a: number) => [number, number];
  readonly wasmbitworkinfo_get_ext: (a: number) => [number, number];
  readonly wasmbitworkinfo_set_ext: (a: number, b: number, c: number) => void;
  readonly __wbg_unisatprovider_free: (a: number, b: number) => void;
  readonly unisatprovider_try_new: () => [number, number, number];
  readonly __wbg_wizzprovider_free: (a: number, b: number) => void;
  readonly wizzprovider_try_new: () => [number, number, number];
  readonly __wbg_atomicalswasm_free: (a: number, b: number) => void;
  readonly atomicalswasm_try_new: () => [number, number, number];
  readonly atomicalswasm_mint_ft: (a: number, b: number, c: number, d: bigint, e: number, f: number, g: number, h: number, i: number, j: number) => any;
  readonly wasmtransaction_new: (a: number, b: number) => number;
  readonly rustsecp256k1_v0_9_2_context_create: (a: number) => number;
  readonly rustsecp256k1_v0_9_2_context_destroy: (a: number) => void;
  readonly rustsecp256k1_v0_9_2_default_illegal_callback_fn: (a: number, b: number) => void;
  readonly rustsecp256k1_v0_9_2_default_error_callback_fn: (a: number, b: number) => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_export_4: WebAssembly.Table;
  readonly __wbindgen_export_5: WebAssembly.Table;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __externref_table_dealloc: (a: number) => void;
  readonly closure5_externref_shim: (a: number, b: number, c: any) => void;
  readonly _dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__ha6c0ae6e289e50fc: (a: number, b: number) => void;
  readonly closure240_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure298_externref_shim: (a: number, b: number, c: any, d: any) => void;
  readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
