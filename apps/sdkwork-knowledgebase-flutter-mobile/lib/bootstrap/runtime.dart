import 'host_adapters.dart';
import 'iam_runtime.dart';
import 'routes.dart';
import 'sdk_clients.dart';

/// Root composition result.
///
/// The root assembles environment resolution, SDK clients, IAM runtime, host
/// adapters, and the route registry only. It owns no product business logic.
class KnowledgebaseMobileRuntime {
  const KnowledgebaseMobileRuntime({required this.sdkClients});

  final SdkClients sdkClients;
}

Future<KnowledgebaseMobileRuntime> bootstrap() async {
  createIamRuntime();
  registerHostAdapters();
  final sdkClients = createSdkClients();
  createRoutes();
  return KnowledgebaseMobileRuntime(sdkClients: sdkClients);
}
