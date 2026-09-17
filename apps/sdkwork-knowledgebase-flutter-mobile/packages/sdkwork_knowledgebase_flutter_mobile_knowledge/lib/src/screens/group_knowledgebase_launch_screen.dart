import 'package:flutter/material.dart';
import 'package:sdkwork_knowledgebase_flutter_mobile_commons/sdkwork_knowledgebase_flutter_mobile_commons.dart';

import '../copy/knowledge_messages.dart';

/// Group knowledgebase launch screen.
///
/// Route-level capability screen mounted by the root shell. The only input is
/// the opaque launch ticket; the screen never reads space identifiers,
/// destinations, session tokens, or caller context from the launch payload.
class GroupKnowledgebaseLaunchScreen extends StatelessWidget {
  const GroupKnowledgebaseLaunchScreen({super.key, this.ticket});

  final String? ticket;

  @override
  Widget build(BuildContext context) {
    final normalized = (ticket ?? '').trim();
    final message = normalized.isEmpty
        ? knowledgebaseMessagesEnUs['knowledgebase.launch.invalidTicket']!
        : knowledgebaseMessagesEnUs['knowledgebase.launch.title']!;
    return Scaffold(
      appBar: AppBar(
        title: const Text('Group knowledgebase'),
      ),
      body: SdkworkKnowledgebaseStatusView(message: message),
    );
  }
}
