/// Knowledgebase app-api SDK port and factory contract.
///
/// Authority: `APP_SDK_INTEGRATION_SPEC.md` and
/// `FLUTTER_APP_MOBILE_ARCHITECTURE_SPEC.md` section 4.
///
/// The Flutter root consumes `/app/v3/api` through the generated Dart app SDK
/// client. This file owns the injected port contract, the base-URL
/// normalization, and the credential resolution boundary. Concrete transport
/// construction stays in the root bootstrap.
///
/// PREREQUISITE: the SDK generation chain currently emits the TypeScript target
/// of `sdkwork-knowledgebase-app-sdk` only. No Dart target exists yet, so this
/// module deliberately declares the port and the adapter seam instead of
/// vendoring a transport copy. Capability packages must never fill this gap
/// with raw request APIs or manual auth headers.
const String knowledgebaseAppApiPrefix = '/app/v3/api';

/// Injected transport ports. The generated Dart clients implement them once the
/// Dart targets land; tests inject fakes.
abstract class KnowledgebaseAppSdkPort {
  String get baseUrl;
}

abstract class DriveAppSdkPort {
  String get baseUrl;
}

class KnowledgebaseAppSdkClientConfig {
  const KnowledgebaseAppSdkClientConfig({
    required this.baseUrl,
    this.accessToken,
    this.authToken,
    this.platform = 'flutter-mobile',
  });

  final String baseUrl;
  final String? accessToken;
  final String? authToken;
  final String platform;
}

class KnowledgebaseAppSdkClients {
  const KnowledgebaseAppSdkClients({
    required this.appApiBaseUrl,
    required this.knowledge,
    required this.drive,
  });

  final String appApiBaseUrl;
  final KnowledgebaseAppSdkPort knowledge;
  final DriveAppSdkPort drive;
}

/// Default port implementations used until the generated Dart clients land.
class KnowledgebaseAppSdkPortPlaceholder implements KnowledgebaseAppSdkPort {
  const KnowledgebaseAppSdkPortPlaceholder({required this.baseUrl});

  @override
  final String baseUrl;
}

class DriveAppSdkPortPlaceholder implements DriveAppSdkPort {
  const DriveAppSdkPortPlaceholder({required this.baseUrl});

  @override
  final String baseUrl;
}

KnowledgebaseAppSdkClients createKnowledgebaseAppSdkClients({
  required String appApiBaseUrl,
  String? authToken,
  String? accessToken,
}) {
  final normalizedSurfaceUrl = normalizeKnowledgebaseAppApiBaseUrl(appApiBaseUrl);
  final config = KnowledgebaseAppSdkClientConfig(
    baseUrl: resolveKnowledgebaseTransportBaseUrl(normalizedSurfaceUrl),
    accessToken: accessToken,
    authToken: authToken,
  );
  return KnowledgebaseAppSdkClients(
    appApiBaseUrl: normalizedSurfaceUrl,
    knowledge: KnowledgebaseAppSdkPortPlaceholder(baseUrl: config.baseUrl),
    drive: DriveAppSdkPortPlaceholder(baseUrl: config.baseUrl),
  );
}

String normalizeKnowledgebaseAppApiBaseUrl(String value) {
  final normalized = value.trim().replaceFirst(RegExp(r'/+$'), '');
  final uri = Uri.tryParse(normalized);
  if (uri == null ||
      !uri.hasScheme ||
      (uri.scheme != 'http' && uri.scheme != 'https') ||
      uri.host.isEmpty ||
      uri.hasQuery ||
      uri.hasFragment ||
      !uri.path.endsWith(knowledgebaseAppApiPrefix)) {
    throw ArgumentError.value(
      value,
      'appApiBaseUrl',
      'must be an absolute HTTP(S) URL ending with ' + knowledgebaseAppApiPrefix,
    );
  }
  final prefix = uri.path.substring(
    0,
    uri.path.length - knowledgebaseAppApiPrefix.length,
  );
  if (prefix.endsWith(knowledgebaseAppApiPrefix)) {
    throw ArgumentError.value(
      value,
      'appApiBaseUrl',
      'must contain ' + knowledgebaseAppApiPrefix + ' exactly once',
    );
  }
  return normalized;
}

String resolveKnowledgebaseTransportBaseUrl(String appApiBaseUrl) {
  final normalized = normalizeKnowledgebaseAppApiBaseUrl(appApiBaseUrl);
  final uri = Uri.parse(normalized);
  final transportPath = uri.path.substring(
    0,
    uri.path.length - knowledgebaseAppApiPrefix.length,
  );
  return uri
      .replace(path: transportPath)
      .toString()
      .replaceFirst(RegExp(r'/+$'), '');
}
