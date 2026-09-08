import { Buffer } from 'buffer';

const globalScope = globalThis as typeof globalThis & { global?: typeof globalThis };

if (!('Buffer' in globalScope)) {
  Reflect.set(globalThis, 'Buffer', Buffer);
}

if (!globalScope.global) {
  globalScope.global = globalThis;
}

export {};
