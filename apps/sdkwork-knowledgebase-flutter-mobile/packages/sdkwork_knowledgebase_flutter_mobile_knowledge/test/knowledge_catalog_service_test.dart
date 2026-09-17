import 'package:flutter_test/flutter_test.dart';
import 'package:sdkwork_knowledgebase_flutter_mobile_knowledge/sdkwork_knowledgebase_flutter_mobile_knowledge.dart';

void main() {
  group('extractKnowledgebaseItems', () {
    test('maps records from the flat envelope', () {
      final items = extractKnowledgebaseItems(<String, Object?>{
        'items': <Object?>[
          <String, Object?>{
            'knowledgebaseId': 'kb-1',
            'name': 'Engineering',
            'description': 'Engineering handbook',
            'documentCount': 12,
          },
        ],
      });
      expect(items.length, 1);
      expect(items.first.id, 'kb-1');
      expect(items.first.documentCount, 12);
    });

    test('maps records from the nested data envelope', () {
      final items = extractKnowledgebaseItems(<String, Object?>{
        'data': <String, Object?>{
          'items': <Object?>[
            <String, Object?>{'id': 'kb-2', 'code': 'Ops'},
          ],
        },
      });
      expect(items.length, 1);
      expect(items.first.id, 'kb-2');
    });

    test('drops records without an identity', () {
      expect(
        extractKnowledgebaseItems(<String, Object?>{
          'items': <Object?>[
            <String, Object?>{'name': 'anonymous'},
          ],
        }),
        isEmpty,
      );
    });
  });

  group('resolveKnowledgebaseHasMore', () {
    test('honours an explicit hasMore flag', () {
      expect(
        resolveKnowledgebaseHasMore(<String, Object?>{
          'pageInfo': <String, Object?>{'hasMore': true},
        }),
        isTrue,
      );
    });

    test('derives from page and totalPages', () {
      expect(
        resolveKnowledgebaseHasMore(<String, Object?>{
          'pageInfo': <String, Object?>{'page': 1, 'totalPages': 3},
        }),
        isTrue,
      );
      expect(
        resolveKnowledgebaseHasMore(<String, Object?>{
          'pageInfo': <String, Object?>{'page': 3, 'totalPages': 3},
        }),
        isFalse,
      );
    });
  });

  group('GroupKnowledgebaseLaunchService', () {
    const service = GroupKnowledgebaseLaunchService();

    test('accepts an opaque URL-safe ticket', () {
      expect(service.isLaunchTicketShapeValid('AbC-123_xyz'), isTrue);
    });

    test('rejects a ticket carrying query syntax or separators', () {
      expect(service.isLaunchTicketShapeValid('a/b?c=d'), isFalse);
      expect(service.isLaunchTicketShapeValid('   '), isFalse);
    });

    test('builds a request from a valid ticket', () {
      expect(service.buildRequest('AbC-123_xyz').ticket, 'AbC-123_xyz');
    });
  });

  group('knowledgebaseRouteContributions', () {
    test('declares the five aligned route identities', () {
      final ids = knowledgebaseRouteContributions.map((r) => r.id).toList();
      expect(ids, <String>[
        'app.intelligence.knowledgebase.list',
        'app.intelligence.knowledgebase.detail',
        'app.intelligence.knowledgebase.search',
        'app.intelligence.knowledgebase.settings',
        'app.intelligence.knowledgebase.launch',
      ]);
    });
  });
}
