/// Appbase IAM runtime wiring for the Flutter mobile root.
///
/// Authority: `APP_SDK_INTEGRATION_SPEC.md` and
/// `IAM_LOGIN_INTEGRATION_SPEC.md`. Logout, refresh failure, account switch,
/// and tenant switch must clear the token manager, session stores, secure
/// platform storage, and realtime bridges together.
class KnowledgebaseFlutterIamRuntime {
  bool _authenticated = false;

  bool isAuthenticated() => _authenticated;

  void markAuthenticated() {
    _authenticated = true;
  }

  void clearOnLogout() {
    _authenticated = false;
  }
}

KnowledgebaseFlutterIamRuntime? _iamRuntime;

KnowledgebaseFlutterIamRuntime createIamRuntime() {
  _iamRuntime = KnowledgebaseFlutterIamRuntime();
  return _iamRuntime!;
}

KnowledgebaseFlutterIamRuntime getIamRuntime() {
  return _iamRuntime ?? createIamRuntime();
}
