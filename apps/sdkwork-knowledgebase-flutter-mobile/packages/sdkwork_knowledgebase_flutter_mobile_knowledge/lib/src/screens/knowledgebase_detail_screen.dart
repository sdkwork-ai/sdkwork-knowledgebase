import 'package:flutter/material.dart';
import 'package:sdkwork_knowledgebase_flutter_mobile_commons/sdkwork_knowledgebase_flutter_mobile_commons.dart';

/// Knowledgebase document detail screen.
///
/// Route-level capability screen. The root shell mounts this; business UI lives
/// in capability packages.
class KnowledgebaseDetailScreen extends StatelessWidget {
  const KnowledgebaseDetailScreen({super.key});
  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Document'),
      ),
      body: const SdkworkKnowledgebaseStatusView(
        message: 'Document detail',
      ),
    );
  }
}
