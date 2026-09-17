import 'package:sdkwork_knowledgebase_flutter_mobile_core/sdkwork_knowledgebase_flutter_mobile_core.dart';

typedef SdkClients = KnowledgebaseAppSdkClients;

const String _configuredAppApiBaseUrl = String.fromEnvironment(
  'SDKWORK_KNOWLEDGEBASE_APP_API_BASE_URL',
  defaultValue: 'http://127.0.0.1:8095/app/v3/api',
);

const String sdkworkKnowledgebaseEnvironment = String.fromEnvironment(
  'SDKWORK_ENVIRONMENT',
  defaultValue: 'development',
);

const String sdkworkKnowledgebaseDeploymentProfile = String.fromEnvironment(
  'SDKWORK_DEPLOYMENT_PROFILE',
  defaultValue: 'standalone',
);

const String sdkworkKnowledgebaseProfileId = String.fromEnvironment(
  'SDKWORK_PROFILE_ID',
  defaultValue: 'standalone.development',
);

const String sdkworkKnowledgebaseRuntimeTarget = String.fromEnvironment(
  'SDKWORK_RUNTIME_TARGET',
  defaultValue: 'flutter-mobile',
);

SdkClients createSdkClients({
  String? appApiBaseUrl,
  String? authToken,
  String? accessToken,
}) {
  return createKnowledgebaseAppSdkClients(
    appApiBaseUrl: appApiBaseUrl ?? _configuredAppApiBaseUrl,
    authToken: authToken,
    accessToken: accessToken,
  );
}
