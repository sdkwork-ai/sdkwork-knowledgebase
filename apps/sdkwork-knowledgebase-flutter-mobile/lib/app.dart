import 'package:flutter/material.dart';

import 'auth_gate.dart';
import 'bootstrap/runtime.dart';

class KnowledgebaseApp extends StatelessWidget {
  const KnowledgebaseApp({required this.runtime, super.key});

  final KnowledgebaseMobileRuntime runtime;

  @override
  Widget build(BuildContext context) {
    return KnowledgebaseRuntimeScope(
      runtime: runtime,
      child: MaterialApp(
        title: 'SDKWork Knowledgebase',
        theme: ThemeData(colorSchemeSeed: const Color(0xFF2563EB)),
        home: const AuthGate(),
      ),
    );
  }
}

class KnowledgebaseRuntimeScope extends InheritedWidget {
  const KnowledgebaseRuntimeScope({
    required this.runtime,
    required super.child,
    super.key,
  });

  final KnowledgebaseMobileRuntime runtime;

  static KnowledgebaseMobileRuntime of(BuildContext context) {
    final scope = context
        .dependOnInheritedWidgetOfExactType<KnowledgebaseRuntimeScope>();
    if (scope == null) {
      throw StateError('KnowledgebaseRuntimeScope is not available.');
    }
    return scope.runtime;
  }

  @override
  bool updateShouldNotify(KnowledgebaseRuntimeScope oldWidget) {
    return !identical(runtime, oldWidget.runtime);
  }
}
