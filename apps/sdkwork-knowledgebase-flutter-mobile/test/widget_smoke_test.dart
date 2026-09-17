import 'package:flutter_test/flutter_test.dart';
import 'package:sdkwork_knowledgebase_flutter_mobile_core/sdkwork_knowledgebase_flutter_mobile_core.dart';

void main() {
  test('app API base URL normalization rejects a base URL without the app-api suffix', () {
    expect(
      () => normalizeKnowledgebaseAppApiBaseUrl('http://127.0.0.1:8095'),
      throwsArgumentError,
    );
  });

  test('app API base URL normalization strips trailing slashes', () {
    expect(
      normalizeKnowledgebaseAppApiBaseUrl('http://127.0.0.1:8095/app/v3/api/'),
      'http://127.0.0.1:8095/app/v3/api',
    );
  });
}
