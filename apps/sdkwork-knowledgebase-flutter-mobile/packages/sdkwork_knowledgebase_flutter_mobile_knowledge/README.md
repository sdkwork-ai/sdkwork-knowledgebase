# sdkwork_knowledgebase_flutter_mobile_knowledge

Knowledge capability for the Flutter mobile root: knowledgebase list, detail,
search, settings, and group-space launch screens, plus their controllers,
services, models, locale fragments, and route contributions.

SDK access is injected: this package never constructs an SDK client. The root
bootstrap creates `KnowledgebaseAppSdkClients` and hands the typed port to the
services.

Authority: `FLUTTER_APP_MOBILE_ARCHITECTURE_SPEC.md` sections 3 and 4.
