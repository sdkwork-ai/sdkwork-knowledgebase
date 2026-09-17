/**
 * Group knowledgebase launch handshake.
 *
 * `sdkwork-im` issues a short-lived, single-use, hash-stored opaque ticket
 * bound to actor/scope/version/epoch; this product verifies it and owns the
 * one-to-one group-space binding, content, and final ACL enforcement
 * (`../sdkwork-im/AGENTS.md` Group Knowledgebase Boundary).
 *
 * Nothing but the opaque ticket is ever accepted from an IM URL: no space
 * identifiers, destinations, session tokens, or caller context.
 */
export type GroupKnowledgebaseLaunchOutcome =
  | { status: "accepted"; spaceId: string }
  | { status: "expired" }
  | { status: "invalid" }
  | { status: "replayed" };

export interface GroupKnowledgebaseLaunchPort {
  consumeGroupLaunchTicket(ticket: string): Promise<GroupKnowledgebaseLaunchOutcome>;
}

export interface GroupKnowledgebaseLaunchService {
  launch(ticket: string): Promise<GroupKnowledgebaseLaunchOutcome>;
}

export function createGroupKnowledgebaseLaunchService(options: {
  port: GroupKnowledgebaseLaunchPort;
}): GroupKnowledgebaseLaunchService {
  return {
    launch(ticket) {
      const trimmed = ticket.trim();
      if (!trimmed) {
        return Promise.resolve({ status: "invalid" as const });
      }
      return options.port.consumeGroupLaunchTicket(trimmed);
    },
  };
}
