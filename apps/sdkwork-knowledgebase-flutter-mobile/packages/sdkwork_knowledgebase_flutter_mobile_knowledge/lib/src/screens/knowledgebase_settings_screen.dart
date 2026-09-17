import 'package:flutter/material.dart';
import 'package:sdkwork_knowledgebase_flutter_mobile_commons/sdkwork_knowledgebase_flutter_mobile_commons.dart';

/// Knowledgebase settings screen.
///
/// Route-level capability screen. The root shell mounts this; business UI lives
/// in capability packages.
class KnowledgebaseSettingsScreen extends StatelessWidget {
  const KnowledgebaseSettingsScreen({super.key});
  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Settings'),
      ),
      body: const SdkworkKnowledgebaseStatusView(
        message: 'Knowledgebase settings',
      ),
    );
  }
}
