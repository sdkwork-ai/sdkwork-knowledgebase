import 'package:flutter/material.dart';

import 'app.dart';
import 'bootstrap/runtime.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  final runtime = await bootstrap();
  runApp(KnowledgebaseApp(runtime: runtime));
}
