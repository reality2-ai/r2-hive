// TypeScript surface for the #26 browser WASM-WS binding (ws-mesh/hive-ws-browser.js).
// The WasmHive/free-fn types (route_frame, verifyFrame, build_frame, frame_origin, transport_profile,
// quality_from_rssi, range_to_loss_db, setGroupHmac …) come from the wasm-pack --target web d.ts
// (pkg/r2_hive_wasm.d.ts) — pass that initialised module as `wasm` below.

/** The wasm-pack module object (after `await init()`): has `WasmHive` + the free fns. */
export interface WasmModule {
  WasmHive: new (hiveId: number) => any;
  frame_origin(frame: Uint8Array): number;
  // also: quality_from_rssi, range_to_loss_db, transport_profile, version …
  [k: string]: any;
}

/** Result of `verifyFrame` — the real r2_trust deliver-gate. */
export interface Gate { keyed: boolean; tg_ok: boolean; hmac_ok: boolean; deliver: boolean; }

/** Result of `route_frame` — the forwarding decision (separate from delivery). */
export interface RouteOut {
  outcome: 'NotR2Wire' | 'Dropped' | 'DeliverOnly' | 'Directed' | 'Flooded';
  sent: number;
  sends: Array<{ kind: number; target: number; frame: string /* hex */ }>;
}

export interface HiveWsOpts {
  /** The initialised wasm-pack --target web module (`await init()` first). */
  wasm: WasmModule;
  /** This hive's id (u32). */
  hiveId: number;
  /** Gateway URL, e.g. `ws://127.0.0.1:21055`. */
  url: string;
  /** 32-byte GroupHmac key (the nodes' shared hk). Omit ⇒ TG-agnostic (accepts all — warns). */
  hk?: Uint8Array | number[];
  /** Trust-group hash (target_group). Required iff `hk` is set. */
  tgHash?: number;
  /** Called when the deliver-gate accepts a frame for this hive's TG. */
  onDeliver?: (hiveId: number, frame: Uint8Array, gate: Gate) => void;
  /** Called with every route_frame forwarding decision. */
  onRoute?: (hiveId: number, out: RouteOut) => void;
}

/** WS bearer routing-kind (1 = Wifi; tags the link for transport-aware routing). */
export const WS_KIND: number;

/**
 * A WasmHive wired to a real WebSocket bearer. Inbound: frame_origin echo-drop → verifyFrame
 * (deliver-gate) → route_frame (forwarding, relays sends[] back onto the bearer). Outbound:
 * originate()/buildFrame()/buildHeartbeat(). WS frames are BINARY raw R2-WIRE bytes.
 */
export class HiveWs {
  constructor(opts: HiveWsOpts);
  readonly id: number;
  readonly keyed: boolean;
  connect(): Promise<HiveWs>;
  /** Emit raw R2-WIRE bytes onto the bearer (gateway broadcasts to the other hives). */
  originate(bytes: Uint8Array | number[]): void;
  buildFrame(targetHive: number, eventHash: number, payload: Uint8Array | number[], seq?: number): Uint8Array;
  buildHeartbeat(seq?: number): Uint8Array;
  close(): void;
}
