import { describe, expect, test } from 'bun:test';
import { JSDOM } from 'jsdom';

import {
  MIME_PREFERENCE,
  type OutputRendererName,
  renderOutput,
  renderOutputs,
  selectOutputRenderer,
} from '../src/lib/output';
import type { NotebookOutput } from '../src/lib/types';

Object.assign(globalThis, { window: new JSDOM('').window });

const PNG =
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVQIHWP4z8DwHwAFgAI/ScL6WQAAAABJRU5ErkJggg==';
const JPEG = '/9j/4AAQSkZJRgABAQAAAQABAAD/2Q==';

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
    check(display({ 'image/png': PNG, 'text/plain': 'bar' }), 'image/png');
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
      'application/javascript': 'alert(1)',
      'image/png': 'baz',
    };
    const output = display(data);
    const original = structuredClone(output);

    expect(renderOutput(output)).toEqual({
      mimeTypes: 'application/javascript, image/png',
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

  test('uses a fixed MIME preference independent of bundle order', () => {
    const data = Object.fromEntries(
      [...MIME_PREFERENCE].reverse().map((mimeType) => {
        switch (mimeType) {
          case 'text/html':
            return [mimeType, '<strong>foo</strong>'];
          case 'image/svg+xml':
            return [mimeType, '<svg><circle r="1" /></svg>'];
          case 'image/png':
            return [mimeType, PNG];
          case 'image/jpeg':
            return [mimeType, JPEG];
          case 'text/markdown':
            return [mimeType, '**foo**'];
          case 'application/json':
            return [mimeType, { foo: 'bar' }];
          case 'text/plain':
            return [mimeType, 'foo'];
        }
      })
    );

    expect(MIME_PREFERENCE).toEqual([
      'text/html',
      'image/svg+xml',
      'image/png',
      'image/jpeg',
      'text/markdown',
      'application/json',
      'text/plain',
    ]);
    expect(selectOutputRenderer(display(data)).name).toBe('text/html');
  });

  test('renders each passive rich MIME type', () => {
    function check(
      mimeType: string,
      value: unknown,
      expected: Record<string, unknown>
    ) {
      expect(renderOutput(display({ [mimeType]: value }))).toMatchObject(
        expected
      );
    }

    check('image/png', PNG, {
      renderer: 'image/png',
      src: `data:image/png;base64,${PNG}`,
    });
    check('image/jpeg', JPEG, {
      renderer: 'image/jpeg',
      src: `data:image/jpeg;base64,${JPEG}`,
    });
    check(
      'application/json',
      { foo: ['bar'] },
      {
        renderer: 'application/json',
        text: '{\n  "foo": [\n    "bar"\n  ]\n}',
      }
    );
    check('text/markdown', ['**foo**\n', 'bar'], {
      renderer: 'text/markdown',
      html: '<p><strong>foo</strong>\nbar</p>\n',
    });
    check('text/html', '<p><strong>foo</strong></p>', {
      renderer: 'text/html',
      html: '<p><strong>foo</strong></p>',
    });
    check('image/svg+xml', '<svg><circle r="1"></circle></svg>', {
      renderer: 'image/svg+xml',
      html: '<svg><circle r="1"></circle></svg>',
    });
  });

  test('falls through malformed payloads and keeps JavaScript unsupported', () => {
    expect(
      renderOutput(
        display({
          'image/png': 'not base64',
          'image/jpeg': ['also invalid'],
          'image/svg+xml': { svg: true },
          'text/html': 42,
          'text/plain': 'foo',
        })
      )
    ).toMatchObject({ renderer: 'text/plain', text: 'foo' });

    const javascript = display({
      'application/javascript': 'globalThis.compromised = true',
      'text/javascript': 'globalThis.compromised = true',
    });

    expect(renderOutput(javascript)).toMatchObject({ renderer: 'unsupported' });
    expect(javascript).toEqual(
      display({
        'application/javascript': 'globalThis.compromised = true',
        'text/javascript': 'globalThis.compromised = true',
      })
    );
  });

  test('sanitizes active content, URLs, remote loads, and IPC paths', () => {
    const rendered = renderOutput(
      display({
        'text/html': `
          <script>globalThis.compromised = true</script>
          <style>@import url(https://example.com/foo.css)</style>
          <form action="http://ipc.localhost/open">
            <button formaction="ipc://localhost/open">foo</button>
          </form>
          <img src="https://example.com/foo.png" onerror="alert(1)">
          <a href="javascript:alert(1)" onclick="alert(1)">bar</a>
          <svg onload="alert(1)">
            <image href="https://example.com/foo.png"></image>
            <circle fill="url(https://example.com/foo.svg#paint)"></circle>
          </svg>
        `,
      })
    );

    if (rendered.renderer !== 'text/html') {
      throw new Error('Expected HTML output');
    }

    const document = new JSDOM('').window.document;
    document.body.innerHTML = rendered.html;

    expect(
      document.querySelector('script, style, form, button, image')
    ).toBeNull();
    expect(document.querySelector('img')?.hasAttribute('src')).toBe(false);
    expect(document.querySelector('a')?.hasAttribute('href')).toBe(false);
    expect(document.querySelector('[onerror], [onclick], [onload]')).toBeNull();
    expect(document.querySelector('circle')?.hasAttribute('fill')).toBe(false);
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
