import { describe, expect, it } from 'vitest';
import { requestWarningKeys } from './warnings';

const warning = (warningType: string) => ({ warningType, message: 'English from Rust' });

describe('request warnings', () => {
  it('shows each meaning once, in the order Rust gives', () => {
    expect(
      requestWarningKeys([warning('dust_change'), warning('no_effect'), warning('empty_patch')])
    ).toEqual(['genericRequest.flow.warning.dustChange', 'genericRequest.flow.warning.noEffect']);
  });

  it('still shows a warning Rust adds later', () => {
    expect(requestWarningKeys([warning('future_code')])).toEqual([
      'genericRequest.flow.warning.other',
    ]);
  });
});
