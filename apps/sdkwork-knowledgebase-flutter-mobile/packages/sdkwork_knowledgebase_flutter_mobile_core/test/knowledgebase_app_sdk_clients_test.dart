import 'package:flutter_test/flutter_test.dart';
import 'package:sdkwork_knowledgebase_flutter_mobile_core/sdkwork_knowledgebase_flutter_mobile_core.dart';

void main() {
  group('normalizeKnowledgebaseAppApiBaseUrl', () {
    test('accepts an absolute URL that ends with the app-api prefix', () {
      expect(
        normalizeKnowledgebaseAppApiBaseUrl('http://127.0.0.1:8095/app/v3/api'),
        'http://127.0.0.1:8095/app/v3/api',
      );
    });

    test('rejects a URL without the app-api prefix', () {
      expect(
        () => normalizeKnowledgebaseAppApiBaseUrl('http://127.0.0.1:8095'),
        throwsArgumentError,
      );
    });

    test('rejects a duplicated app-api prefix', () {
      expect(
        () => normalizeKnowledgebaseAppApiBaseUrl(
          'http://127.0.0.1:8095/app/v3/api/app/v3/api',
        ),
        throwsArgumentError,
      );
    });
  });

  group('resolveKnowledgebaseTransportBaseUrl', () {
    test('strips the app-api prefix for transport construction', () {
      expect(
        resolveKnowledgebaseTransportBaseUrl('http://127.0.0.1:8095/app/v3/api'),
        'http://127.0.0.1:8095',
      );
    });
  });

  group('core composition inventory', () {
    test('declares the three aligned app-api SDK dependencies', () {
      expect(listSdkworkCoreSdkInventory(), <String>[
        'sdkwork-drive-app-sdk',
        'sdkwork-iam-app-sdk',
        'sdkwork-knowledgebase-app-sdk',
      ]);
    });

    test('declares the knowledge capability as its only module', () {
      final modules = listKnowledgebaseModules();
      expect(modules.length, 1);
      expect(modules.first.routeIdPrefix, 'app.intelligence.knowledgebase.');
    });
  });
}
