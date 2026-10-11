#!/usr/bin/env node
// Builds request test vectors with verus-typescript-primitives, so the Rust
// parser is tested against the library's own bytes. Signatures are placeholders:
// these vectors test parsing and acceptance, not signature verification.
//
// Usage: node scripts/generate-request-vectors.mjs > src/lib/genericRequest/fixtures/request-vectors.json
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const p = require('verus-typescript-primitives');
const { BN } = require('bn.js');

const TESTNET = 'iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq';
const REQUESTER = 'iGHXn8mvh9kutBNotJ2LCNz3zDxvemEb2i';
const TARGET = 'i8KEYF3QeR1xPVVCL5BSpDXkA1CAfFDvmk';
const vectorId = (name) => p.nameAndParentAddrToIAddr(name, TESTNET);
const APP = vectorId('vector-app');
const REQUEST_ID = vectorId('vector-request');
const AUTH_ID = vectorId('vector-auth');
const UPDATE_ID = vectorId('vector-update');
const R_ADDRESS = 'RFWcHcpnFh57ovrV1hmzN8o8wmMZjyqCSh';
const SHIELDED = 'zs13akhfdhypx9a8t2efkfgxpd5d9h3glz7d5pug2ys65mrzt4mw5vjqx0vw29gx82y3e0xjjuhrwn';
const CREATED_AT = 1_760_000_000;
const PLACEHOLDER_SIGNATURE = Buffer.alloc(72, 7);

const compact = (address) => p.CompactIAddressObject.fromAddress(address, 'VRSCTEST');
const fqn = (name) =>
  new p.CompactIAddressObject({
    type: p.CompactIAddressObject.TYPE_FQN,
    address: name,
    rootSystemName: 'VRSCTEST',
  });

const MAINNET = 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV';

function signature(identity = compact(REQUESTER), system = TESTNET) {
  return new p.VerifiableSignatureData({
    systemID: compact(system),
    identityID: identity,
    signatureAsVch: PLACEHOLDER_SIGNATURE,
  });
}

function auth({ requestID = AUTH_ID, expiryTime = CREATED_AT + 600, constraints = [] } = {}) {
  return new p.AuthenticationRequestOrdinalVDXFObject({
    data: new p.AuthenticationRequestDetails({
      requestID: requestID ? compact(requestID) : undefined,
      expiryTime: expiryTime ? new BN(expiryTime) : undefined,
      recipientConstraints: constraints.map(
        ([type, identity]) => new p.RecipientConstraint({ type, identity: compact(identity) })
      ),
    }),
  });
}

function update() {
  const identity = p.PartialIdentity.fromJson({
    name: 'player7',
    parent: TESTNET,
    primaryaddresses: [R_ADDRESS],
    minimumsignatures: 1,
    revocationauthority: TARGET,
    recoveryauthority: TARGET,
    privateaddress: SHIELDED,
    contentmultimap: {
      'TestBob.antafri@::qa.note': [{ [p.DATA_TYPE_STRING.vdxfid]: 'Disposable QA note' }],
    },
  });
  return new p.IdentityUpdateRequestOrdinalVDXFObject({
    data: new p.IdentityUpdateRequestDetails({
      requestID: compact(UPDATE_ID),
      identity,
      expiryHeight: new BN(1_270_500),
      systemID: p.IdentityID.fromAddress(TESTNET),
    }),
  });
}

function provision() {
  return new p.ProvisionIdentityDetailsOrdinalVDXFObject({
    data: new p.ProvisionIdentityDetails({
      flags: new BN(0),
      uri: p.ResponseURI.fromUriString('https://lumen.example/provision', p.ResponseURI.TYPE_POST),
      systemID: compact(TESTNET),
      parentID: compact(REQUESTER),
      identityID: fqn('alex.TestBob.antafri@'),
    }),
  });
}

function invoice() {
  const details = new p.VerusPayInvoiceDetails({
    amount: new BN(1_000_000),
    destination: new p.TransferDestination({
      type: p.DEST_PKH,
      destinationBytes: p.fromBase58Check(R_ADDRESS).hash,
    }),
    requestedcurrencyid: TESTNET,
    expiryheight: new BN(1_270_600),
  });
  details.setFlags({ expires: true, isTestnet: true });
  return new p.VerusPayInvoiceDetailsOrdinalVDXFObject({ data: details });
}

function appEncryption() {
  return new p.AppEncryptionRequestOrdinalVDXFObject({
    data: new p.AppEncryptionRequestDetails({
      version: new BN(1),
      derivationNumber: new BN(1),
      requestID: compact(REQUEST_ID),
    }),
  });
}

function dataPacket() {
  const details = new p.DataPacketRequestDetails({
    flags: p.DataPacketRequestDetails.FLAG_FOR_USERS_SIGNATURE,
    signableObjects: [new p.DataDescriptor({ objectdata: Buffer.from('hello'), label: 'Message' })],
    statements: ['I agree'],
    requestID: compact(REQUEST_ID),
  });
  return new p.DataPacketRequestOrdinalVDXFObject({ data: details });
}

function envelope({
  details,
  signed = true,
  signer,
  requestID = REQUEST_ID,
  testnet = true,
  appOrDelegatedID,
  encryptTo,
  responseURIs = [
    p.ResponseURI.fromUriString('https://lumen.example/verus/response', p.ResponseURI.TYPE_POST),
    p.ResponseURI.fromUriString('https://lumen.example/verus/return', p.ResponseURI.TYPE_REDIRECT),
  ],
}) {
  const request = new p.GenericRequest({
    details,
    signature: signed ? signature(signer, testnet ? TESTNET : MAINNET) : undefined,
    requestID: requestID ? compact(requestID) : undefined,
    createdAt: new BN(CREATED_AT),
    appOrDelegatedID: appOrDelegatedID ? compact(appOrDelegatedID) : undefined,
    responseURIs,
    encryptResponseToAddress: encryptTo
      ? p.SaplingPaymentAddress.fromAddressString(encryptTo)
      : undefined,
  });
  if (testnet) request.setIsTestnet();
  request.setFlags();
  return request;
}

const cases = [];
function add(name, request, expect) {
  const hex = request.toBuffer().toString('hex');
  const roundTrip = new p.GenericRequest();
  roundTrip.fromBuffer(Buffer.from(hex, 'hex'), 0);
  cases.push({
    name,
    hex,
    requestHash: request.getRawDataSha256().toString('hex'),
    libraryAccepts: requireStructure(roundTrip),
    expect,
  });
}

// verusid-ts-client's structural rule, as both the harness and mobile apply it.
const { VerusIdInterface } = require('verusid-ts-client');
function requireStructure(request) {
  return VerusIdInterface.validateUnsignedGenericRequest(request);
}

add('auth', envelope({ details: [auth()] }), {
  ordinals: [2],
  signed: true,
  requestId: REQUEST_ID,
  signerIdentity: REQUESTER,
  responseUris: [
    [1, 'https://lumen.example/verus/response'],
    [2, 'https://lumen.example/verus/return'],
  ],
  authRequestId: AUTH_ID,
  authExpiryTime: CREATED_AT + 600,
});
add(
  'auth-constraints-no-ids',
  envelope({
    details: [
      auth({
        requestID: null,
        constraints: [
          [2, TESTNET],
          [3, REQUESTER],
        ],
      }),
    ],
    requestID: null,
  }),
  {
    ordinals: [2],
    authConstraints: [
      [2, TESTNET],
      [3, REQUESTER],
    ],
  }
);
add('auth-update', envelope({ details: [auth(), update()] }), {
  ordinals: [2, 4],
  update: {
    requestId: UPDATE_ID,
    target: TARGET,
    expiryHeight: 1_270_500,
    systemId: TESTNET,
    name: 'player7',
    parent: TESTNET,
    primaryAddresses: [R_ADDRESS],
    minimumSignatures: 1,
    revocation: TARGET,
    recovery: TARGET,
    privateAddress: SHIELDED,
    contentKeys: ['TestBob.antafri@::qa.note'],
  },
});
add('auth-provision', envelope({ details: [auth(), provision()] }), { ordinals: [2, 6] });
add('fqn-signer', envelope({ details: [auth()], signer: fqn('TestBob.antafri@') }), {
  signerIdentity: REQUESTER,
});
add('delegated-app', envelope({ details: [auth()], appOrDelegatedID: APP }), {
  appOrDelegatedId: APP,
});
add('encrypt-to', envelope({ details: [auth(), dataPacket()], encryptTo: SHIELDED }), {
  ordinals: [2, 9],
  encryptTo: SHIELDED,
});
add('invoice-unsigned', envelope({ details: [invoice()], signed: false, requestID: null }), {
  ordinals: [1],
  signed: false,
  invoice: {
    amount: 1_000_000,
    destination: R_ADDRESS,
    currency: TESTNET,
    expiryHeight: 1_270_600,
  },
});
add('auth-invoice', envelope({ details: [auth(), invoice()] }), { ordinals: [2, 1] });
add('auth-app-encryption', envelope({ details: [auth(), appEncryption()] }), { ordinals: [2, 7] });
add('mainnet-auth', envelope({ details: [auth()], testnet: false }), { testnet: false });

// Ordering cases from verusid-ts-client validateUnsignedGenericRequest.
add('update-before-auth', envelope({ details: [update(), auth()] }), { reject: 'order' });
add('two-auths', envelope({ details: [auth(), auth()] }), { reject: 'order' });
add('provision-without-auth', envelope({ details: [provision()] }), { reject: 'order' });
add('app-encryption-without-auth', envelope({ details: [appEncryption()] }), { reject: 'order' });
add('invoice-and-update', envelope({ details: [auth(), invoice(), update()] }), {
  reject: 'order',
});
add('invoice-not-last', envelope({ details: [auth(), invoice(), dataPacket()] }), {
  reject: 'order',
});
add('unsigned-auth', envelope({ details: [auth()], signed: false }), { reject: 'unsigned' });
add(
  'unsigned-invoice-with-app',
  envelope({ details: [invoice()], signed: false, appOrDelegatedID: APP }),
  {
    reject: 'unsigned',
  }
);

// Unsigned responses built the way Verus Mobile builds them
// (prepareGenericResponseForSigning, ensureGenericResponseSigner), for byte
// parity with the Rust builder. The signature slot stays empty.
const RESPONSE_CREATED_AT = CREATED_AT + 30;
const TXID = '11'.repeat(32);
function mobileResponse(hex, responseDetails, signer) {
  const request = new p.GenericRequest();
  request.fromBuffer(Buffer.from(hex, 'hex'), 0);
  const response = new p.GenericResponse({ details: responseDetails });
  response.signature = new p.VerifiableSignatureData({
    systemID: p.CompactAddressObject.fromIAddress(TESTNET),
    identityID: p.CompactAddressObject.fromIAddress(signer),
  });
  response.setSigned();
  if (request.isTestnet()) response.setIsTestnet();
  response.requestID = request.requestID;
  response.requestHash = request.getRawDataSha256();
  response.requestHashType = new BN(p.EHashTypes.HASH_SHA256);
  response.createdAt = new BN(RESPONSE_CREATED_AT);
  response.handledBy = p.VERUS_MOBILE_GENERIC_REQUEST_HANDLER_ID;
  response.setFlags();
  return response.toBuffer().toString('hex');
}
const authResponse = (requestID) =>
  new p.AuthenticationResponseOrdinalVDXFObject({
    data: new p.AuthenticationResponseDetails({ requestID: compact(requestID) }),
  });
const updateResponse = (requestID, txid) =>
  new p.IdentityUpdateResponseOrdinalVDXFObject({
    data: new p.IdentityUpdateResponseDetails({
      requestID: compact(requestID),
      txid: Buffer.from(txid, 'hex').reverse(),
    }),
  });
const byName = (name) => cases.find((entry) => entry.name === name);
const responses = [
  {
    request: 'auth',
    signer: TARGET,
    createdAt: RESPONSE_CREATED_AT,
    hex: mobileResponse(byName('auth').hex, [authResponse(REQUEST_ID)], TARGET),
  },
  {
    request: 'auth-update',
    signer: TARGET,
    createdAt: RESPONSE_CREATED_AT,
    txid: TXID,
    hex: mobileResponse(
      byName('auth-update').hex,
      [authResponse(REQUEST_ID), updateResponse(UPDATE_ID, TXID)],
      TARGET
    ),
  },
];

process.stdout.write(
  `${JSON.stringify({ generatedBy: 'scripts/generate-request-vectors.mjs', cases, responses }, null, 2)}\n`
);
