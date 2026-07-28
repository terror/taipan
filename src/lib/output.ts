import type {
  DisplayDataOutput,
  ErrorOutput,
  ExecuteResultOutput,
  NotebookOutput,
  StreamOutput,
} from './types';

export const OUTPUT_TEXT_LIMIT = 10_000;

export type OutputRendererName =
  'error' | 'stream' | 'text/plain' | 'unsupported';

export interface OutputRenderer {
  name: OutputRendererName;
  accepts: (output: NotebookOutput) => boolean;
}

interface RenderedOutputBase {
  outputType: NotebookOutput['output_type'];
  truncated: boolean;
  omittedCharacters: number;
}

export interface RenderedStreamOutput extends RenderedOutputBase {
  renderer: 'stream';
  stream: string;
  text: string;
}

export interface RenderedErrorOutput extends RenderedOutputBase {
  renderer: 'error';
  name: string;
  value: string;
  traceback: string;
}

export interface RenderedTextOutput extends RenderedOutputBase {
  renderer: 'text/plain';
  text: string;
}

export interface RenderedUnsupportedOutput extends RenderedOutputBase {
  renderer: 'unsupported';
  mimeTypes: string;
}

export type RenderedOutput =
  | RenderedErrorOutput
  | RenderedStreamOutput
  | RenderedTextOutput
  | RenderedUnsupportedOutput;

const streamRenderer: OutputRenderer = {
  name: 'stream',
  accepts: (output) => output.output_type === 'stream',
};

const errorRenderer: OutputRenderer = {
  name: 'error',
  accepts: (output) => output.output_type === 'error',
};

const textRenderer: OutputRenderer = {
  name: 'text/plain',
  accepts: (output) =>
    (output.output_type === 'execute_result' ||
      output.output_type === 'display_data') &&
    isMultilineText(output.data['text/plain']),
};

const unsupportedRenderer: OutputRenderer = {
  name: 'unsupported',
  accepts: () => true,
};

export const outputRendererRegistry: readonly OutputRenderer[] = [
  streamRenderer,
  errorRenderer,
  textRenderer,
  unsupportedRenderer,
];

export function selectOutputRenderer(output: NotebookOutput): OutputRenderer {
  return outputRendererRegistry.find((renderer) => renderer.accepts(output))!;
}

export function renderOutputs(
  outputs: readonly NotebookOutput[],
  limit = OUTPUT_TEXT_LIMIT
): RenderedOutput[] {
  return outputs.map((output) => renderOutput(output, limit));
}

export function renderOutput(
  output: NotebookOutput,
  limit = OUTPUT_TEXT_LIMIT
): RenderedOutput {
  const renderer = selectOutputRenderer(output);

  switch (renderer.name) {
    case 'stream':
      return renderStream(output as StreamOutput, limit);
    case 'error':
      return renderError(output as ErrorOutput, limit);
    case 'text/plain':
      return renderText(
        output as DisplayDataOutput | ExecuteResultOutput,
        limit
      );
    case 'unsupported':
      return renderUnsupported(output, limit);
  }
}

function renderStream(
  output: StreamOutput,
  limit: number
): RenderedStreamOutput {
  const bounded = boundText(multilineText(output.text), limit);

  return {
    renderer: 'stream',
    outputType: output.output_type,
    stream: output.name,
    ...bounded,
  };
}

function renderError(output: ErrorOutput, limit: number): RenderedErrorOutput {
  const bounded = boundTextParts(
    [output.ename, output.evalue, output.traceback.join('\n')],
    limit
  );

  return {
    renderer: 'error',
    outputType: output.output_type,
    name: bounded.text[0],
    value: bounded.text[1],
    traceback: bounded.text[2],
    truncated: bounded.truncated,
    omittedCharacters: bounded.omittedCharacters,
  };
}

function renderText(
  output: DisplayDataOutput | ExecuteResultOutput,
  limit: number
): RenderedTextOutput {
  const value = output.data['text/plain'];
  const bounded = boundText(multilineText(value), limit);

  return {
    renderer: 'text/plain',
    outputType: output.output_type,
    ...bounded,
  };
}

function renderUnsupported(
  output: NotebookOutput,
  limit: number
): RenderedUnsupportedOutput {
  const mimeTypes =
    output.output_type === 'display_data' ||
    output.output_type === 'execute_result'
      ? Object.keys(output.data)
      : [];
  const bounded = boundText(mimeTypes.join(', '), limit);

  return {
    renderer: 'unsupported',
    outputType: output.output_type,
    mimeTypes: bounded.text,
    truncated: bounded.truncated,
    omittedCharacters: bounded.omittedCharacters,
  };
}

function isMultilineText(value: unknown): value is string | string[] {
  return (
    typeof value === 'string' ||
    (Array.isArray(value) && value.every((line) => typeof line === 'string'))
  );
}

function multilineText(value: unknown): string {
  if (!isMultilineText(value)) {
    throw new TypeError('Expected a Jupyter multiline string');
  }

  return Array.isArray(value) ? value.join('') : value;
}

function boundText(
  text: string,
  limit: number
): { text: string; truncated: boolean; omittedCharacters: number } {
  const boundedLimit = Math.max(0, Math.floor(limit));
  const rendered = text.slice(0, boundedLimit);

  return {
    text: rendered,
    truncated: rendered.length < text.length,
    omittedCharacters: text.length - rendered.length,
  };
}

function boundTextParts(
  parts: readonly string[],
  limit: number
): { text: string[]; truncated: boolean; omittedCharacters: number } {
  let remaining = Math.max(0, Math.floor(limit));
  let omittedCharacters = 0;

  const text = parts.map((part) => {
    const rendered = part.slice(0, remaining);
    remaining -= rendered.length;
    omittedCharacters += part.length - rendered.length;
    return rendered;
  });

  return {
    text,
    truncated: omittedCharacters > 0,
    omittedCharacters,
  };
}
