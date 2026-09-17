import 'package:flutter/material.dart';
import 'package:sdkwork_knowledgebase_flutter_mobile_commons/sdkwork_knowledgebase_flutter_mobile_commons.dart';

/// Knowledgebase list screen.
///
/// Route-level capability screen. The root shell mounts this; business UI lives
/// in capability packages.
class KnowledgebaseListScreen extends StatelessWidget {
  const KnowledgebaseListScreen({super.key});
  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Knowledgebase'),
      ),
      body: const SdkworkKnowledgebaseStatusView(
        message: 'Knowledgebase list',
      ),
    );
  }
}
