export type PreflightRequestToken = {
  generation: number;
  signature: string;
};

/**
 * Rejects responses from an invalidated or superseded preflight request.
 * The backend remains authoritative; this only prevents stale UI state from
 * reviving a preflight after the route, amount, recipient, or fee mode changes.
 */
export class PreflightRequestGuard {
  #generation = 0;
  #disposed = false;

  begin(signature: string): PreflightRequestToken {
    this.#generation += 1;
    return { generation: this.#generation, signature };
  }

  invalidate(): void {
    this.#generation += 1;
  }

  dispose(): void {
    this.#disposed = true;
    this.invalidate();
  }

  isCurrent(token: PreflightRequestToken, currentSignature: string): boolean {
    return (
      !this.#disposed &&
      token.generation === this.#generation &&
      token.signature === currentSignature
    );
  }
}
