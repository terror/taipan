import {
  imageDataUrl,
  multilineText,
  renderMarkdown,
  sanitizeRichHtml,
  sanitizeSvg,
} from './rich-content';
import type {
  ErrorOutput,
  MimeBundle,
  NotebookOutput,
  StreamOutput,
} from './types';

export const OUTPUT_TEXT_LIMIT = 10_000;

const ANSI_SEQUENCE_PATTERN =
  /(?:\u001b\][^\u0007]*(?:\u0007|\u001b\\)|(?:\u001b\[|\u009b)[0-?]*[ -/]*[@-~])/g;

export const MIME_PREFERENCE = [
  'text/html',
  'image/svg+xml',
  'image/png',
  'image/jpeg',
  'text/markdown',
  'application/json',
  'text/plain',
] as const;

export type RichMimeType = (typeof MIME_PREFERENCE)[number];

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
  renderer: 'application/json' | 'text/plain';
  text: string;
}

export interface RenderedHtmlOutput extends RenderedOutputBase {
  renderer: 'image/svg+xml' | 'text/html' | 'text/markdown';
  html: string;
}

export interface RenderedImageOutput extends RenderedOutputBase {
  renderer: 'image/jpeg' | 'image/png';
  src: string;
}

export interface RenderedUnsupportedOutput extends RenderedOutputBase {
  renderer: 'unsupported';
  mimeTypes: string;
}

export type RenderedOutput =
  | RenderedErrorOutput
  | RenderedHtmlOutput
  | RenderedImageOutput
  | RenderedStreamOutput
  | RenderedTextOutput
  | RenderedUnsupportedOutput;

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
  if (output.output_type === 'stream') {
    return renderStream(output, limit);
  }

  if (output.output_type === 'error') {
    return renderError(output, limit);
  }

  const data = outputData(output);

  for (const mimeType of MIME_PREFERENCE) {
    const rendered = renderMime(mimeType, data?.[mimeType]);

    if (rendered === undefined) {
      continue;
    }

    const common = {
      outputType: output.output_type,
      truncated: false,
      omittedCharacters: 0,
    };

    if (mimeType === 'image/png' || mimeType === 'image/jpeg') {
      return { renderer: mimeType, src: rendered, ...common };
    }

    if (
      mimeType === 'text/html' ||
      mimeType === 'text/markdown' ||
      mimeType === 'image/svg+xml'
    ) {
      return { renderer: mimeType, html: rendered, ...common };
    }

    return {
      renderer: mimeType,
      outputType: output.output_type,
      ...boundText(rendered, limit),
    };
  }

  return renderUnsupported(output, limit);
}

function renderMime(
  mimeType: RichMimeType,
  value: unknown
): string | undefined {
  if (mimeType === 'image/png' || mimeType === 'image/jpeg') {
    return imageDataUrl(mimeType, value);
  }

  if (mimeType === 'application/json') {
    try {
      return value === undefined ? undefined : JSON.stringify(value, null, 2);
    } catch {
      return undefined;
    }
  }

  const text = multilineText(value);

  if (text === undefined) {
    return undefined;
  }

  switch (mimeType) {
    case 'text/html':
      return sanitizeRichHtml(text);
    case 'image/svg+xml':
      return sanitizeSvg(text);
    case 'text/markdown':
      return renderMarkdown(text);
    case 'text/plain':
      return stripAnsi(text);
  }
}

function renderStream(
  output: StreamOutput,
  limit: number
): RenderedStreamOutput {
  const bounded = boundText(stripAnsi(multilineText(output.text)!), limit);

  return {
    renderer: 'stream',
    outputType: output.output_type,
    stream: output.name,
    ...bounded,
  };
}

function renderError(output: ErrorOutput, limit: number): RenderedErrorOutput {
  const bounded = boundTextParts(
    [output.ename, output.evalue, output.traceback.join('\n')].map(stripAnsi),
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

function stripAnsi(text: string): string {
  return text.replace(ANSI_SEQUENCE_PATTERN, '');
}

function renderUnsupported(
  output: NotebookOutput,
  limit: number
): RenderedUnsupportedOutput {
  const data = outputData(output);
  const bounded = boundText(data ? Object.keys(data).join(', ') : '', limit);

  return {
    renderer: 'unsupported',
    outputType: output.output_type,
    mimeTypes: bounded.text,
    truncated: bounded.truncated,
    omittedCharacters: bounded.omittedCharacters,
  };
}

function outputData(output: NotebookOutput): MimeBundle | undefined {
  return output.output_type === 'display_data' ||
    output.output_type === 'execute_result'
    ? output.data
    : undefined;
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
