// Test-only stand-in for SvelteKit's $app/navigation in the mounted Vitest config.
export const navigation = {
  calls: [] as Array<{ to: string; options?: unknown }>,
  failNextGoto: false,
};

export async function goto(to: string, options?: unknown): Promise<void> {
  navigation.calls.push({ to, options });
  if (navigation.failNextGoto) {
    navigation.failNextGoto = false;
    throw new Error('navigation failed');
  }
}
