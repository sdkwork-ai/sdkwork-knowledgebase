/// AuthGate integration for the Flutter mobile shell.
///
/// Route guards are shell/runtime responsibilities. Capability packages
/// declare auth mode and permission hints only.
class SdkworkKnowledgebaseAuthGateDecision {
  const SdkworkKnowledgebaseAuthGateDecision({required this.allowed, this.reason});

  final bool allowed;
  final String? reason;
}

SdkworkKnowledgebaseAuthGateDecision evaluateSdkworkKnowledgebaseAuthGate({
  required bool authRequired,
  required bool isAuthenticated,
}) {
  if (!authRequired || isAuthenticated) {
    return const SdkworkKnowledgebaseAuthGateDecision(allowed: true);
  }
  return const SdkworkKnowledgebaseAuthGateDecision(
    allowed: false,
    reason: 'authentication-required',
  );
}
