/* tslint:disable */
/* eslint-disable */
export function mine_nonce_range(tx_wrapper: WasmTransaction, start_nonce: number, end_nonce: number, bitwork: string): number | undefined;
export function mine_transaction(tx_wrapper: WasmTransaction, bitwork_wrapper: WasmBitworkInfo, options: MiningOptions): Promise<any>;
export class AtomicalsWasm {
  free(): void;
  constructor();
  mint_ft(tick: string, mint_amount: bigint, bitwork_c?: string | null, bitwork_r?: string | null, num_workers?: number | null, batch_size?: number | null): Promise<any>;
  mint_realm(name: string, sats_output: bigint, bitwork_c?: string | null, bitwork_r?: string | null, container?: string | null, parent?: string | null, parent_owner?: string | null, num_workers?: number | null, batch_size?: number | null): Promise<any>;
  mint_subrealm(name: string, parent_realm_id: string, claim_type: string, sats_output: bigint, bitwork_c?: string | null, bitwork_r?: string | null, container?: string | null, meta?: string | null, ctx?: string | null, init?: string | null, num_workers?: number | null, batch_size?: number | null): Promise<any>;
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
export class WasmRealmConfig {
  free(): void;
  constructor(name: string);
  with_bitworkc(bitworkc: string): WasmRealmConfig;
  with_bitworkr(bitworkr: string): WasmRealmConfig;
  with_container(container: string): WasmRealmConfig;
  with_parent(parent: string, parent_owner?: string | null): WasmRealmConfig;
  with_sats_output(sats: bigint): WasmRealmConfig;
  validate(): void;
  readonly name: string;
  readonly bitworkc: string | undefined;
  readonly bitworkr: string | undefined;
  readonly container: string | undefined;
  readonly parent: string | undefined;
  readonly parent_owner: string | undefined;
  readonly sats_output: bigint;
}
export class WasmTransaction {
  free(): void;
  constructor(hex: string);
  to_hex(): string;
  static from_hex(hex: string): WasmTransaction;
  set_sequence(nonce: number): boolean;
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
  readonly wasmtransaction_set_sequence: (a: number, b: number) => number;
  readonly __wbg_wasmbitworkinfo_free: (a: number, b: number) => void;
  readonly wasmbitworkinfo_new: (a: number, b: number, c: number, d: number) => number;
  readonly wasmbitworkinfo_get_difficulty: (a: number) => [number, number];
  readonly wasmbitworkinfo_get_prefix: (a: number) => [number, number];
  readonly wasmbitworkinfo_get_ext: (a: number) => [number, number];
  readonly wasmbitworkinfo_set_ext: (a: number, b: number, c: number) => void;
  readonly __wbg_wasmrealmconfig_free: (a: number, b: number) => void;
  readonly wasmrealmconfig_new: (a: number, b: number) => number;
  readonly wasmrealmconfig_name: (a: number) => [number, number];
  readonly wasmrealmconfig_bitworkc: (a: number) => [number, number];
  readonly wasmrealmconfig_bitworkr: (a: number) => [number, number];
  readonly wasmrealmconfig_container: (a: number) => [number, number];
  readonly wasmrealmconfig_parent: (a: number) => [number, number];
  readonly wasmrealmconfig_parent_owner: (a: number) => [number, number];
  readonly wasmrealmconfig_sats_output: (a: number) => bigint;
  readonly wasmrealmconfig_with_bitworkc: (a: number, b: number, c: number) => number;
  readonly wasmrealmconfig_with_bitworkr: (a: number, b: number, c: number) => number;
  readonly wasmrealmconfig_with_container: (a: number, b: number, c: number) => number;
  readonly wasmrealmconfig_with_parent: (a: number, b: number, c: number, d: number, e: number) => number;
  readonly wasmrealmconfig_with_sats_output: (a: number, b: bigint) => number;
  readonly wasmrealmconfig_validate: (a: number) => [number, number];
  readonly __wbg_unisatprovider_free: (a: number, b: number) => void;
  readonly unisatprovider_try_new: () => [number, number, number];
  readonly __wbg_wizzprovider_free: (a: number, b: number) => void;
  readonly wizzprovider_try_new: () => [number, number, number];
  readonly __wbg_atomicalswasm_free: (a: number, b: number) => void;
  readonly atomicalswasm_try_new: () => [number, number, number];
  readonly atomicalswasm_mint_ft: (a: number, b: number, c: number, d: bigint, e: number, f: number, g: number, h: number, i: number, j: number) => any;
  readonly atomicalswasm_mint_realm: (a: number, b: number, c: number, d: bigint, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number, m: number, n: number, o: number, p: number) => any;
  readonly atomicalswasm_mint_subrealm: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: bigint, i: number, j: number, k: number, l: number, m: number, n: number, o: number, p: number, q: number, r: number, s: number, t: number, u: number, v: number) => any;
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
  readonly _dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h34ced8d1dcb0005d: (a: number, b: number) => void;
  readonly closure247_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure311_externref_shim: (a: number, b: number, c: any, d: any) => void;
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
