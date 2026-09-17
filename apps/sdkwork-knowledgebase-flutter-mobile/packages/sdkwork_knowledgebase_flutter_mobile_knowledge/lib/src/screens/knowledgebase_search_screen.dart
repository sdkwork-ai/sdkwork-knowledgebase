import 'package:flutter/material.dart';
import 'package:sdkwork_knowledgebase_flutter_mobile_commons/sdkwork_knowledgebase_flutter_mobile_commons.dart';

/// Knowledgebase search screen.
///
/// Route-level capability screen. The root shell mounts this; business UI lives
/// in capability packages.
class KnowledgebaseSearchScreen extends StatelessWidget {
  const KnowledgebaseSearchScreen({super.key});
  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Search'),
      ),
      body: const SdkworkKnowledgebaseStatusView(
        message: 'Knowledgebase search',
      ),
    );
  }
}
