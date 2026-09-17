import 'package:sdkwork_knowledgebase_flutter_mobile_core/sdkwork_knowledgebase_flutter_mobile_core.dart';

/// Host adapter registration.
///
/// Capability packages depend on the typed interfaces declared by core and
/// never call platform APIs directly.
void registerHostAdapters() {
  registerKnowledgebaseHostCapabilities(listKnowledgebaseHostCapabilities());
}
