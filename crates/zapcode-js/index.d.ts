export interface ZapcodeOptions {
  inputs?: string[];
  externalFunctions?: string[];
  memoryLimitMb?: number;
  timeLimitMs?: number;
}

export interface ZapcodeSessionOptions {
  externalFunctions?: string[];
  memoryLimitMb?: number;
  timeLimitMs?: number;
}

export interface ZapcodeTraceSpan {
  name: string;
  startTimeMs: number;
  endTimeMs: number;
  durationUs: number;
  status: string;
  attributes: string[][];
  children: ZapcodeTraceSpan[];
}

export interface ZapcodeResult {
  completed: true;
  output: unknown;
  stdout: string;
  trace: ZapcodeTraceSpan;
}

export interface ZapcodeSuspension {
  completed: false;
  functionName: string;
  args: unknown[];
  snapshot: Buffer;
}

export interface ZapcodeSessionResult {
  completed: true;
  output: unknown;
  stdout: string;
  session: Buffer;
}

export interface ZapcodeSessionSuspension {
  completed: false;
  functionName: string;
  args: unknown[];
  stdout: string;
  session: Buffer;
}

export class ZapcodeSnapshotHandle {
  static load(bytes: Buffer): ZapcodeSnapshotHandle;
  dump(): Buffer;
  resume(returnValue: unknown): ZapcodeResult | ZapcodeSuspension;
}

export class ZapcodeSessionHandle {
  static create(options?: ZapcodeSessionOptions): ZapcodeSessionHandle;
  static load(bytes: Buffer): ZapcodeSessionHandle;
  dump(): Buffer;
  runChunk(
    code: string,
    inputs?: Record<string, unknown>
  ): ZapcodeSessionResult | ZapcodeSessionSuspension;
  resume(returnValue: unknown): ZapcodeSessionResult | ZapcodeSessionSuspension;
}

export class Zapcode {
  constructor(code: string, options?: ZapcodeOptions);
  run(inputs?: Record<string, unknown>): ZapcodeResult;
  start(inputs?: Record<string, unknown>): ZapcodeResult | ZapcodeSuspension;
}
