import 'package:sdkwork_knowledgebase_flutter_mobile_core/sdkwork_knowledgebase_flutter_mobile_core.dart';

import '../models/knowledge_models.dart';

/// Knowledgebase catalog orchestration.
///
/// The SDK port is injected by root bootstrap; this service maps records and
/// interprets pagination only and never constructs transport
/// (`APP_SDK_INTEGRATION_SPEC.md`, `PAGINATION_SPEC.md`).
const int defaultKnowledgebasePageSize = 20;

class KnowledgebaseCatalogService {
  const KnowledgebaseCatalogService({required this.client});

  final KnowledgebaseAppSdkPort client;

  Future<KnowledgebasePage> loadPage({
    required int page,
    int pageSize = defaultKnowledgebasePageSize,
  }) async {
    if (page < 1) {
      throw ArgumentError.value(page, 'page', 'must be a positive integer');
    }
    if (pageSize < 1) {
      throw ArgumentError.value(pageSize, 'pageSize', 'must be a positive integer');
    }
    // The Dart-adapted transport is not generated yet; the port is injected so
    // this service compiles unchanged once the adapter lands.
    return const KnowledgebasePage(
      items: <KnowledgebaseListItem>[],
      page: 1,
      hasMore: false,
    );
  }

  String baseUrl() => client.baseUrl;
}

/// Structural mapping because the transport DTO is generator-owned.
List<KnowledgebaseListItem> extractKnowledgebaseItems(Object? response) {
  final items = <KnowledgebaseListItem>[];
  for (final record in extractKnowledgebaseRecords(response)) {
    final mapped = mapKnowledgebaseRecord(record);
    if (mapped != null) {
      items.add(mapped);
    }
  }
  return items;
}

List<Object?> extractKnowledgebaseRecords(Object? response) {
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

KnowledgebaseListItem? mapKnowledgebaseRecord(Object? record) {
  if (record is! Map) {
    return null;
  }
  final id = record['knowledgebaseId'] ?? record['id'] ?? record['code'];
  if (id is! String || id.isEmpty) {
    return null;
  }
  final name = record['name'] ?? record['displayName'] ?? record['code'] ?? 'Knowledgebase';
  final description = record['description'];
  final documentCount = record['documentCount'] ?? record['document_count'] ?? 0;
  return KnowledgebaseListItem(
    id: id,
    name: name is String ? name : name.toString(),
    description: description is String ? description : '',
    documentCount: documentCount is int
        ? documentCount
        : int.tryParse(documentCount.toString()) ?? 0,
  );
}

bool resolveKnowledgebaseHasMore(Object? response) {
  if (response is! Map) {
    return false;
  }
  Map<Object?, Object?> pageInfo = const <Object?, Object?>{};
  final direct = response['pageInfo'];
  if (direct is Map) {
    pageInfo = direct;
  } else {
    final data = response['data'];
    if (data is Map && data['pageInfo'] is Map) {
      pageInfo = data['pageInfo'] as Map;
    }
  }
  if (pageInfo['hasMore'] == true) {
    return true;
  }
  final page = int.tryParse((pageInfo['page'] ?? 1).toString()) ?? 1;
  final totalPages =
      int.tryParse((pageInfo['totalPages'] ?? pageInfo['total_pages'] ?? 0).toString()) ?? 0;
  return totalPages > 0 && page < totalPages;
}
