import { createRequire } from 'node:module';

// @ts-expect-error The pinned Verus fork does not publish TypeScript declarations.
import utxo from '@bitgo/utxo-lib';
import {
  LoginConsentProvisioningChallenge,
  LoginConsentProvisioningRequest,
} from 'verus-typescript-primitives';
import { VerusdRpcInterface } from 'verusd-rpc-ts-client';
import { describe, expect, it } from 'vitest';

const require = createRequire(import.meta.url);

/**
 * @param {string} parentEntry
 * @param {string} dependencyName
 */
function dependencyVersion(parentEntry, dependencyName) {
  const packagePath = require.resolve(`${dependencyName}/package.json`, {
    paths: [parentEntry],
  });
  return require(packagePath).version;
}

describe('pinned Verus dependency vectors', () => {
  it('matches the provisioning challenge bytes and signature hash parsed by Rust', () => {
    const challenge = new LoginConsentProvisioningChallenge({
      challenge_id: 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV',
      created_at: 1_700_000_000,
      name: 'alice',
      system_id: 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV',
      parent: 'iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq',
    });
    const request = new LoginConsentProvisioningRequest({
      signing_address: 'RNw8Gf9cVAKx1Lt2KnUyqMoXoexM4XkfVr',
      challenge,
    });

    expect(challenge.toBuffer().toString('hex')).toBe(
      'bf5b8b3997fe0381a6383fddc8f055c1f0dd67740165141af5b8015c64d39ab44c60ead8317f9f5a9b6c4c00f153650000000000567b9a87b17131f6eeb2dd0bdd191ece4a5d603b01010005616c696365141af5b8015c64d39ab44c60ead8317f9f5a9b6c4c14a6ef9ea235635e328124ff3429db9f9e91b64e2d'
    );
    expect(request.getChallengeHash().toString('hex')).toBe(
      '35d3bef3d19528f98c4edeebba920d50438ca346d38257aa1c91b53805bd5c31'
    );
  });

  it('round-trips a deterministic Verus Sapling transaction', () => {
    const tx = new utxo.Transaction(utxo.networks.verus);
    tx.version = 4;
    tx.overwintered = 1;
    tx.versionGroupId = 0x892f2085;
    tx.expiryHeight = 123_456;

    const outputHash = utxo.address.fromBase58Check('RAutMoGh771ECTDbTq2qwwZo7MF5Tov3ka').hash;
    outputHash.fill(0x11);
    const outputScript = utxo.script.compile([
      utxo.opcodes.OP_DUP,
      utxo.opcodes.OP_HASH160,
      outputHash,
      utxo.opcodes.OP_EQUALVERIFY,
      utxo.opcodes.OP_CHECKSIG,
    ]);
    const inputHash = tx.getHash();
    inputHash.fill(0x22);
    tx.addInput(inputHash, 1, 0xfffffffe);
    tx.addOutput(outputScript, 123_456_789);

    const serialized = tx.toBuffer();
    expect(serialized.toString('hex')).toBe(
      '0400008085202f890122222222222222222222222222222222222222222222222222222222222222220100000000feffffff0115cd5b07000000001976a914111111111111111111111111111111111111111188ac0000000040e201000000000000000000000000'
    );
    expect(tx.getId()).toBe('c7ed21dd1384fae820769cbe27bd29ffaa57ed74a6194caa8b51b47024b4f622');

    const parsed = utxo.Transaction.fromBuffer(serialized, utxo.networks.verus);
    expect(parsed.toBuffer().equals(serialized)).toBe(true);
  });

  it('round-trips a known Verus WIF and rejects a Unicode lookalike', () => {
    const bitgoEntry = require.resolve('@bitgo/utxo-lib');
    const wifEntry = require.resolve('wif', { paths: [bitgoEntry] });
    const bs58checkEntry = require.resolve('bs58check', { paths: [wifEntry] });
    const bs58Entry = require.resolve('bs58', { paths: [bs58checkEntry] });
    expect(dependencyVersion(bs58Entry, 'base-x')).toBe('3.0.11');

    const knownWif = 'Up3VgAKQio8guDjySfZTAnh8RbBZmdLt42AbuvVMB7SRabip7y9r';
    const keyPair = utxo.ECPair.fromWIF(knownWif, utxo.networks.verus);
    expect(keyPair.toWIF()).toBe(knownWif);
    expect(keyPair.getAddress()).toBe('RLNcgZpJgK6Uh3zXgkm2z7As5nJJVt6HXr');

    const lookalikeWif = `Ｕ${knownWif.slice(1)}`;
    expect(() => utxo.ECPair.fromWIF(lookalikeWif, utxo.networks.verus)).toThrow(
      'Non-base58 character'
    );
  });

  it('uses patched Axios through the Verus RPC client adapter contract', async () => {
    const rpcClientEntry = require.resolve('verusd-rpc-ts-client');
    const idClientEntry = require.resolve('verusid-ts-client');
    expect(dependencyVersion(rpcClientEntry, 'axios')).toBe('1.20.0');
    expect(dependencyVersion(idClientEntry, 'axios')).toBe('1.20.0');

    /** @type {{ baseURL?: string; method?: string; url?: string; data?: string } | undefined} */
    let adapterRequest;
    const rpcClient = new VerusdRpcInterface('VRSC', 'https://rpc.invalid', {
      adapter: async (config) => {
        adapterRequest = config;
        const request = JSON.parse(config.data);
        return {
          config,
          data: { id: request.id, result: { longestchain: 321 }, error: null },
          headers: {},
          status: 200,
          statusText: 'OK',
        };
      },
    });

    await expect(rpcClient.getInfo()).resolves.toEqual({
      id: 0,
      result: { longestchain: 321 },
      error: null,
    });
    expect(adapterRequest).toMatchObject({
      baseURL: 'https://rpc.invalid',
      method: 'post',
      url: '/',
    });
    if (!adapterRequest?.data) throw new Error('Axios adapter did not receive request data');
    expect(JSON.parse(adapterRequest.data)).toEqual({
      jsonrpc: '1.0',
      id: 0,
      params: [],
      method: 'getinfo',
    });
  });
});
