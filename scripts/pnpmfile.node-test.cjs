const assert = require('node:assert/strict');
const { test } = require('node:test');

const { hooks } = require('../.pnpmfile.cjs');

test('promotes each trusted Git edge to a peer dependency', () => {
  const pkg = {
    name: 'verusid-ts-client',
    version: '0.1.0',
    dependencies: {
      axios: '1.13.5',
      '@bitgo/utxo-lib':
        'git+https://github.com/VerusCoin/BitGoJS.git#9582a20f7211a7a6aed7bfae3c651e6b76c1f9bb',
      'verus-typescript-primitives':
        'git+https://github.com/VerusCoin/verus-typescript-primitives.git#7a7b01db697222cd68507a9dbf15f289615ea890',
      'verusd-rpc-ts-client':
        'git+https://github.com/VerusCoin/verusd-rpc-ts-client#58689ea52500a6a6e4aa741e5d2ed41d7bc6ddd7',
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

const reviewedManifests = [
  {
    name: 'verusid-ts-client',
    version: '0.1.0',
    dependencies: {
      '@bitgo/utxo-lib':
        'git+https://github.com/VerusCoin/BitGoJS.git#9582a20f7211a7a6aed7bfae3c651e6b76c1f9bb',
      'verus-typescript-primitives':
        'git+https://github.com/VerusCoin/verus-typescript-primitives.git#7a7b01db697222cd68507a9dbf15f289615ea890',
      'verusd-rpc-ts-client':
        'git+https://github.com/VerusCoin/verusd-rpc-ts-client#58689ea52500a6a6e4aa741e5d2ed41d7bc6ddd7',
    },
  },
  {
    name: '@bitgo/utxo-lib',
    version: '1.9.6',
    dependencies: {
      'bitcoin-ops': 'git+https://github.com/VerusCoin/bitcoin-ops',
      'verus-typescript-primitives':
        'git+https://github.com/VerusCoin/verus-typescript-primitives.git',
    },
  },
  {
    name: 'verus-typescript-primitives',
    version: '1.0.0',
    dependencies: {
      blake2b: 'https://github.com/VerusCoin/blake2b',
    },
  },
  {
    name: 'verusd-rpc-ts-client',
    version: '0.1.0',
    dependencies: {
      blake2b: 'https://github.com/VerusCoin/blake2b',
      'verus-typescript-primitives':
        'git+https://github.com/VerusCoin/verus-typescript-primitives.git#7a7b01db697222cd68507a9dbf15f289615ea890',
    },
  },
];
for (const manifest of reviewedManifests) {
  test(`accepts reviewed ${manifest.name} and rejects every altered edge`, () => {
    const result = hooks.readPackage(structuredClone(manifest));
    for (const [name, url] of Object.entries(manifest.dependencies)) {
      assert.equal(result.dependencies[name], undefined);
      assert.equal(result.peerDependencies[name], '*');
      for (const altered of [
        url + '#unreviewed',
        url.replace('github.com', 'example.invalid'),
        undefined,
      ]) {
        const changed = structuredClone(manifest);
        changed.dependencies[name] = altered;
        assert.throws(() => hooks.readPackage(changed), /Unexpected dependency/);
      }
    }
    assert.throws(
      () => hooks.readPackage({ ...structuredClone(manifest), version: '9.9.9' }),
      /Unexpected package version/
    );
  });
}
