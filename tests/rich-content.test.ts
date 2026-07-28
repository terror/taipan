import { describe, expect, test } from 'bun:test';
import { JSDOM } from 'jsdom';

import { renderMarkdown, sanitizeSvg } from '../src/lib/rich-content';

Object.assign(globalThis, { window: new JSDOM('').window });

const PNG =
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVQIHWP4z8DwHwAFgAI/ScL6WQAAAABJRU5ErkJggg==';

describe('rich content', () => {
  test('renders local Markdown attachments including encoded names', () => {
    const html = renderMarkdown('![foo](attachment:foo%20bar.png)', {
      'foo bar.png': { 'image/png': [PNG.slice(0, 40), PNG.slice(40)] },
    });

    expect(html).toContain(`src="data:image/png;base64,${PNG}"`);
    expect(html).toContain('alt="foo"');
    expect(html).not.toContain('attachment:');
  });

  test('uses a sanitized SVG attachment before creating its local URL', () => {
    const html = renderMarkdown('![foo](attachment:foo.svg)', {
      'foo.svg': {
        'image/svg+xml': `
          <svg onload="alert(1)">
            <script>alert(1)</script>
            <image href="https://example.com/foo.png"></image>
            <circle r="2"></circle>
          </svg>
        `,
      },
    });
    const match = html.match(
      /src="data:image\/svg\+xml;charset=utf-8,([^"]+)"/
    );

    expect(match).not.toBeNull();

    const svg = decodeURIComponent(match![1]);

    expect(svg).toContain('<svg');
    expect(svg).toContain('<circle r="2"></circle>');
    expect(svg).not.toContain('<script');
    expect(svg).not.toContain('<image');
    expect(svg).not.toContain('onload');
    expect(svg).not.toContain('https://');
  });

  test('removes Markdown URLs, remote images, raw HTML, and IPC paths', () => {
    const html = renderMarkdown(
      '[remote](https://example.com)\n' +
        '[ipc](http://ipc.localhost/open)\n' +
        '[script](javascript:alert(1))\n' +
        '![remote image](https://example.com/foo.png)\n' +
        '<img src="ipc://localhost/open" onerror="alert(1)">\n' +
        '<iframe src="https://example.com"></iframe>'
    );

    expect(html).toContain('remote');
    expect(html).toContain('ipc');
    expect(html).not.toContain('href=');
    expect(html).not.toContain('src=');
    expect(html).not.toContain('<iframe');
    expect(html).not.toContain('onerror');
    expect(html).not.toContain('https://');
    expect(html).not.toContain('ipc://');
    expect(html).not.toContain('javascript:');
  });

  test('rejects malformed SVG and strips active SVG features', () => {
    expect(sanitizeSvg('<p>foo</p>')).toBeUndefined();

    const svg = sanitizeSvg(`
      <svg>
        <foreignObject><script>alert(1)</script></foreignObject>
        <use href="https://example.com/foo.svg#bar"></use>
        <animate attributeName="x" values="0;1"></animate>
        <path d="M0 0" style="filter:url(https://example.com/foo.svg)"></path>
      </svg>
    `)!;

    expect(svg).toContain('<svg>');
    expect(svg).toContain('<path d="M0 0"></path>');
    expect(svg).not.toContain('foreignObject');
    expect(svg).not.toContain('<use');
    expect(svg).not.toContain('<animate');
    expect(svg).not.toContain('style=');
    expect(svg).not.toContain('https://');
  });
});
