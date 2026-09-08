const trustedGitPackageEdges = {
  '@bitgo/utxo-lib': {
    version: '1.9.6',
    dependencies: {
      'bitcoin-ops': 'git+https://github.com/VerusCoin/bitcoin-ops',
      'verus-typescript-primitives':
        'git+https://github.com/VerusCoin/verus-typescript-primitives.git',
    },
  },
  'verus-typescript-primitives': {
    version: '1.0.0',
    dependencies: {
      blake2b: 'https://github.com/VerusCoin/blake2b',
    },
  },
  'verusd-rpc-ts-client': {
    version: '0.1.0',
    dependencies: {
      blake2b: 'https://github.com/VerusCoin/blake2b',
      'verus-typescript-primitives':
        'git+https://github.com/VerusCoin/verus-typescript-primitives.git',
    },
  },
  'verusid-ts-client': {
    version: '0.1.0',
    dependencies: {
      '@bitgo/utxo-lib': 'git+https://github.com/VerusCoin/BitGoJS.git#utxo-lib-verus',
      'verus-typescript-primitives':
        'git+https://github.com/VerusCoin/verus-typescript-primitives.git',
      'verusd-rpc-ts-client': 'git+https://github.com/VerusCoin/verusd-rpc-ts-client',
    },
  },
};

function readPackage(pkg) {
  const trustedPackage = trustedGitPackageEdges[pkg.name];
  if (!trustedPackage) return pkg;

  if (pkg.version !== trustedPackage.version) {
    throw new Error(
      `Unexpected package version for ${pkg.name}: expected ${trustedPackage.version}, got ${pkg.version}`
    );
  }

  for (const [dependencyName, expectedUrl] of Object.entries(trustedPackage.dependencies)) {
    const actualUrl = pkg.dependencies?.[dependencyName];
    if (actualUrl !== expectedUrl) {
      throw new Error(
        `Unexpected dependency ${pkg.name} -> ${dependencyName}: expected ${expectedUrl}, got ${actualUrl}`
      );
    }

    delete pkg.dependencies[dependencyName];
    pkg.peerDependencies = { ...pkg.peerDependencies, [dependencyName]: '*' };
  }

  return pkg;
}

// pnpm blocks transitive Git dependencies. These exact reviewed packages and
// dependency URLs are promoted to peers supplied by the root's commit pins.
module.exports = { hooks: { readPackage } };
