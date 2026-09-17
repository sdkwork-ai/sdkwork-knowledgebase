import '../models/knowledge_models.dart';

/// Group-knowledgebase launch orchestration.
///
/// Authority: the group-knowledgebase boundary declared in
/// `../sdkwork-im/AGENTS.md` (Group Knowledgebase Boundary) — `sdkwork-im`
/// issues an opaque, short-lived, single-use launch ticket; this application
/// owns the one-to-one group-space binding, content, and final ACL enforcement.
///
/// The ticket is the only value crossing the boundary. Space identifiers,
/// destinations, session tokens, and caller context are never accepted from IM
/// payloads, URLs, or deep links.
class GroupKnowledgebaseLaunchService {
  const GroupKnowledgebaseLaunchService();

  /// Validates the opaque ticket shape before it is exchanged for a session.
  ///
  /// The exchange itself requires the Knowledgebase RPC SDK over mTLS
  /// (`RPC_SPEC.md` section 13.2) and is therefore tracked as pending until
  /// that transport is generated for this runtime.
  bool isLaunchTicketShapeValid(String ticket) {
    final normalized = ticket.trim();
    if (normalized.isEmpty) {
      return false;
    }
    // Opaque, single-use, hash-stored tickets are URL-safe base64 without
    // padding. Anything containing separators or query syntax is rejected.
    return RegExp(r'^[A-Za-z0-9_-]+$').hasMatch(normalized);
  }

  GroupKnowledgebaseLaunchRequest buildRequest(String ticket) {
    final normalized = ticket.trim();
    if (!isLaunchTicketShapeValid(normalized)) {
      throw ArgumentError.value(ticket, 'ticket', 'must be an opaque launch ticket');
    }
    return GroupKnowledgebaseLaunchRequest(ticket: normalized);
  }
}
