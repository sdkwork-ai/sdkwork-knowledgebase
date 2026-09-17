import '../models/knowledge_models.dart';

/// Knowledgebase search orchestration.
///
/// Authority: `PAGINATION_SPEC.md`. Search hits are mapped structurally until
/// the generated Dart app SDK target lands.
class KnowledgebaseSearchService {
  const KnowledgebaseSearchService();

  List<KnowledgebaseSearchHit> mapHits(Object? response) {
    final hits = <KnowledgebaseSearchHit>[];
    for (final record in _records(response)) {
      if (record is! Map) {
        continue;
      }
      final documentId = record['documentId'] ?? record['document_id'] ?? record['id'];
      if (documentId is! String || documentId.isEmpty) {
        continue;
      }
      final knowledgebaseId =
          record['knowledgebaseId'] ?? record['knowledgebase_id'] ?? '';
      final title = record['title'] ?? 'Document';
      final snippet = record['snippet'] ?? record['content'] ?? '';
      final rawScore = record['score'] ?? 0;
      hits.add(
        KnowledgebaseSearchHit(
          documentId: documentId,
          knowledgebaseId: knowledgebaseId is String ? knowledgebaseId : '',
          title: title is String ? title : title.toString(),
          snippet: snippet is String ? snippet : '',
          score: rawScore is num ? rawScore.toDouble() : double.tryParse(rawScore.toString()) ?? 0,
        ),
      );
    }
    return hits;
  }

  List<Object?> _records(Object? response) {
    if (response is Map) {
      final items = response['items'];
      if (items is List) {
        return items;
      }
      final data = response['data'];
      if (data is Map && data['items'] is List) {
        return data['items'] as List;
      }
    }
    return const <Object?>[];
  }
}
