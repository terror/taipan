import createDOMPurify, { type Config } from 'dompurify';
import { Marked } from 'marked';

import type { Attachments, MimeBundle } from './types';

const ATTACHMENT_PREFIX = '#taipan-attachment-';

const SANITIZE_CONFIG: Config = {
  ALLOWED_ATTR: [
    'alt',
    'aria-hidden',
    'aria-label',
    'checked',
    'colspan',
    'cx',
    'cy',
    'd',
    'disabled',
    'fill',
    'height',
    'href',
    'opacity',
    'points',
    'preserveAspectRatio',
    'r',
    'reversed',
    'role',
    'rowspan',
    'rx',
    'ry',
    'src',
    'start',
    'stroke',
    'stroke-linecap',
    'stroke-linejoin',
    'stroke-width',
    'title',
    'transform',
    'type',
    'viewBox',
    'width',
    'x',
    'x1',
    'x2',
    'y',
    'y1',
    'y2',
    'xmlns',
  ],
  ALLOWED_TAGS: [
    'a',
    'b',
    'blockquote',
    'br',
    'caption',
    'circle',
    'code',
    'col',
    'colgroup',
    'dd',
    'del',
    'details',
    'div',
    'dl',
    'dt',
    'ellipse',
    'em',
    'figcaption',
    'figure',
    'g',
    'h1',
    'h2',
    'h3',
    'h4',
    'h5',
    'h6',
    'hr',
    'i',
    'img',
    'kbd',
    'li',
    'line',
    'mark',
    'ol',
    'p',
    'path',
    'polygon',
    'polyline',
    'pre',
    'q',
    'rect',
    's',
    'samp',
    'small',
    'span',
    'strong',
    'sub',
    'summary',
    'sup',
    'svg',
    'table',
    'tbody',
    'td',
    'tfoot',
    'th',
    'thead',
    'tr',
    'u',
    'ul',
    'var',
  ],
  ALLOW_DATA_ATTR: false,
  SANITIZE_NAMED_PROPS: true,
};

let purifier: ReturnType<typeof createDOMPurify> | undefined;
let attachmentPlaceholdersAllowed = false;

export function sanitizeRichHtml(
  value: string,
  allowAttachmentPlaceholders = false
): string {
  const sanitizer = getPurifier();
  attachmentPlaceholdersAllowed = allowAttachmentPlaceholders;

  try {
    return sanitizer.sanitize(value, SANITIZE_CONFIG);
  } finally {
    attachmentPlaceholdersAllowed = false;
  }
}

export function sanitizeSvg(value: string): string | undefined {
  const html = sanitizeRichHtml(value);
  return /<svg(?:\s|>)/i.test(html) ? html : undefined;
}

export function renderMarkdown(
  source: string,
  attachments: Attachments = {}
): string {
  const sources: string[] = [];
  const markdown = new Marked({
    gfm: true,
    walkTokens: (token) => {
      if (token.type !== 'image') {
        return;
      }

      const source = attachmentSource(token.href, attachments);

      if (source) {
        token.href = `${ATTACHMENT_PREFIX}${sources.push(source) - 1}`;
      }
    },
  });
  const parsed = markdown.parse(source, { async: false });
  const html = sanitizeRichHtml(parsed, true);

  return sources.reduce(
    (rendered, source, index) =>
      rendered.replaceAll(
        `src="${ATTACHMENT_PREFIX}${index}"`,
        `src="${source}"`
      ),
    html
  );
}

export function imageDataUrl(
  mimeType: 'image/jpeg' | 'image/png',
  value: unknown
): string | undefined {
  const text = multilineText(value);

  if (text === undefined) {
    return undefined;
  }

  const base64 = text.replaceAll(/\s/g, '');
  const validBase64 =
    /^(?:[a-z\d+/]{4})*(?:[a-z\d+/]{2}==|[a-z\d+/]{3}=)?$/i.test(base64);
  const validSignature =
    mimeType === 'image/png'
      ? base64.startsWith('iVBORw0KGgo')
      : base64.startsWith('/9j/');

  return validBase64 && validSignature
    ? `data:${mimeType};base64,${base64}`
    : undefined;
}

export function multilineText(value: unknown): string | undefined {
  return typeof value === 'string'
    ? value
    : Array.isArray(value) && value.every((line) => typeof line === 'string')
      ? value.join('')
      : undefined;
}

function getPurifier(): ReturnType<typeof createDOMPurify> {
  if (!purifier) {
    purifier = createDOMPurify(window);
    purifier.addHook('uponSanitizeAttribute', (_node, data) => {
      const name = data.attrName.toLowerCase();
      const value = data.attrValue.trim();

      if (
        name.startsWith('on') ||
        name === 'style' ||
        /\burl\s*\(/i.test(value) ||
        ([
          'action',
          'formaction',
          'href',
          'src',
          'srcset',
          'xlink:href',
        ].includes(name) &&
          !(
            attachmentPlaceholdersAllowed &&
            /^#taipan-attachment-\d+$/.test(value)
          ))
      ) {
        data.keepAttr = false;
      }
    });
  }

  return purifier;
}

function attachmentSource(
  href: string,
  attachments: Attachments
): string | undefined {
  if (!href.startsWith('attachment:')) {
    return undefined;
  }

  const name = decodeAttachmentName(href.slice('attachment:'.length));
  const bundle = name === undefined ? undefined : attachments[name];

  return bundle && attachmentImage(bundle);
}

function attachmentImage(bundle: MimeBundle): string | undefined {
  const svgText = multilineText(bundle['image/svg+xml']);
  const svg = svgText === undefined ? undefined : sanitizeSvg(svgText);

  if (svg !== undefined) {
    return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
  }

  return (
    imageDataUrl('image/png', bundle['image/png']) ??
    imageDataUrl('image/jpeg', bundle['image/jpeg'])
  );
}

function decodeAttachmentName(name: string): string | undefined {
  try {
    return decodeURIComponent(name);
  } catch {
    return undefined;
  }
}
