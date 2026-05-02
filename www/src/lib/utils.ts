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
        `${instruction.opcode}${instruction.argument === undefined ? '' : ` ${formatInstructionArgument(instruction.argument)}`}`
    )
    .join('\n');
}

/**
 * Formats an instruction argument for the bytecode listing.
 *
 * Scalar arguments are displayed directly. Object arguments, which are used by
 * instructions with named fields, are displayed as `key=value` pairs so the
 * listing remains readable.
 *
 * @param argument Instruction argument from serialized bytecode.
 * @returns A compact string representation for display.
 */
function formatInstructionArgument(argument: unknown): string {
  if (typeof argument === 'object' && argument !== null) {
    return Object.entries(argument)
      .map(([key, value]) => `${key}=${value}`)
      .join(' ');
  }

  return String(argument);
}
