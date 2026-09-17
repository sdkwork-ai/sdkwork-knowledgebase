import { describe, expect, it } from '@ohos/hypium';

/**
 * HarmonyOS module smoke test.
 *
 * Runs under `hvigor test` with the HarmonyOS SDK available.
 */
export default function abilityTest() {
  describe('EntryAbilityTest', () => {
    it('module scaffold is loadable', 0, () => {
      expect(true).assertTrue();
    });
  });
}
