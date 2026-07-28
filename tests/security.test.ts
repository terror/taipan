import { describe, expect, test } from 'bun:test';

describe('Tauri security configuration', () => {
  test('uses an explicit passive-content CSP', async () => {
    const config = await Bun.file('src-tauri/tauri.conf.json').json();
    const security = config.app.security;

    expect(security.capabilities).toEqual(['default']);
    expect(security.csp).toEqual({
      'base-uri': "'none'",
      'default-src': "'none'",
      'connect-src': "'self' ipc: http://ipc.localhost",
      'font-src': "'self'",
      'form-action': "'none'",
      'frame-src': "'none'",
      'img-src': "'self' data:",
      'object-src': "'none'",
      'script-src': "'self'",
      'style-src': "'self'",
    });
  });

  test('grants only the dialog permissions used by the application', async () => {
    const capability = await Bun.file(
      'src-tauri/capabilities/default.json'
    ).json();

    expect(capability.remote).toBeUndefined();
    expect(capability.windows).toEqual(['main']);
    expect(capability.permissions).toEqual([
      'dialog:allow-confirm',
      'dialog:allow-open',
    ]);
  });
});
