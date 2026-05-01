import init, { type Execution, compile, execute } from '@/wasm/taipan_wasm';
import wasmUrl from '@/wasm/taipan_wasm_bg.wasm?url';

import type { Code } from './types';

let wasm: Promise<void> | undefined;

/**
 * Compile Python source to Taipan bytecode using the shared WASM module.
 *
 * @param source Python source code to compile.
 * @returns Structured bytecode emitted by the Taipan compiler.
 */
export async function compileSource(source: string) {
  await initialize();
  return compile(source) as Code;
}

/**
 * Compile and execute Python source, returning captured stdout and result text.
 *
 * @param source Python source code to execute.
 * @returns Captured stdout and the final VM result string.
 */
export async function executeSource(source: string) {
  await initialize();

  const execution: Execution = execute(source);
  const { output, result } = execution;

  execution.free();

  return { output, result };
}

/**
 * Initialize the WASM module once and share it across compile and execute calls.
 *
 * @returns A promise that resolves once the WASM module is ready.
 */
async function initialize() {
  wasm ??= init({ module_or_path: wasmUrl }).then(() => undefined);
  await wasm;
}
