import 'package:flutter/material.dart';

/// Domain-neutral screen/list state primitives.
///
/// Capability packages map their own data onto these payload-free primitives.
/// Authority: `APP_FLUTTER_UI_SPEC.md`.
enum SdkworkKnowledgebaseScreenStatus { loading, ready, empty, error }

SdkworkKnowledgebaseScreenStatus resolveSdkworkKnowledgebaseScreenStatus(
  int itemCount,
  bool loading,
  String? errorMessage,
) {
  if (loading) {
    return SdkworkKnowledgebaseScreenStatus.loading;
  }
  if (errorMessage != null && errorMessage.isNotEmpty) {
    return SdkworkKnowledgebaseScreenStatus.error;
  }
  return itemCount == 0
      ? SdkworkKnowledgebaseScreenStatus.empty
      : SdkworkKnowledgebaseScreenStatus.ready;
}

/// Payload-free status widget used by capability screens.
class SdkworkKnowledgebaseStatusView extends StatelessWidget {
  const SdkworkKnowledgebaseStatusView({super.key, required this.message});

  final String message;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Text(message, textAlign: TextAlign.center),
      ),
    );
  }
}
