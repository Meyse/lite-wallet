export class EntryContextGuard {
  #appliedKey: string | null = null;

  claim(key: string, ready: boolean): boolean {
    if (!ready || this.#appliedKey === key) return false;
    this.#appliedKey = key;
    return true;
  }
}
