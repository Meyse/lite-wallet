export type Dispose = () => void;

/**
 * Tracks cleanup registered by asynchronous setup work. A cleanup that arrives
 * after disposal runs immediately, so a destroyed route cannot leak listeners.
 */
export class DisposableScope {
  #active = true;
  #disposers: Dispose[] = [];

  get active(): boolean {
    return this.#active;
  }

  add(dispose: Dispose): boolean {
    if (!this.#active) {
      dispose();
      return false;
    }

    this.#disposers.push(dispose);
    return true;
  }

  dispose(): void {
    if (!this.#active) return;

    this.#active = false;
    for (const dispose of this.#disposers.splice(0).reverse()) {
      dispose();
    }
  }
}
