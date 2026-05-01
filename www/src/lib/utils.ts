import { type ClassValue, clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

import type { Code } from './types';

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/**
 * Formats compiler bytecode as the compact instruction listing shown in the
 * playground.
 *
 * @param code Compiler bytecode returned by the WASM module.
 * @returns A newline-delimited instruction listing.
 */
export function formatBytecode(code: Code): string {
  return code.instructions
    .map(
      (instruction) =>
        `${instruction.opcode}${instruction.argument === undefined ? '' : ` ${instruction.argument}`}`
    )
    .join('\n');
}
