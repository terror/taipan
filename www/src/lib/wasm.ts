import init, { type Execution, compile, execute } from '@/wasm/taipan_wasm';
import wasmUrl from '@/wasm/taipan_wasm_bg.wasm?url';

let wasm: Promise<void> | undefined;

export async function compileSource(source: string) {
  await initialize();

  return compile(source);
}

export async function executeSource(source: string) {
  await initialize();

  const execution: Execution = execute(source);
  const output = execution.output;
  const result = execution.result;

  execution.free();

  return { output, result };
}

async function initialize() {
  wasm ??= init({ module_or_path: wasmUrl }).then(() => undefined);

  await wasm;
}
