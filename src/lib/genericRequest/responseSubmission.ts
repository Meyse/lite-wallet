export type ResponseSubmissionToken = Readonly<{ sequence: number }>;

export class ResponseSubmissionLifetime {
  private sequence = 0;
  private disposed = false;

  begin(): ResponseSubmissionToken {
    this.sequence += 1;
    return { sequence: this.sequence };
  }

  cancel(): void {
    this.sequence += 1;
  }

  dispose(): void {
    this.disposed = true;
    this.cancel();
  }

  isActive(token: ResponseSubmissionToken): boolean {
    return !this.disposed && token.sequence === this.sequence;
  }
}

type RunResponseSubmissionOptions<TSigned> = {
  lifetime: ResponseSubmissionLifetime;
  sign: () => Promise<TSigned>;
  deliver?: (signed: TSigned) => Promise<void>;
  complete: (signed: TSigned) => void;
  fail: (error: unknown) => void;
  settle: () => void;
};

export async function runResponseSubmission<TSigned>({
  lifetime,
  sign,
  deliver,
  complete,
  fail,
  settle,
}: RunResponseSubmissionOptions<TSigned>): Promise<void> {
  const token = lifetime.begin();

  try {
    const signed = await sign();
    if (!lifetime.isActive(token)) return;

    if (deliver) {
      await deliver(signed);
      if (!lifetime.isActive(token)) return;
    }

    complete(signed);
  } catch (error) {
    if (lifetime.isActive(token)) {
      fail(error);
    }
  } finally {
    if (lifetime.isActive(token)) {
      settle();
    }
  }
}
