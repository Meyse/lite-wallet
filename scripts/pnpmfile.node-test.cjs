const assert = require('node:assert/strict');
const { test } = require('node:test');

const { hooks } = require('../.pnpmfile.cjs');

test('promotes each trusted Git edge to a peer dependency', () => {
  const pkg = {
    name: 'verusid-ts-client',
    version: '0.1.0',
    dependencies: {
      axios: '1.13.5',
      '@bitgo/utxo-lib': 'git+https://github.com/VerusCoin/BitGoJS.git#utxo-lib-verus',
      'verus-typescript-primitives':
        'git+https://github.com/VerusCoin/verus-typescript-primitives.git',
      'verusd-rpc-ts-client': 'git+https://github.com/VerusCoin/verusd-rpc-ts-client',
    },
  };

  assert.deepEqual(hooks.readPackage(pkg), {
    name: 'verusid-ts-client',
    version: '0.1.0',
    dependencies: { axios: '1.13.5' },
    peerDependencies: {
      '@bitgo/utxo-lib': '*',
      'verus-typescript-primitives': '*',
      'verusd-rpc-ts-client': '*',
    },
  });
});

test('rejects a changed package version or Git dependency URL', () => {
  assert.throws(
    () =>
      hooks.readPackage({
        name: 'verusd-rpc-ts-client',
        version: '0.2.0',
        dependencies: {},
      }),
    /Unexpected package version/
  );

  assert.throws(
    () =>
      hooks.readPackage({
        name: 'verus-typescript-primitives',
        version: '1.0.0',
        dependencies: { blake2b: 'https://example.invalid/blake2b' },
      }),
    /Unexpected dependency/
  );
});

test('leaves unrelated packages unchanged', () => {
  const pkg = { name: 'example', version: '1.0.0', dependencies: { leftpad: '1.0.0' } };
  assert.strictEqual(hooks.readPackage(pkg), pkg);
});
