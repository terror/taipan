export interface Instruction {
  opcode: string;
  argument?: number;
}

export interface CodeObject {
  constants: ObjectValue[];
  freevars: string[];
  instructions: Instruction[];
  locals: string[];
  names: string[];
}

export type ObjectValue =
  | { type: 'bool'; value: boolean }
  | { type: 'builtin'; name: string }
  | {
      type: 'function';
      name: string;
      parameters: string[];
      bytecode: CodeObject;
    }
  | { type: 'float'; value: number }
  | { type: 'int'; value: number }
  | { type: 'none' }
  | { type: 'string'; value: string };

export function formatBytecode(code: CodeObject): string {
  return code.instructions
    .map((instruction) => {
      const argument =
        instruction.argument === undefined ? '' : ` ${instruction.argument}`;

      return `${instruction.opcode}${argument}`;
    })
    .join('\n');
}
