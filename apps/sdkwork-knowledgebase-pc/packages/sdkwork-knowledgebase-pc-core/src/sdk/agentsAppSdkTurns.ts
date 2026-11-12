/**
 * Agents app SDK turn helpers re-exported for capability packages.
 * Feature packages import agents streaming through this core export instead of
 * importing the generated SDK module directly (APP_SDK_INTEGRATION_SPEC §9).
 */
export {
  completeAgentTurn,
  completeAgentTurnStream,
  turnStreamErrorFromEvent,
} from '@sdkwork/agents-app-sdk';
export type {
  CompleteAgentTurnResult,
  CreateAgentTurnRequest,
  TurnRichToolEvent,
  TurnStreamHandlers,
} from '@sdkwork/agents-app-sdk';
