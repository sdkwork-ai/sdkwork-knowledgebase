import 'package:flutter/material.dart';

/// Root authentication gate.
///
/// Route guards are shell/runtime responsibilities; capability packages only
/// declare the auth mode and permission hints for their routes.
class AuthGate extends StatelessWidget {
  const AuthGate({super.key});

  @override
  Widget build(BuildContext context) {
    return const Scaffold(
      body: Center(
        child: Text('SDKWork Knowledgebase Flutter scaffold'),
      ),
    );
  }
}
