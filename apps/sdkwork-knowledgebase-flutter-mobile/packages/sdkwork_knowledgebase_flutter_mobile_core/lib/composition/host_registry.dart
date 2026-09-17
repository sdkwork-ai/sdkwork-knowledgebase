/// Host adapter capability registry.
///
/// Renderer/feature code routes through this declared capability set; direct
/// platform branches are forbidden.
const List<String> sdkworkKnowledgebaseHostCapabilities = <String>[
  'secureStorage',
  'networkStatus',
  'appLifecycle',
  'deepLinks',
  'filePicker',
  'camera',
  'qrScanner',
  'pushNotifications',
  'clipboard',
  'deviceInfo',
];

List<String> listKnowledgebaseHostCapabilities() =>
    sdkworkKnowledgebaseHostCapabilities;

bool hasKnowledgebaseHostCapability(String capability) =>
    sdkworkKnowledgebaseHostCapabilities.contains(capability);

List<String> _registeredHostCapabilities = <String>[];

void registerKnowledgebaseHostCapabilities(List<String> capabilities) {
  _registeredHostCapabilities = List<String>.unmodifiable(capabilities);
}

List<String> registeredKnowledgebaseHostCapabilities() =>
    List<String>.unmodifiable(_registeredHostCapabilities);
