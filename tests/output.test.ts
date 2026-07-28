import { describe, expect, test } from 'bun:test';

import {
  type OutputRendererName,
  renderOutput,
  renderOutputs,
  selectOutputRenderer,
} from '../src/lib/output';
import type { NotebookOutput } from '../src/lib/types';

function stream(name: string, text: string | string[]): NotebookOutput {
  return { name, output_type: 'stream', text };
}

function display(data: Record<string, unknown>): NotebookOutput {
  return { data, metadata: {}, output_type: 'display_data' };
}

describe('output renderer registry', () => {
  test('selects renderers by output content', () => {
    function check(output: NotebookOutput, expected: OutputRendererName) {
      expect(selectOutputRenderer(output).name).toBe(expected);
    }

    check(stream('stdout', 'foo'), 'stream');
    check(
      {
        ename: 'Error',
        evalue: 'foo',
        output_type: 'error',
        traceback: [],
      },
      'error'
    );
    check(display({ 'image/png': 'foo', 'text/plain': 'bar' }), 'text/plain');
    check(display({ 'image/png': 'foo' }), 'unsupported');
    check(display({ 'text/plain': { foo: 'bar' } }), 'unsupported');
  });

  test('renders stream and MIME multiline arrays without adding characters', () => {
    expect(renderOutput(stream('stdout', ['foo\n', 'bar']))).toMatchObject({
      renderer: 'stream',
      stream: 'stdout',
      text: 'foo\nbar',
    });
    expect(
      renderOutput({
        data: { 'text/plain': ['baz\n', 'qux'] },
        execution_count: 1,
        metadata: {},
        output_type: 'execute_result',
      })
    ).toMatchObject({
      outputType: 'execute_result',
      renderer: 'text/plain',
      text: 'baz\nqux',
    });
    expect(
      renderOutput(display({ 'text/plain': ['foo', '', 'bar'] }))
    ).toMatchObject({
      outputType: 'display_data',
      renderer: 'text/plain',
      text: 'foobar',
    });
  });

  test('preserves stdout and stderr identities', () => {
    expect(renderOutput(stream('stdout', 'foo'))).toMatchObject({
      stream: 'stdout',
    });
    expect(renderOutput(stream('stderr', 'bar'))).toMatchObject({
      stream: 'stderr',
    });
  });

  test('renders error name, value, and traceback frames', () => {
    expect(
      renderOutput({
        ename: 'ValueError',
        evalue: 'foo',
        output_type: 'error',
        traceback: ['frame foo', 'frame bar'],
      })
    ).toEqual({
      name: 'ValueError',
      omittedCharacters: 0,
      outputType: 'error',
      renderer: 'error',
      traceback: 'frame foo\nframe bar',
      truncated: false,
      value: 'foo',
    });
  });

  test('uses an unchanged MIME bundle for unsupported output', () => {
    const data = {
      'application/json': { foo: ['bar'] },
      'image/png': 'baz',
    };
    const output = display(data);
    const original = structuredClone(output);

    expect(renderOutput(output)).toEqual({
      mimeTypes: 'application/json, image/png',
      omittedCharacters: 0,
      outputType: 'display_data',
      renderer: 'unsupported',
      truncated: false,
    });
    expect(output).toEqual(original);
    expect('data' in output && output.data).toBe(data);
  });

  test('retains persisted output order', () => {
    const outputs: NotebookOutput[] = [
      stream('stdout', 'foo'),
      display({ 'image/png': 'bar' }),
      stream('stderr', 'baz'),
      display({ 'text/plain': 'qux' }),
    ];

    expect(
      renderOutputs(outputs).map((output) =>
        output.renderer === 'stream' ? output.stream : output.renderer
      )
    ).toEqual(['stdout', 'unsupported', 'stderr', 'text/plain']);
  });

  test('bounds rendered text and reports truncation without changing data', () => {
    const output = stream('stdout', ['foo', 'bar']);

    expect(renderOutput(output, 4)).toEqual({
      omittedCharacters: 2,
      outputType: 'stream',
      renderer: 'stream',
      stream: 'stdout',
      text: 'foob',
      truncated: true,
    });
    expect(output).toEqual(stream('stdout', ['foo', 'bar']));
  });
});
